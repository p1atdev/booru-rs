use std::collections::HashMap;

use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Method, RequestBuilder, Response, Url, Version,
};
use serde::Serialize;

use crate::board::{Board, BoardEndpoint, BoardQuery};

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

    // /// Create a new Safebooru Client
    // pub fn safebooru(auth: Auth) -> Result<Self> {
    //     Client::new(Board::Safebooru, auth)
    // }
}

/// Methods
impl Client {
    /// Compose a url with path
    fn _compose(&self, path: &str, query: &str) -> Result<Url> {
        let mut url = Url::parse(self.board.host())?.join(path)?;
        url.set_query(Some(&query));
        Ok(url)
    }

    pub fn compose<E: BoardEndpoint, Q: BoardQuery>(&self, endpoint: E, query: Q) -> Result<Url> {
        self._compose(&endpoint.path(), &query.to_string())
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

        let mut query = danbooru::Query::posts(None);
        query.limit(3);

        let url = client.compose(danbooru::Endpoint::Posts, query).unwrap();
        let res = client.get(url).await.unwrap();

        assert!(res.status().is_success());

        let posts = danbooru::response::posts::Posts::from_str(&res.text().await.unwrap()).unwrap();
        assert_eq!(posts.len(), 3);
    }
}
