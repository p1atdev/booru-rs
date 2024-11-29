use anyhow::Result;
use booru::board::danbooru;
use booru::board::{BoardQuery, BoardSearchTagsBuilder};
use booru::client::{Auth, Client};
use clap::{Args, Parser, ValueEnum};
use imgcatr::ops;
use reqwest::{Method, Url};
use std::env;
use std::io::stdout;
use std::str::FromStr;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[arg(long, env = "DANBOORU_USERNAME", hide_env_values = true)]
    pub username: String,
    #[arg(long, env = "DANBOORU_API_KEY", hide_env_values = true)]
    pub api_key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let auth = Auth::new(&args.username, &args.api_key);
    let client = Client::danbooru(auth)?;

    // search "tags" builder
    let mut builder = danbooru::SearchTagsBuilder::new();
    builder.add_tag("cat_ears");
    builder.ratings(vec![danbooru::Rating::General]);
    let filetypes = vec![
        danbooru::FileExt::Png,
        danbooru::FileExt::Jpg,
        danbooru::FileExt::Webp,
    ];
    builder.filetypes(filetypes.clone());
    builder.scores(vec![danbooru::search::Score::Min(50)]); // score:>=50
    builder.order(danbooru::search::Order::Random);

    // build "url" search query
    let mut query = danbooru::Query::posts(&builder.build());
    query.limit(3);

    let url = client.compose(danbooru::Endpoint::Posts, query)?;
    let posts = client
        .fetch::<danbooru::response::Posts>(url, Method::GET)
        .await?;

    assert_eq!(posts.len(), 3);

    for post in posts {
        println!("id: {}, updated_at: {}", post.id, post.updated_at);

        let url = post.preview_file_url.unwrap();
        println!("small_url: {}", url);

        // to avoid blocking by cloudflare, use the client instead of empty reqwest
        let res = client.fetch_raw(Url::from_str(&url)?, Method::GET).await?;
        assert!(res.status().is_success());
        let bytes = res.bytes().await?;

        let image = image::io::Reader::new(std::io::Cursor::new(bytes))
            .with_guessed_format()?
            .decode()?;
        // resize, devide by 4
        let image = image.resize(
            image.width() / 4,
            image.height() / 4,
            image::imageops::FilterType::Nearest,
        );

        ops::write_ansi_truecolor(&mut stdout(), &image)
    }

    Ok(())
}
