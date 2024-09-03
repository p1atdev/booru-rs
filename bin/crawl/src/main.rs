mod args;

use anyhow::Result;
use args::Cli;
use booru::board::danbooru::{response, search, Endpoint, FileExt, Query};
use booru::board::{danbooru, BoardQuery, BoardSearchTagsBuilder};
use booru::client::{Auth, Client};
use clap::Parser;
use reqwest::Url;

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
    builder.add_tag("-is:banned");
    builder.filetypes(vec![FileExt::Png, FileExt::Jpg, FileExt::Webp]);

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

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    let env = Env::new();

    let auth = Auth::new(&env.username, &env.api_key);
    let client = Client::new(args.domain.board(), auth)?;

    let year_start = args.date.year_start;
    let month_start = args.date.month_start;
    // let year_end = args.date.year_end;
    // let month_end = args.date.month_end;

    let query = build_query(&year_start, &month_start, &args.tags);

    let url = compose_url(&client, query)?;
    let posts = client
        .fetch::<response::Posts>(url, reqwest::Method::GET)
        .await?;

    println!("posts: {:?}", posts.len());

    Ok(())
}
