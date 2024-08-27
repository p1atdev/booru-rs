use super::post::Post;
use crate::board::BoardResponse;
use anyhow::Result;

pub type Posts = Vec<Post>;

impl BoardResponse for Posts {
    fn from_str(s: &str) -> Result<Self> {
        let posts: Posts = serde_json::from_str(s)?;
        Ok(posts)
    }
}
