mod args;
mod cache;

use anyhow::Result;
use args::Cli;
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

    let output_dir = args.output.output_path;
    let threads = args.output.threads;
    let overwrite = args.output.overwrite;
    let num_posts = args.output.num_posts;

    // let cache_dir = &args.cache.cache_path;
    // let cache_lifetime = &args.cache.lifetime();

    tokio::fs::create_dir_all(&output_dir).await?;

    let query = build_query(&tags, score_min, score_max);

    let bar = ProgressBar::new(num_posts as u64);
    bar.set_style(ProgressStyle::with_template(PBAR_TEMPLATE)?);
    bar.set_message(format!("{}, page: 1", &tags));
    let shared_bar = Arc::new(tokio::sync::Mutex::new(bar));

    let mut page = 1;
    loop {
        let mut query = query.clone();
        query.page(page);

        let url = compose_url(&client, query)?;
        let posts = client.fetch::<response::Posts>(url, Method::GET).await?;

        println!("{} posts got", posts.len());

        if posts.is_empty() {
            // no more posts
            break;
        }

        let rest_posts = num_posts - shared_bar.clone().lock().await.position() as u32;
        let required_posts = &posts
            .into_iter()
            .filter(|post| post.file_url.is_some())
            .take(rest_posts as usize)
            .collect::<Vec<_>>();

        let shared_client = Arc::new(client.clone());

        let mut tasks = vec![];

        for post in required_posts.clone() {
            if post.file_url.is_none() {
                continue;
            }

            let file_url = post.file_url.unwrap();
            let cloned_bar = Arc::clone(&shared_bar);
            let cloned_client = Arc::clone(&shared_client);

            // TODO: customizable extension
            let image_path = get_image_path(&output_dir, &post.id, "webp")?;

            // let tag_path = get_tag_path(&output_dir, &post.id);

            let task = tokio::spawn(async move {
                let bar = cloned_bar.lock().await;

                println!("downloading: {}", file_url);

                // donwload the image
                let res = cloned_client
                    .fetch_raw(Url::parse(&file_url).unwrap(), Method::GET)
                    .await
                    .unwrap();

                println!("downloaded: {}", file_url);

                let bytes = res.bytes().await.unwrap();
                let img = image::load_from_memory(bytes.as_ref()).unwrap();
                // convert to compressed webp
                let encoder = Encoder::from_image(&img).unwrap();
                let webp = encoder.encode_lossless().deref().to_vec();

                println!("converted to webp: {}", image_path);

                let mut image_file = tokio::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(image_path)
                    .await
                    .expect("Failed to open file");

                // write
                image_file.write_all(&*webp).await.unwrap();

                // TODO: write tags

                bar.inc(1);
            });
            tasks.push(task);
        }

        stream::iter(tasks)
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
