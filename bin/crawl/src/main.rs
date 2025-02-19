mod args;

use anyhow::Result;
use args::Cli;
use booru::board::danbooru::{response, search, Endpoint, FileExt, Query};
use booru::board::{danbooru, BoardQuery, BoardSearchTagsBuilder};
use booru::client::{Auth, Client};
use clap::Parser;
use futures::stream::{self, StreamExt};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::{Method, Url};
use std::path::Path;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};
use tokio::sync::Mutex;

const PBAR_TEMPLATE: &str =
    "{spinner:.green} [{elapsed_precise}] {bar:50.cyan/blue} {pos:>7}/{len:7} ({eta_precise}) {msg}";

fn build_query(id_from: u32, id_to: u32, tags: &str) -> Query {
    let mut builder = danbooru::SearchTagsBuilder::new();
    builder.add_tag(tags);

    builder.ids(vec![search::Id::InEx {
        min: id_from,
        max: id_to,
    }]);
    builder.order(search::Order::Id(search::OrderBy::Asc));

    // println!("query: {}", builder.build());

    let mut query = Query::posts(&builder.build());
    query.limit(200);

    query
}

fn compose_url(client: &Client, query: Query) -> Result<Url> {
    Ok(client.compose(Endpoint::Posts, query)?)
}

fn get_output_file_path<P: AsRef<Path>>(base_dir: P, name: &str) -> String {
    base_dir
        .as_ref()
        .join(format!("{name}.jsonl"))
        .to_string_lossy()
        .to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    let auth = Auth::new(&args.username, &args.api_key);
    let client = Client::new(args.domain.board(), auth)?;

    let output_dir = args.output.output_path;
    let output_name = args.output.prefix.unwrap_or(args.domain.to_string());
    let write_concurrency = args.output.write_concurrency;
    let overwrite = args.output.overwrite;
    let max_requests_per_second = args.output.max_requests_per_second;

    // if output dir does not exist, create it
    tokio::fs::create_dir_all(&output_dir).await?;
    let output_file_path = get_output_file_path(&output_dir, &output_name);

    let id_start = if std::fs::metadata(&output_file_path).is_ok() && !overwrite {
        // get the max id from the output file
        let continue_file = OpenOptions::new()
            .read(true)
            .open(&output_file_path)
            .await
            .expect("Failed to open file");

        let mut reader = tokio::io::BufReader::new(continue_file);
        let mut id_start = args.id.id_start;
        // read jsonl file line by line
        let mut current_line = String::new();
        while let Ok(bytes) = reader.read_line(&mut current_line).await {
            if bytes == 0 {
                break;
            }
            // read until the last line
            if !current_line.is_empty() {
                let post: response::Post = serde_json::from_str(&current_line.trim())?;
                id_start = (u32::try_from(post.id)?).max(id_start);
            }
            current_line.clear();
        }

        id_start
    } else {
        args.id.id_start
    };
    let id_end = args.id.id_end;

    println!("Fetching posts from {} to {}", id_start, id_end);

    // create output file reference
    let output_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(overwrite)
        .append(!overwrite)
        .open(output_file_path)
        .await
        .expect("Failed to open file");
    let shared_output_file = Arc::new(Mutex::new(output_file));
    let delay = std::time::Duration::from_secs_f64(1.0 / max_requests_per_second as f64);
    let bar = ProgressBar::new(u64::from(id_end - id_start));
    bar.set_style(ProgressStyle::with_template(PBAR_TEMPLATE)?);

    let mut id_head = id_start;
    let page_size = 200;
    while id_head < id_end {
        let id_tail = id_head + page_size;
        let query = build_query(id_head, id_tail, &args.tags);

        let mut tasks = vec![];

        // start crawling
        bar.set_message(format!("{id_head}~{id_tail}"));

        let mut query = query.clone();
        query.page(1);

        let url = compose_url(&client, query)?;
        let posts = client.fetch::<response::Posts>(url, Method::GET).await?;

        if posts.is_empty() {
            id_head += page_size;
            continue;
        }
        let last_post_id = (&posts.last().unwrap().id).clone() as u32;

        // write out
        let cloned_output_file = Arc::clone(&shared_output_file);

        let task = tokio::spawn(async move {
            let mut file = cloned_output_file.lock().await;
            for post in posts {
                // write post as inline json with newline
                let post_str = serde_json::to_string(&post).unwrap();
                file.write_all(post_str.as_bytes()).await.unwrap();
                file.write_all(b"\n").await.unwrap();
            }
        });
        tasks.push(task);

        // wait for all writing tasks to finish
        stream::iter(tasks)
            .buffer_unordered(write_concurrency)
            .collect::<Vec<_>>()
            .await;

        bar.set_position(last_post_id as u64);

        // delay for rate limiting
        tokio::time::sleep(delay).await;

        id_head = last_post_id + 1;
    }

    Ok(())
}
