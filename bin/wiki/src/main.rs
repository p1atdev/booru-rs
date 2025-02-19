use anyhow::{bail, Context, Result};
use booru::board::danbooru::response::WikiPage;
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
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::BufRead;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;

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

    #[arg(short, long, default_value = "./output/tag-wiki.jsonl")]
    pub output: PathBuf,

    #[arg(long, default_value = "./output/not_founds.txt")]
    pub not_founds: PathBuf,

    #[arg(short, long, default_value_t = 2)]
    pub num_connections: usize,

    #[arg(short, long, default_value_t = 10)]
    pub limit_per_sec: usize,
}

#[derive(thiserror::Error, Debug)]
pub enum TagWikiError {
    #[error("the data for key `{0}` is not available")]
    NotFound(String),
    #[error("too many requests: {0}")]
    TooManyRequests(String),
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error("failed to decode text")]
    FailedToDecode,
    #[error("failed to parse json")]
    FailedToParseJSON(String),
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

fn compose_url(client: &Client, title: &str) -> Result<Url> {
    Ok(client.compose(Endpoint::WikiPages(title.to_string()), Query::new())?)
}

fn with_underscore(tag: &str) -> String {
    tag.replace(" ", "_")
}

async fn fetch_wiki_page(client: &Client, title: &str) -> Result<response::WikiPage, TagWikiError> {
    let title = with_underscore(title);
    let url = compose_url(client, &title).map_err(|e| TagWikiError::Unknown(e))?;
    let res = client
        .fetch_raw(url, Method::GET)
        .await
        .map_err(|e| TagWikiError::Unknown(e))?;

    match res.status() {
        reqwest::StatusCode::NOT_FOUND => {
            return Err(TagWikiError::NotFound(title.to_string()).into());
        }
        reqwest::StatusCode::TOO_MANY_REQUESTS => {
            return Err(TagWikiError::TooManyRequests(title.to_string()).into());
        }
        reqwest::StatusCode::BAD_REQUEST => {
            return Err(TagWikiError::BadRequest(title.to_string()).into());
        }
        _ => {}
    }

    let text = res.text().await.map_err(|_| TagWikiError::FailedToDecode)?;
    let wiki = response::WikiPage::from_str(&text)
        .map_err(|_| TagWikiError::FailedToParseJSON(title.clone()))?;

    Ok(wiki)
}

fn load_tags_ds(repo_name: &str) -> Result<Vec<SerializedFileReader<File>>> {
    let api = Api::new()?;
    let ds = from_hub(&api, repo_name.to_string(), Some("main".to_string()))?;
    Ok(ds)
}

fn split_tags(tag_text: &str) -> Vec<String> {
    tag_text
        .split_terminator(", ")
        .map(|tag| tag.to_string())
        .collect()
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct WikiPageWithCategory {
    id: i64,
    created_at: String,
    updated_at: String,
    title: String,
    other_names: Vec<String>,
    body: String,
    is_locked: bool,
    is_deleted: bool,

    category: String,
    tag: String,
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
                                    let target_tags_set = match column_name.as_str() {
                                        "copyright" | "tag_string_copyright" => &copyright_tags,
                                        "character" | "tag_string_character" => &character_tags,
                                        "artist" | "tag_string_artist" => &artist_tags,
                                        "general" | "tag_string_general" => &general_tags,
                                        "meta" | "tag_string_meta" => &meta_tags,
                                        _ => return anyhow::Result::<()>::Ok(()), // do nothing
                                    };
                                    let target_tags = match column_name.as_str() {
                                        "copyright" | "character" | "artist" | "general"
                                        | "meta" => split_tags(value.as_str())
                                            .iter()
                                            .map(|s| with_underscore(s))
                                            .collect::<Vec<_>>(),
                                        "tag_string_copyright"
                                        | "tag_string_character"
                                        | "tag_string_artist"
                                        | "tag_string_general"
                                        | "tag_string_meta" => value
                                            .split_terminator(" ")
                                            .map(|s| s.to_string())
                                            .collect::<Vec<_>>(),
                                        _ => return anyhow::Result::<()>::Ok(()), // do nothing
                                    };

                                    target_tags_set.write().unwrap().extend(target_tags);

                                    anyhow::Result::<()>::Ok(())
                                }
                                //     // list of tags
                                // Field::ListInternal(value) => {
                                //     let target_tags = match column_name.as_str() {
                                //         "copyright" => &copyright_tags,
                                //         "character" => &character_tags,
                                //         "artist" => &artist_tags,
                                //         "general" => &general_tags,
                                //         "meta" => &meta_tags,
                                //         _ => return anyhow::Result::<()>::Ok(()), // do nothing
                                //     };

                                //     assert!(value.elements().iter().all(|e| match e {
                                //         Field::Str(_) => true,
                                //         _ => false,
                                //     }));
                                //     target_tags.write().unwrap().extend(
                                //         value.elements().iter().map(|e| e.to_string()),
                                //     );

                                //     anyhow::Result::<()>::Ok(())
                                // }
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

    // tag to category map
    let mut tag_to_category = std::collections::HashMap::<String, String>::new();
    for tag in &copyright_tags {
        tag_to_category.insert(tag.clone(), "copyright".to_string());
    }
    for tag in &character_tags {
        tag_to_category.insert(tag.clone(), "character".to_string());
    }
    for tag in &artist_tags {
        tag_to_category.insert(tag.clone(), "artist".to_string());
    }
    for tag in &general_tags {
        tag_to_category.insert(tag.clone(), "general".to_string());
    }
    for tag in &meta_tags {
        tag_to_category.insert(tag.clone(), "meta".to_string());
    }

    // 2. concat tags
    let mut all_tags = tag_to_category
        .keys()
        .map(|tag| tag.clone())
        .collect::<Vec<_>>();

    // create output directory
    {
        let parent_dir = args.output.parent().context("output file path")?;
        std::fs::create_dir_all(parent_dir)?;
        println!("output directory created");
        anyhow::Result::<_>::Ok(())
    }?;
    // filter already fetched tags
    {
        if let Ok(input_file) = std::fs::OpenOptions::new()
            .read(true)
            .create(false)
            .open(&args.output)
        {
            println!("filtering already fetched tags...");
            let reader = std::io::BufReader::new(input_file);
            let lines = reader.lines().collect::<Result<Vec<_>, _>>()?;
            let tags = lines
                .into_iter()
                .par_bridge()
                .map(|line| {
                    let wiki: WikiPageWithCategory = serde_json::from_str(&line)?;
                    anyhow::Result::<_>::Ok(wiki.title)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let tags = tags.into_iter().collect::<Vec<_>>();
            all_tags = all_tags
                .into_iter()
                .par_bridge()
                .filter(|tag| !tags.contains(tag))
                .collect::<Vec<_>>();
            println!("tags to fetch: {:?}", all_tags.len());
        } else {
            println!("output file not found, fetching all tags...");
        }
    }
    {
        if let Ok(not_found_file) = std::fs::OpenOptions::new()
            .read(true)
            .create(false)
            .open(&args.not_founds)
        {
            println!("reading not found file...");
            let reader = std::io::BufReader::new(not_found_file);
            let lines = reader.lines().collect::<Result<Vec<_>, _>>()?;
            let tags = lines
                .into_iter()
                .par_bridge()
                .map(|line| line.trim().to_string())
                .collect::<Vec<_>>();
            let tags = tags.into_iter().collect::<Vec<_>>();
            all_tags = all_tags
                .into_iter()
                .par_bridge()
                .filter(|tag| !tags.contains(tag))
                .collect::<Vec<_>>();
            println!("tags to fetch: {:?}", all_tags.len());
        } else {
            println!("not found file not found, fetching all tags...");
        }
    }

    // 3. fetch each tag wiki
    println!("fetching tag wiki...");
    let num_connections = args.num_connections;
    let pbar = ProgressBar::new(all_tags.len() as u64)
        .with_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE)?);
    let output_file = Arc::new(tokio::sync::Mutex::new(tokio::io::BufWriter::new(
        tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .truncate(false)
            .open(&args.output)
            .await?,
    )));
    let not_founds = Arc::new(tokio::sync::Mutex::new(
        tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .truncate(false)
            .open(&args.not_founds)
            .await?,
    ));
    let client = Arc::new(Client::new(booru::board::Board::Safebooru, auth)?);
    let delay_time = Arc::new(std::time::Duration::from_secs_f64(
        1.0 / args.limit_per_sec as f64,
    ));
    let tag_to_category = Arc::new(tag_to_category);

    let _ = pbar
        .wrap_stream(futures::stream::iter(all_tags))
        .map(|tag| {
            let client = client.clone();
            async move {
                let wiki = fetch_wiki_page(&client, &tag.clone()).await?;
                Ok((tag, wiki))
            }
        })
        .buffer_unordered(num_connections)
        .map(|pair: Result<(String, WikiPage), TagWikiError>| {
            let file = output_file.clone();
            let not_founds = not_founds.clone();
            let delay_time = delay_time.clone();
            let tag_to_category = tag_to_category.clone();
            async move {
                match pair {
                    Result::Ok((tag, wiki)) => {
                        let mut file = file.lock().await;
                        let wiki_str = serde_json::to_string(&wiki)?;
                        let wiki: response::WikiPage = serde_json::from_str(&wiki_str)?;
                        let category = tag_to_category.get(&tag).unwrap();

                        let wiki: WikiPageWithCategory = WikiPageWithCategory {
                            id: wiki.id,
                            created_at: wiki.created_at,
                            updated_at: wiki.updated_at,
                            title: wiki.title,
                            other_names: wiki.other_names,
                            body: wiki.body,
                            is_locked: wiki.is_locked,
                            is_deleted: wiki.is_deleted,
                            category: category.clone(),
                            tag: tag.clone(),
                        };
                        file.write_all(serde_json::to_string(&wiki).unwrap().as_bytes())
                            .await?;
                        file.write_all(b"\n").await?;
                    }
                    Result::Err(e) => {
                        eprintln!("error: {:?}", e);
                        match e {
                            TagWikiError::NotFound(tag) => {
                                let mut file = not_founds.lock().await;
                                file.write_all(tag.as_bytes()).await?;
                                file.write_all(b"\n").await?;
                            }
                            TagWikiError::TooManyRequests(msg) => {
                                eprintln!("too many requests: {}", msg);
                                sleep(*delay_time).await;
                            }
                            TagWikiError::BadRequest(msg) => {
                                eprintln!("bad request: {}", msg);
                            }
                            TagWikiError::FailedToParseJSON(text) => {
                                bail!("failed to parse json: {}", text);
                            }
                            TagWikiError::FailedToDecode => {
                                bail!("failed to decode text");
                            }
                            TagWikiError::Unknown(e) => {
                                bail!(e);
                            }
                        }
                    }
                }
                sleep(*delay_time).await;
                anyhow::Result::<_>::Ok(())
            }
        })
        .buffer_unordered(num_connections)
        .try_collect::<Vec<_>>()
        .await?;

    pbar.finish_with_message("done");

    anyhow::Result::<_>::Ok(())
}
