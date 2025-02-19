use booru::board::Board;
use clap::{Args, Parser, ValueEnum};

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[arg(short, long)]
    pub domain: Domain,

    #[arg(short, long, default_value = "")]
    pub tags: String,

    #[command(flatten)]
    pub id: Id,

    #[command(flatten)]
    pub output: Output,

    #[arg(long, env = "DANBOORU_USERNAME", hide_env_values = true)]
    pub username: String,
    #[arg(long, env = "DANBOORU_API_KEY", hide_env_values = true)]
    pub api_key: String,
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
pub struct Id {
    #[arg(long, default_value_t = 1)]
    pub id_start: u32,

    #[arg(long)]
    pub id_end: u32,
}

#[derive(Args, Debug, Clone)]
pub struct Date {
    #[arg(long, default_value_t = 2024)]
    pub year_start: u16,

    #[arg(long)]
    pub year_end: Option<u16>,

    #[arg(long, default_value_t = 1)]
    pub month_start: u8,

    #[arg(long)]
    pub month_end: Option<u8>,
}

#[derive(Args, Debug, Clone)]
pub struct Output {
    /// Output folder path
    #[arg(short, long, default_value = "output")]
    pub output_path: String,

    #[arg(short, long)]
    pub prefix: Option<String>,

    #[arg(long, default_value_t = 4)]
    pub write_concurrency: usize,

    #[arg(long, default_value_t = 8)]
    pub max_requests_per_second: usize,

    #[arg(long)]
    pub overwrite: bool,
}
