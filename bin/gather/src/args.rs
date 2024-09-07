use booru::board::Board;
use clap::{Args, Parser, ValueEnum};
use std::time::Duration;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long, default_value = "danbooru")]
    pub domain: Domain,

    pub tags: String,

    #[command(flatten)]
    pub output: Output,

    #[command(flatten)]
    pub condition: Condition,

    #[command(flatten)]
    pub cache: Cache,
}

#[derive(Parser, Debug, ValueEnum, Clone)]
pub enum Domain {
    Danbooru,
    Safebooru,
}

impl Domain {
    pub fn board(&self) -> Board {
        match self {
            Domain::Danbooru => Board::Danbooru,
            Domain::Safebooru => Board::Safebooru,
        }
    }
}

impl ToString for Domain {
    fn to_string(&self) -> String {
        match self {
            Domain::Danbooru => "danbooru",
            Domain::Safebooru => "safebooru",
        }
        .to_string()
    }
}

#[derive(Args, Debug, Clone)]
pub struct Output {
    /// Output folder path
    #[arg(short, long, default_value = "output")]
    pub output_path: String,

    #[arg(long, default_value_t = 4)]
    pub threads: usize,

    #[arg(long)]
    pub overwrite: bool,

    #[arg(short, long, default_value_t = 20)]
    pub num_posts: u32,

    #[arg(
        short,
        long,
        default_value = "{people}, {character}, {copyright}, {general}, {meta}, {artist}"
    )]
    pub tag_template: String,
}

#[derive(Args, Debug, Clone)]
pub struct Condition {
    #[arg(long, default_value_t = 1)]
    pub score_min: i32,

    #[arg(long, default_value = None)]
    pub score_max: Option<i32>,
}

#[derive(Args, Debug, Clone)]
pub struct Cache {
    /// Cache folder path
    #[arg(long, default_value = "~/.cache/booru-rs/gather")]
    pub cache_path: String,

    /// Cache lifetime
    #[arg(long, default_value = "1week")]
    lifetime: String,
}

impl Cache {
    pub fn lifetime(&self) -> Duration {
        duration_str::parse(&self.lifetime).unwrap()
    }
}
