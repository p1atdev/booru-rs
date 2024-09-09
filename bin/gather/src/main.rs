mod args;
mod utils;

use anyhow::{bail, Context, Result};
use args::{Cli, Optimization};
use booru::board::danbooru::{response, search, Endpoint, FileExt, Query};
use booru::board::{danbooru, BoardQuery, BoardSearchTagsBuilder};
use booru::client::{Auth, Client};
use clap::Parser;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Method, Url};
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use webp::Encoder;

const PBAR_TEMPLATE: &str = "[{elapsed_precise}] {bar:50.cyan/blue} {pos:>7}/{len:7} {msg}";

pub struct Env {
    pub username: String,
    pub api_key: String,
}

impl Env {
    pub fn new() -> Self {
        use dotenv::dotenv;
        use std::env;

        dotenv().ok();
        Env {
            username: env::var("DANBOORU_USERNAME").unwrap(),
            api_key: env::var("DANBOORU_API_KEY").unwrap(),
        }
    }
}

fn build_query(tags: &str, score_min: i32, score_max: Option<i32>) -> Query {
    let mut builder = danbooru::SearchTagsBuilder::new();
    builder.add_tag(tags);
    builder.add_tag("-is:banned");
    builder.filetypes(vec![FileExt::Png, FileExt::Jpg, FileExt::Webp]);

    if let Some(max) = score_max {
        builder.scores(vec![search::Score::MinMax {
            min: score_min,
            max: max,
        }]);
    } else {
        builder.scores(vec![search::Score::Min(score_min)]);
    }

    println!("query: {}", builder.build());

    let mut query = Query::posts(&builder.build());
    query.limit(200);

    query
}

fn compose_url(client: &Client, query: Query) -> Result<Url> {
    Ok(client.compose(Endpoint::Posts, query)?)
}

fn get_image_path<P: AsRef<Path>>(base_dir: P, id: &i64, extension: &str) -> Result<String> {
    let filename = format!("{}.{}", id, extension);
    let path = base_dir
        .as_ref()
        .join(filename)
        .to_string_lossy()
        .to_string();

    Ok(path)
}

fn get_tag_path<P: AsRef<Path>>(base_dir: P, id: &i64) -> String {
    base_dir
        .as_ref()
        .join(format!("{}.txt", &id))
        .to_string_lossy()
        .to_string()
}

fn get_image_file_ext(optim: &Optimization, url: String) -> Result<String> {
    match optim {
        Optimization::None => {
            let url = Url::parse(&url)?;
            let path = url.path();
            let file_ext = path
                .split('.')
                .last()
                .context("Failed to get file extension")?;
            Ok(file_ext.to_string())
        }
        Optimization::Webp => Ok("webp".to_string()),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    println!("{:?}", args);

    let env = Env::new();

    let auth = Auth::new(&env.username, &env.api_key);
    let client = Client::new(args.domain.board(), auth)?;

    let tags = args.tags;
    let score_min = args.condition.score_min;
    let score_max = args.condition.score_max;

    let output_dir = Arc::new(args.output.output_path);
    let connections = args.output.connections;
    let threads = args.output.threads;
    let overwrite = args.output.overwrite;
    let num_posts = args.output.num_posts;
    let optim = Arc::new(args.output.optim);
    let tag_template = Arc::new(args.output.tag_template);

    // let cache_dir = &args.cache.cache_path;
    // let cache_lifetime = &args.cache.lifetime();

    tokio::fs::create_dir_all(&output_dir.clone().as_ref()).await?;

    let query = build_query(&tags, score_min, score_max);

    let bar = ProgressBar::new(num_posts as u64);
    bar.set_style(ProgressStyle::with_template(PBAR_TEMPLATE)?);
    bar.set_message(format!("{}, page: 1", &tags));
    let shared_bar = Arc::new(tokio::sync::Mutex::new(bar));
    let tag_manager = Arc::new(utils::TagManager::new());

    let mut page = 1;
    loop {
        let mut query = query.clone();
        query.page(page);

        let url = compose_url(&client, query)?;
        let posts = client.fetch::<response::Posts>(url, Method::GET).await?;

        if posts.is_empty() {
            // no more posts
            break;
        }

        let rest_posts = num_posts - shared_bar.clone().lock().await.position() as u32;
        let required_posts = &posts
            .into_iter()
            .filter(|post| {
                if post.file_url.is_none() {
                    return false;
                }
                if overwrite {
                    // if overwrite is enabled, download all images
                    return true;
                }

                // don't overwrite existing files~~

                let ext =
                    get_image_file_ext(optim.as_ref(), post.clone().file_url.unwrap()).unwrap();
                let image_path = get_image_path(&output_dir.as_ref(), &post.id, &ext).unwrap();
                let tag_path = get_tag_path(&output_dir.as_ref(), &post.id);

                // if both image and tag files exist, skip
                if Path::new(&image_path).exists() && Path::new(&tag_path).exists() {
                    return false; // skip
                }

                return true;
            })
            .take(rest_posts as usize)
            .collect::<Vec<_>>();

        // firstly download images
        stream::iter(required_posts.clone())
            .map(|post| {
                let file_url = post.clone().file_url.unwrap();
                let cloned_client = client.clone();

                tokio::spawn(async move {
                    // donwload the image
                    let res = cloned_client
                        .fetch_raw(Url::parse(&file_url)?, Method::GET)
                        .await?;
                    let bytes = res.bytes().await?;
                    Result::<_>::Ok((bytes, post))
                })
            })
            .buffer_unordered(connections)
            .map(|pair| pair?)
            // then convert to webp
            .map(|pair| match optim.clone().as_ref() {
                Optimization::None => {
                    return tokio::task::spawn_blocking(move || {
                        let (bytes, post) = pair?;
                        let bytes = bytes.deref().to_vec();
                        Result::<_>::Ok((bytes, post))
                    });
                }
                Optimization::Webp => tokio::task::spawn_blocking(move || {
                    let (bytes, post) = pair?;
                    let img = image::load_from_memory(&bytes)?;

                    let encoder = match Encoder::from_image(&img) {
                        Ok(encoder) => encoder,
                        Err(e) => bail!("Failed to encode image: {}", e),
                    };

                    let bytes = encoder.encode_lossless().deref().to_vec();
                    Result::<_>::Ok((bytes, post))
                }),
            })
            .buffer_unordered(threads)
            .map(|pair| pair?)
            // finally write to disk
            .map(|pair| {
                let cloned_bar = shared_bar.clone();
                let cloned_output_dir = output_dir.clone();
                let cloned_optim = optim.clone();
                let cloned_tag_template = tag_template.clone();
                let cloned_tag_manager = tag_manager.clone();

                tokio::spawn(async move {
                    let (bytes, post) = pair?;

                    let file_ext = get_image_file_ext(
                        &cloned_optim.as_ref(),
                        post.clone().file_url.context("file_url must not be null")?,
                    )?;
                    let image_path =
                        get_image_path(&cloned_output_dir.as_ref(), &post.id, &file_ext)?;
                    let tag_path = get_tag_path(&cloned_output_dir.as_ref(), &post.id);

                    // write the image
                    let mut image_file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(image_path)
                        .await
                        .expect("Failed to open image file");
                    image_file.write_all(bytes.as_ref()).await?;

                    // write tags
                    let mut tag_file = tokio::fs::OpenOptions::new()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(tag_path)
                        .await
                        .expect("Failed to open tag text file");
                    let tag_text = cloned_tag_manager.format_template(&cloned_tag_template, &post);
                    tag_file.write_all(tag_text.as_bytes()).await?;

                    cloned_bar.lock().await.inc(1);

                    Result::<_>::Ok(())
                })
            })
            .buffer_unordered(threads)
            .collect::<Vec<_>>()
            .await;

        if shared_bar.clone().lock().await.position() as u32 >= num_posts {
            break;
        }

        page += 1;
        shared_bar
            .clone() // clone the Arc
            .lock()
            .await
            .set_message(format!("{}, page: {}", &tags, page));
    }
    shared_bar.lock_owned().await.finish();

    Ok(())
}
