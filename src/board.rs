use anyhow::Result;
use std::collections::HashMap;

pub mod danbooru;
pub mod safebooru;

/// Supported WebSite enum
#[derive(Debug, Clone)]
pub enum Board {
    Danbooru,
    Safebooru,
}

impl Board {
    pub fn host(&self) -> &str {
        match self {
            Board::Danbooru => danbooru::HOST,
            Board::Safebooru => safebooru::HOST,
        }
    }
}

/// Response object post from board
pub trait BoardResponse {
    fn from_str(s: &str) -> Result<Self>
    where
        Self: Sized;
}

/// Endpoint trait
pub trait BoardEndpoint {
    fn path(&self) -> String;
}

/// Request query
pub trait BoardQuery {
    /// convert to query string
    fn to_string(&self) -> String;

    /// insert query parameter
    fn insert<T: ToString, K: ToString>(&mut self, key: T, value: K);

    /// insert "limit" query parameter
    fn limit(&mut self, limit: i64) {
        self.insert("limit", limit);
    }
}

/// search tags builder
pub trait BoardSearchTagsBuilder {
    /// create new tags builder
    fn new() -> Self;

    /// get current tags
    fn tags(&self) -> Vec<String>;

    /// get current metatags
    fn metatags(&self) -> HashMap<String, String>;

    /// add search tags to builder
    fn add_tag(&mut self, tag: &str);

    /// add search metatags to builder
    fn set_metatag(&mut self, key: &str, value: Vec<String>);

    /// append search metatags to builder
    fn append_metatag(&mut self, key: &str, value: &str);

    /// convert to query string
    fn build(&self) -> String;
}
