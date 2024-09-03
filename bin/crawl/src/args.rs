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
    pub date: Date,
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
