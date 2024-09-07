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
use tokio::io::AsyncWriteExt;
use tokio::sync::Mutex;

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

fn build_query(year: &u16, month: &u8, tags: &str) -> Query {
    let next_month = (month % 12) + 1;
    let next_year = year + ((month.clone() as u16) / 12);

    let mut builder = danbooru::SearchTagsBuilder::new();
    builder.add_tag(tags);
    // builder.add_tag("-is:banned");
    // builder.filetypes(vec![FileExt::Png, FileExt::Jpg, FileExt::Webp]);

    builder.dates(vec![search::Date::InEx {
        min: format!("{}-{}-01", year, month),
        max: format!("{}-{}-01", next_year, next_month),
    }]);

    println!("query: {}", builder.build());

    let mut query = Query::posts(&builder.build());
    query.limit(200);

    query
}

fn compose_url(client: &Client, query: Query) -> Result<Url> {
    Ok(client.compose(Endpoint::Posts, query)?)
}

fn get_output_file_path<P: AsRef<Path>>(
    base_dir: P,
    prefix: &str,
    year: &u16,
    month: &u8,
) -> String {
    base_dir
        .as_ref()
        .join(format!("{}-{}-{:02}.jsonl", prefix, year, month))
        .to_string_lossy()
        .to_string()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    let env = Env::new();

    let auth = Auth::new(&env.username, &env.api_key);
    let client = Client::new(args.domain.board(), auth)?;

    let year_start = args.date.year_start;
    let month_start = args.date.month_start;
    let year_end = args.date.year_end.unwrap_or(year_start);
    let month_end = args.date.month_end.unwrap_or(month_start);

    let output_dir = args.output.output_path;
    let output_prefix = args.output.prefix.unwrap_or(args.domain.to_string());
    let write_concurrency = args.output.write_concurrency;
    let overwrite = args.output.overwrite;

    // if output dir does not exist, create it
    tokio::fs::create_dir_all(&output_dir).await?;

    for year in year_start..=year_end {
        for month in month_start..=month_end {
            let query = build_query(&year, &month, &args.tags);

            let output_file_path = get_output_file_path(&output_dir, &output_prefix, &year, &month);
            if tokio::fs::metadata(&output_file_path).await.is_ok() && !overwrite {
                eprintln!("File {} already exists, skipping", output_file_path);
                continue;
            }

            // create output file reference
            let output_file = OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(output_file_path)
                .await
                .expect("Failed to open file");
            let shared_output_file = Arc::new(Mutex::new(output_file));
            let mut tasks = vec![];

            // start crawling
            let bar = ProgressBar::new(1000);
            bar.set_style(ProgressStyle::with_template(PBAR_TEMPLATE)?);
            bar.set_message(format!("{}-{:02}", year, month));
            let mut page = 1;
            loop {
                let mut query = query.clone();
                query.page(page);

                let url = compose_url(&client, query)?;
                let posts = client.fetch::<response::Posts>(url, Method::GET).await?;

                if posts.is_empty() {
                    break;
                }

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

                page += 1;
                bar.inc(1);
            }
            bar.finish();

            // wait for all writing tasks to finish
            stream::iter(tasks)
                .buffer_unordered(write_concurrency)
                .collect::<Vec<_>>()
                .await;
        }
    }

    Ok(())
}
