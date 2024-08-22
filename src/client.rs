use std::collections::HashMap;

use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Method, RequestBuilder, Response, Url, Version,
};
use serde::Serialize;

use crate::board::Board;

/// Auth struct
#[derive(Debug, Clone)]
pub struct Auth {
    username: String,
    api_key: String,
}

impl Auth {
    /// Create a new Auth struct
    pub fn new(username: &str, api_key: &str) -> Self {
        Auth {
            username: username.to_string(),
            api_key: api_key.to_string(),
        }
    }

    /// Get basic auth
    pub fn basic(&self) -> String {
        format!(
            "Basic {}",
            general_purpose::STANDARD.encode(&format!("{}:{}", self.username, self.api_key))
        )
    }
}

/// Query struct
pub struct Query(HashMap<String, String>);

impl Query {
    /// Create a new Query struct
    pub fn new() -> Self {
        Query(HashMap::new())
    }

    /// convert to query string
    pub fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join("&")
    }

    pub fn insert<T: ToString, K: ToString>(&mut self, key: T, value: K) {
        self.0.insert(key.to_string(), value.to_string());
    }
}

/// Danbooru API Client
#[derive(Debug, Clone)]
pub struct Client {
    client: reqwest::Client,
    pub board: Board,
}

/// Initialization
impl Client {
    /// Create a new Client
    pub fn new(board: Board, auth: Auth) -> Result<Self> {
        // create default headers
        let mut headers: HeaderMap = HeaderMap::new();
        headers.insert(
            header::USER_AGENT,
            HeaderValue::from_static("danboorust client"),
        );
        headers.insert(header::AUTHORIZATION, HeaderValue::from_str(&auth.basic())?);

        // get client builder and gen client
        let client_builder = reqwest::Client::builder()
            .http3_prior_knowledge() // enable http3
            .default_headers(headers);

        let client = client_builder.build()?;

        Ok(Client { client, board })
    }

    /// Create a new Danbooru Client
    pub fn danbooru(auth: Auth) -> Result<Self> {
        Client::new(Board::Danbooru, auth)
    }

    /// Create a new Safebooru Client
    pub fn safebooru(auth: Auth) -> Result<Self> {
        Client::new(Board::Safebooru, auth)
    }
}

/// Methods
impl Client {
    /// Compose a url
    pub fn compose(&self, path: &str, query: &Query) -> Result<Url> {
        let mut url = Url::parse(self.board.host())?.join(path)?;
        url.set_query(Some(&query.to_string()));
        Ok(url)
    }

    /// create request builder
    fn request_builder(&self, method: Method, url: Url) -> RequestBuilder {
        let builder = self.client.request(method, url).version(Version::HTTP_3);
        builder
    }

    /// Get request
    pub async fn get(&self, url: Url) -> Result<Response> {
        let builder = self.request_builder(Method::GET, url);
        let res = builder.send().await?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::{
        board::{danbooru, BoardResponse},
        test_utils::Env,
    };

    #[test]
    fn test_auth() {
        let auth = Auth::new("username", "PassW0rd!");

        assert_eq!(auth.basic(), "Basic dXNlcm5hbWU6UGFzc1cwcmQh");
    }

    #[tokio::test]
    async fn test_get_posts() {
        let env = Env::new();

        let auth = Auth::new(&env.username, &env.api_key);
        let client = Client::danbooru(auth).unwrap();

        let mut query = Query::new();
        query.insert("limit", 3);

        let url = client.compose("/posts.json", &query).unwrap();
        let res = client.get(url).await.unwrap();

        assert!(res.status().is_success());

        let posts = danbooru::Posts::from_str(&res.text().await.unwrap()).unwrap();
        assert_eq!(posts.len(), 3);
    }
}
