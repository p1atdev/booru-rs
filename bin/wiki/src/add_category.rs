use anyhow::{Context, Result};
use booru::board::danbooru::response::WikiPage;
use clap::Parser;
use hf_hub::api::sync::Api;
use indicatif::{MultiProgress, ParallelProgressIterator, ProgressBar, ProgressStyle};
use parquet::file::reader::{FileReader, SerializedFileReader};
use parquet::record::Field;
use rayon::iter::{IntoParallelIterator, ParallelBridge, ParallelIterator};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufWriter, Write};
use std::path::PathBuf;
use std::sync::RwLock;

use hf::from_hub;

const PBAR_TEMPLATE: &str =
    "[{elapsed_precise}] {bar:50.cyan/blue} {pos:>7}/{len:7} {msg} {eta_precise}";

#[derive(Debug, Parser)]
struct Args {
    #[arg(long, default_value = "isek-ai/danbooru-tags-2024")]
    pub tags_ds: String,

    #[arg(short, long, default_value = "./output/tag-wiki-dedup.jsonl")]
    pub input: PathBuf,

    #[arg(short, long, default_value = "./output/tag-wiki-dedup-category.jsonl")]
    pub output: PathBuf,
}

fn with_underscore(tag: &str) -> String {
    tag.replace(" ", "_")
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

#[derive(Debug, Clone, PartialEq, Serialize)]
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
    let tag2category = copyright_tags
        .into_iter()
        .map(|tag| (tag, "copyright".to_string()))
        .chain(
            character_tags
                .into_iter()
                .map(|tag| (tag, "character".to_string())),
        )
        .chain(
            artist_tags
                .into_iter()
                .map(|tag| (tag, "artist".to_string())),
        )
        .chain(
            general_tags
                .into_iter()
                .map(|tag| (tag, "general".to_string())),
        )
        .chain(meta_tags.into_iter().map(|tag| (tag, "meta".to_string())))
        .collect::<HashMap<String, String>>();
    let title2tag = tag2category
        .clone()
        .into_iter()
        .map(|(tag, _)| (with_underscore(&tag), tag))
        .collect::<HashMap<String, String>>();

    // create output directory
    {
        let parent_dir = args.output.parent().context("output file path")?;
        std::fs::create_dir_all(parent_dir)?;
        println!("output directory created");
        anyhow::Result::<_>::Ok(())
    }?;

    // 3. fetch each tag wiki
    println!("adding categories...");
    // let batch_size = num_cpus::get();
    let input_file = std::fs::File::open(&args.input)?;

    let pbar = ProgressBar::new(input_file.metadata()?.len())
        .with_style(ProgressStyle::default_bar().template(PBAR_TEMPLATE)?);
    let output_file = std::sync::Mutex::new(BufWriter::new(
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&args.output)?,
    ));

    // 4 MB
    let read_buf = std::io::BufReader::with_capacity(4 * 1024 * 1024, input_file);

    let _ = read_buf
        .lines()
        .par_bridge()
        .progress_with(pbar.clone())
        .map(|line| {
            let line = line?;
            let wiki: WikiPage = serde_json::from_str(&line)?;

            if wiki.is_deleted {
                println!("wiki {} is deleted", wiki.title);
                return anyhow::Result::<_>::Ok(());
            }

            let title = wiki.title.clone();
            let tag = title2tag.get(&title);
            if tag.is_none() {
                eprintln!("title {} not found in title2tag. wiki: {:?}", title, wiki);
                return anyhow::Result::<_>::Ok(());
            }
            let tag = tag.unwrap();
            let category = tag2category
                .get(tag)
                .context(format!("tag {} not found tag2category", tag))?;
            let wiki = WikiPageWithCategory {
                category: category.clone(),
                tag: tag.clone(),
                id: wiki.id,
                created_at: wiki.created_at,
                updated_at: wiki.updated_at,
                title: wiki.title,
                other_names: wiki.other_names,
                body: wiki.body,
                is_locked: wiki.is_locked,
                is_deleted: wiki.is_deleted,
            };
            {
                let wiki_str = serde_json::to_string(&wiki)?;
                let mut file = output_file.lock().unwrap();
                file.write_all(wiki_str.as_bytes())?;
                file.write_all(b"\n")?;
            }
            anyhow::Result::<_>::Ok(())
        })
        .collect::<Result<Vec<_>>>()?;

    pbar.finish_with_message("done");

    anyhow::Result::<_>::Ok(())
}
