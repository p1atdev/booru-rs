mod args;
mod utils;

use anyhow::{Context, Result};
use args::{Cli, FileExt as SaveFileExt};
use booru::board::danbooru::{response, search, Endpoint, FileExt, Query};
use booru::board::{danbooru, BoardQuery, BoardSearchTagsBuilder};
use booru::client::{Auth, Client};
use clap::Parser;
use futures::stream::{self, StreamExt};
use futures::TryStreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Method, Url};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

const PBAR_TEMPLATE: &str = "[{elapsed_precise}] {bar:50.cyan/blue} {pos:>7}/{len:7} {msg}";

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

fn get_image_file_ext(file_ext: Option<SaveFileExt>, url: String) -> Result<String> {
    match file_ext {
        None => {
            let url = Url::parse(&url)?;
            let path = url.path();
            let file_ext = path
                .split('.')
                .last()
                .context("Failed to get file extension")?;
            Ok(file_ext.to_string())
        }
        Some(ext) => Ok(ext.to_string()),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    // println!("{:?}", args);

    let auth = Auth::new(&args.username, &args.api_key);
    let client = Client::new(args.domain.board(), auth)?;

    let tags = args.tags;
    let score_min = args.condition.score_min;
    let score_max = args.condition.score_max;

    let output_dir = Arc::new(args.output.output_path);
    let connections = args.output.connections;
    let threads = args.output.threads;
    let overwrite = args.output.overwrite;
    let num_posts = args.output.num_posts;
    let file_ext = args.output.file_ext;
    let tag_template = Arc::new(args.output.tag_template);

    // let cache_dir = &args.cache.cache_path;
    // let cache_lifetime = &args.cache.lifetime();

    tokio::fs::create_dir_all(&output_dir.clone().as_ref()).await?;

    let query = build_query(&tags, score_min, score_max);

    let bar = ProgressBar::new(num_posts as u64);
    bar.set_style(ProgressStyle::with_template(PBAR_TEMPLATE)?);
    bar.set_message(format!("{}, page: 1", &tags));

    // let shared_bar = Arc::new(tokio::sync::Mutex::new(bar));
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

        let rest_posts = num_posts - bar.position() as u32;
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
                    get_image_file_ext(file_ext.clone(), post.clone().file_url.unwrap()).unwrap();
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
        let _ = bar
            .wrap_stream(stream::iter(required_posts.clone()))
            .map(|post| {
                let file_url = post.clone().file_url.unwrap();
                let cloned_client = client.clone();

                async move {
                    // donwload the image
                    let res = cloned_client
                        .fetch_raw(Url::parse(&file_url)?, Method::GET)
                        .await?;
                    let bytes = res.bytes().await?;
                    Result::<_>::Ok((bytes, post))
                }
            })
            .buffer_unordered(connections)
            // load the image
            .map_ok(|(bytes, post)| async move {
                let image = image::load_from_memory(&bytes)?;
                Result::<_>::Ok((image, post))
            })
            .try_buffer_unordered(threads)
            .map_ok(|(image, post)| {
                let cloned_output_dir = output_dir.clone();
                let cloned_file_ext = file_ext.clone();

                async move {
                    let file_ext = get_image_file_ext(
                        cloned_file_ext,
                        post.clone().file_url.context("file_url must not be null")?,
                    )?;
                    let image_path =
                        get_image_path(&cloned_output_dir.as_ref(), &post.id, &file_ext)?;

                    // write the image
                    image.save(image_path)?;

                    Result::<_>::Ok(post)
                }
            })
            .try_buffer_unordered(threads)
            .map_ok(|post| {
                let cloned_output_dir = output_dir.clone();
                let cloned_tag_template = tag_template.clone();
                let cloned_tag_manager = tag_manager.clone();

                async move {
                    let tag_path = get_tag_path(&cloned_output_dir.as_ref(), &post.id);

                    // write tags
                    let mut tag_file = File::options()
                        .write(true)
                        .create(true)
                        .truncate(true)
                        .open(tag_path)
                        .await
                        .expect("Failed to open tag text file");
                    let tag_text = cloned_tag_manager.format_template(&cloned_tag_template, &post);
                    tag_file.write_all(tag_text.as_bytes()).await?;
                    tag_file.flush().await?;

                    Result::<_>::Ok(())
                }
            })
            .try_buffer_unordered(threads)
            .try_collect::<Vec<_>>()
            .await?;

        if bar.position() as u32 >= num_posts {
            break;
        }

        page += 1;
        bar.set_message(format!("{}, page: {}", &tags, page));
    }
    bar.finish();

    Ok(())
}
