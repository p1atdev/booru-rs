mod hf;

use anyhow::{Context, Result};
use booru::board::danbooru::{response, Endpoint, Query};
use booru::board::BoardResponse;
use booru::client::{Auth, Client};
use clap::Parser;
use futures::stream::StreamExt;
use futures::TryStreamExt;
use hf_hub::api::sync::Api;
use indicatif::{MultiProgress, ParallelProgressIterator, ProgressBar, ProgressStyle};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::Field;
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use reqwest::{Method, Url};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use tokio::io::AsyncWriteExt;

use hf::from_hub;

const PBAR_TEMPLATE: &str =
    "[{elapsed_precise}] {bar:50.cyan/blue} {pos:>7}/{len:7} {msg} {eta_precise}";

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, env = "DANBOORU_USERNAME", hide_env_values = true)]
    pub username: String,
    #[arg(long, env = "DANBOORU_API_KEY", hide_env_values = true)]
    pub api_key: String,

    #[arg(long, default_value = "isek-ai/danbooru-tags-2024")]
    pub tags_ds: String,

    #[arg(long, default_value = "./output/tag-wiki.jsonl")]
    pub output: PathBuf,

    #[arg(long, default_value = "2")]
    pub num_connections: usize,
}

fn compose_url(client: &Client, title: &str) -> Result<Url> {
    Ok(client.compose(Endpoint::WikiPages(title.to_string()), Query::new())?)
}

fn with_underscore(tag: &str) -> String {
    tag.replace(" ", "_")
}

async fn fetch_wiki_page(client: &Client, title: &str) -> Result<response::WikiPage> {
    let url = compose_url(client, &with_underscore(title))?;
    let res = client.fetch_raw(url, Method::GET).await?;

    res.error_for_status_ref()?;

    let text = res.text().await?;
    let wiki = response::WikiPage::from_str(&text)?;

    Ok(wiki)
}

fn load_tags_ds(repo_name: &str) -> Result<Vec<SerializedFileReader<File>>> {
    let api = Api::new()?;
    let ds = from_hub(&api, repo_name.to_string())?;
    Ok(ds)
}

fn split_tags(tag_text: &str) -> Vec<String> {
    tag_text
        .split_terminator(", ")
        .map(|tag| tag.to_string())
        .collect()
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let auth = Auth::new(&args.username, &args.api_key);

    let ds = load_tags_ds(&args.tags_ds)?;

    // println!("ds: {:?}", ds.len());

    let pbar = ProgressBar::new(ds.len() as u64)
        .with_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE)?);
    let multi = MultiProgress::new();

    // collect tags
    let copyright_tags = RwLock::new(HashSet::<String>::new());
    let character_tags = RwLock::new(HashSet::<String>::new());
    let artist_tags = RwLock::new(HashSet::<String>::new());
    let general_tags = RwLock::new(HashSet::<String>::new());
    let meta_tags = RwLock::new(HashSet::<String>::new());

    // 1. collect unique tags
    println!("collecting tags...");
    let _ = ds
        .into_par_iter()
        .progress_with(pbar)
        .map(|file| {
            let schema = file.metadata().file_metadata().schema();
            let pbar = multi.add(
                ProgressBar::new(file.metadata().file_metadata().num_rows() as u64)
                    .with_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE)?),
            );

            let _ = file
                .get_row_iter(Some(schema.clone()))?
                .into_iter()
                .par_bridge()
                .progress_with(pbar.clone())
                .map(|row_iter| {
                    if let std::result::Result::Ok(row) = row_iter {
                        // println!("{:?}", row);
                        let _ = row
                            .get_column_iter()
                            .into_iter()
                            .par_bridge()
                            .map(|(column_name, value)| match value {
                                Field::Str(value) => {
                                    let target_tags = match column_name.as_str() {
                                        "copyright" => &copyright_tags,
                                        "character" => &character_tags,
                                        "artist" => &artist_tags,
                                        "general" => &general_tags,
                                        "meta" => &meta_tags,
                                        _ => return anyhow::Result::<()>::Ok(()), // do nothing
                                    };

                                    target_tags
                                        .write()
                                        .unwrap()
                                        .extend(split_tags(value.as_str()));

                                    anyhow::Result::<()>::Ok(())
                                }
                                _ => anyhow::Result::<()>::Ok(()), //  do nothing
                            })
                            .collect::<Result<Vec<_>>>()?;
                    }

                    anyhow::Result::<()>::Ok(())
                })
                .collect::<Result<Vec<_>>>()?;

            pbar.finish_with_message("done");

            anyhow::Result::<()>::Ok(())
        })
        .collect::<Result<Vec<_>>>()?;

    // show each tag counts
    let copyright_tags = copyright_tags.into_inner()?;
    let character_tags = character_tags.into_inner()?;
    let artist_tags = artist_tags.into_inner()?;
    let general_tags = general_tags.into_inner()?;
    let meta_tags = meta_tags.into_inner()?;

    println!("copyright: {:?} tags", copyright_tags.len());
    println!("character: {:?} tags", character_tags.len());
    println!("artist: {:?} tags", artist_tags.len());
    println!("general: {:?} tags", general_tags.len());
    println!("meta: {:?} tags", meta_tags.len());

    // 2. concat tags
    let mut all_tags = copyright_tags
        .iter()
        .chain(character_tags.iter())
        .chain(artist_tags.iter())
        .chain(general_tags.iter())
        .chain(meta_tags.iter())
        .collect::<Vec<_>>();

    // create output directory
    {
        let parent_dir = args.output.parent().context("output file path")?;
        std::fs::create_dir_all(parent_dir)?;
        anyhow::Result::<_>::Ok(())
    }?;
    // filter already fetched tags
    {
        if let Ok(input_file) = std::fs::OpenOptions::new()
            .read(true)
            .create(false)
            .open(&args.output)
        {
            let reader = std::io::BufReader::new(input_file);
            let lines = reader.lines().collect::<Result<Vec<_>, _>>()?;
            let tags = lines
                .into_iter()
                .map(|line| {
                    let wiki: response::WikiPage = serde_json::from_str(&line)?;
                    anyhow::Result::<_>::Ok(wiki.title)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let tags = tags.into_iter().collect::<Vec<_>>();
            all_tags = all_tags
                .into_iter()
                .filter(|tag| !tags.contains(*tag))
                .collect::<Vec<_>>();
            println!("tags to fetch: {:?}", all_tags.len());
        } else {
            println!("output file not found, fetching all tags...");
        }
    }

    // 3. fetch each tag wiki
    println!("fetching tag wiki...");
    let num_connections = args.num_connections;
    let pbar = ProgressBar::new(all_tags.len() as u64)
        .with_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE)?);
    let output_file = Arc::new(tokio::sync::Mutex::new(
        tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .truncate(false)
            .open(&args.output)
            .await?,
    ));
    let client = Arc::new(Client::new(booru::board::Board::Safebooru, auth)?);

    let _ = pbar
        .wrap_stream(futures::stream::iter(all_tags))
        .map(|tag| async {
            let client = client.clone();

            let wiki = fetch_wiki_page(&client, &tag.clone()).await?;
            anyhow::Result::<_>::Ok(wiki)
        })
        .buffer_unordered(num_connections)
        .map(|wiki| {
            let file = output_file.clone();
            async move {
                match wiki {
                    Result::Ok(wiki) => {
                        let mut file = file.lock().await;
                        let wiki_str = serde_json::to_string(&wiki)?;
                        file.write_all(wiki_str.as_bytes()).await?;
                        file.write_all(b"\n").await?;
                    }
                    Result::Err(e) => {
                        eprintln!("error: {:?}", e);
                    }
                }
                anyhow::Result::<_>::Ok(())
            }
        })
        .buffer_unordered(num_connections)
        .try_collect::<Vec<_>>()
        .await?;

    pbar.finish_with_message("done");

    anyhow::Result::<_>::Ok(())
}
