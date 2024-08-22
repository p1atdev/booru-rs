use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use reqwest::{
    header::{self, HeaderMap, HeaderName, HeaderValue},
    tls, Client, ClientBuilder, RequestBuilder, Response, Url, Version,
};
use std::{collections::HashMap, sync::Arc, time::Duration};

const USERNAME: &str = "p1atdev";
const API_KEY: &str = "s69u9FHckSPKWQ4MPYpd58SE";

#[tokio::main]
async fn main() -> Result<()> {
    let url = Url::parse("https://danbooru.donmai.us").unwrap();

    // create default headers
    let mut default_headers: HeaderMap = HeaderMap::new();
    default_headers.insert(header::USER_AGENT, HeaderValue::from_static("reqwest"));
    default_headers.insert(
        header::AUTHORIZATION,
        HeaderValue::from_str(&format!(
            "Basic {}",
            general_purpose::STANDARD.encode(&format!("{}:{}", USERNAME, API_KEY))
        ))?,
    );

    // get client builder and gen client
    let client_builder: ClientBuilder = Client::builder()
        .http3_prior_knowledge() // enable http3
        .default_headers(default_headers)
        .timeout(Duration::from_secs(10));
    let client = client_builder.build()?;

    // get request builder
    let mut post_url = url.join("/posts.json")?;
    post_url.set_query(Some("page=1"));
    let request_builder: RequestBuilder = client.get(post_url).version(Version::HTTP_3);
    println!("{:?}", client);
    let res: Response = request_builder.send().await?;

    // get response headers
    for (name, value) in res.headers().iter() {
        println!("{}: {}", name, value.to_str()?);
    }

    // get response body
    // let text = res.text().await?;
    // println!("{}", text);

    Ok(())
}
