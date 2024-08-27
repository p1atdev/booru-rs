pub mod response;
pub mod search;

use std::{collections::HashMap, str::FromStr};

use anyhow::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

use super::{BoardEndpoint, BoardQuery, BoardResponse, BoardSearchTagsBuilder};

pub const HOST: &str = "https://danbooru.donmai.us";

// -- commmon types --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rating {
    #[serde(rename = "g")]
    General,
    #[serde(rename = "s")]
    Sensitive,
    #[serde(rename = "q")]
    Questionable,
    #[serde(rename = "e")]
    Explicit,
}

impl ToString for Rating {
    fn to_string(&self) -> String {
        match self {
            Rating::General => "g".to_string(),
            Rating::Sensitive => "s".to_string(),
            Rating::Questionable => "q".to_string(),
            Rating::Explicit => "e".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileExt {
    #[serde(alias = "jpeg")]
    Jpg,
    Png,
    Webp,
    Webm,
    Zip,
    Mp4,
    Gif,
    Avif,
}

impl ToString for FileExt {
    fn to_string(&self) -> String {
        match self {
            FileExt::Jpg => "jpg".to_string(),
            FileExt::Png => "png".to_string(),
            FileExt::Webp => "webp".to_string(),
            FileExt::Webm => "webm".to_string(),
            FileExt::Zip => "zip".to_string(),
            FileExt::Mp4 => "mp4".to_string(),
            FileExt::Gif => "gif".to_string(),
            FileExt::Avif => "avif".to_string(),
            // FileExt::Other => "other".to_string(),
        }
    }
}

// -- danbooru types --

/// danbooru api endpoint
#[derive(Debug, Clone)]
pub enum Endpoint {
    Posts,
    Post(i64),
}

impl BoardEndpoint for Endpoint {
    fn path(&self) -> String {
        match self {
            Endpoint::Posts => "/posts.json".to_string(),
            Endpoint::Post(id) => format!("/posts/{}.json", id),
        }
    }
}

/// danbooru api query
#[derive(Debug, Clone)]
pub struct Query(Vec<(String, String)>);

impl Query {
    pub fn new() -> Self {
        Query(Vec::new())
    }

    /// request parameters for /posts.json
    pub fn posts(tags: Option<&str>) -> Self {
        let mut query = Query::new();
        if tags.is_some() {
            query.insert("tags", tags.unwrap());
        }
        query
    }

    /// request parameters for /post/{id}.json
    pub fn post() -> Self {
        Query::new()
    }
}

impl BoardQuery for Query {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<String>>()
            .join("&")
    }

    fn insert<T: ToString, K: ToString>(&mut self, key: T, value: K) {
        self.0.push((key.to_string(), value.to_string()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rating_to_string() {
        assert_eq!(Rating::General.to_string(), "g");
        assert_eq!(Rating::Sensitive.to_string(), "s");
        assert_eq!(Rating::Questionable.to_string(), "q");
        assert_eq!(Rating::Explicit.to_string(), "e");
    }

    #[test]
    fn test_endpoint_path() {
        assert_eq!(Endpoint::Posts.path(), "/posts.json");
        assert_eq!(Endpoint::Post(1234).path(), "/posts/1234.json");
    }

    #[test]
    fn test_query_to_string() {
        let mut query = Query::posts(Some("1girl"));
        query.insert("limit", 3);
        assert_eq!(query.to_string(), "tags=1girl&limit=3");
    }
}
