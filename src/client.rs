use crate::board::{Board, BoardEndpoint, BoardQuery, BoardResponse};
use anyhow::Result;
use base64::{engine::general_purpose, Engine};
use reqwest::{
    header::{self, HeaderMap, HeaderValue},
    Method, RequestBuilder, Response, Url, Version,
};
use std::sync::Arc;

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
    client: Arc<reqwest::Client>,
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
            .default_headers(headers)
            .http2_prior_knowledge();

        #[cfg(feature = "http3")]
        let client_builder = reqwest::Client::builder()
            .default_headers(headers)
            .http3_prior_knowledge();

        let client = client_builder.build()?;

        Ok(Client {
            client: Arc::new(client),
            board,
        })
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
    pub fn request_builder(&self, method: Method, url: Url) -> RequestBuilder {
        let builder =
            self.client
                .clone()
                .request(method, url)
                .version(match cfg!(feature = "http3") {
                    true => Version::HTTP_3,
                    false => Version::HTTP_2,
                });
        builder
    }

    /// Send get request and return response
    pub async fn fetch_raw(&self, url: Url, method: Method) -> Result<Response> {
        let builder = self.request_builder(method, url);
        let res = builder.send().await?;
        Ok(res)
    }

    /// Send get request and return response as specified type
    pub async fn fetch<T: BoardResponse>(&self, url: Url, method: Method) -> Result<T> {
        let res = self.fetch_raw(url, method).await?;
        let text = res.text().await?;
        let res = T::from_str(&text)?;
        Ok(res)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::board::{danbooru, safebooru, BoardSearchTagsBuilder};
    use crate::test_utils::Env;

    #[test]
    fn test_auth() {
        let auth = Auth::new("username", "PassW0rd!");

        assert_eq!(auth.basic(), "Basic dXNlcm5hbWU6UGFzc1cwcmQh");
    }

    #[tokio::test]
    async fn test_base_url() {
        let env = Env::new();

        let auth = Auth::new(&env.username, &env.api_key);
        let client = Client::danbooru(auth.clone()).unwrap();

        assert_eq!(client.board.host(), "https://danbooru.donmai.us");

        let client = Client::safebooru(auth.clone()).unwrap();

        assert_eq!(client.board.host(), "https://safebooru.donmai.us");
    }

    #[tokio::test]
    async fn test_danbooru_get_posts() {
        let env = Env::new();

        let auth = Auth::new(&env.username, &env.api_key);
        let client = Client::danbooru(auth).unwrap();

        let mut query = danbooru::Query::new();
        query.limit(3);

        let url = client.compose(danbooru::Endpoint::Posts, query).unwrap();
        let res = client.fetch_raw(url, Method::GET).await.unwrap();

        assert!(res.status().is_success());

        let posts = danbooru::response::Posts::from_str(&res.text().await.unwrap()).unwrap();
        assert_eq!(posts.len(), 3);
    }

    #[tokio::test]
    async fn test_danbooru_search_posts() {
        let env = Env::new();

        let auth = Auth::new(&env.username, &env.api_key);
        let client = Client::danbooru(auth).unwrap();

        let mut builder = danbooru::SearchTagsBuilder::new();
        builder.add_tag("2girls");
        builder.add_tag("cat_ears");
        builder.ratings(vec![danbooru::Rating::General]);
        let filetypes = vec![danbooru::FileExt::Webp];
        builder.filetypes(filetypes.clone());
        builder.scores(vec![danbooru::search::Score::Min(10)]); // score:>=10

        let mut query = danbooru::Query::posts(&builder.build());
        query.limit(3);

        let url = client.compose(danbooru::Endpoint::Posts, query).unwrap();
        let res = client.fetch_raw(url, Method::GET).await.unwrap();

        assert!(res.status().is_success());

        let posts = danbooru::response::Posts::from_str(&res.text().await.unwrap()).unwrap();
        assert_eq!(posts.len(), 3);

        // check
        for post in posts {
            assert!(post.tag_string_general.contains("2girls"));
            assert!(post.tag_string_general.contains("cat_ears"));
            assert_eq!(post.rating, danbooru::Rating::General);
            assert!(filetypes.contains(&post.file_ext));
            assert!(post.score >= 10);
        }
    }

    #[tokio::test]
    async fn test_safebooru_get_posts() {
        let env = Env::new();

        let auth = Auth::new(&env.username, &env.api_key);
        let client = Client::safebooru(auth).unwrap();

        let mut query = safebooru::Query::new();
        query.limit(10);

        let url = client.compose(safebooru::Endpoint::Posts, query).unwrap();
        let posts = client
            .fetch::<safebooru::response::Posts>(url, Method::GET)
            .await
            .unwrap();

        assert_eq!(posts.len(), 10);
    }
}
