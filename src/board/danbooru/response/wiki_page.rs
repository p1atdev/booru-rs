use serde::{Deserialize, Serialize};

use crate::board::BoardResponse;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WikiPage {
    pub id: i64,
    pub created_at: String,
    pub updated_at: String,
    pub title: String,
    pub other_names: Vec<String>,
    pub body: String,
    pub is_locked: bool,
    pub is_deleted: bool,
}

impl BoardResponse for WikiPage {
    fn from_str(s: &str) -> anyhow::Result<Self> {
        let wiki_page: WikiPage = serde_json::from_str(s)?;
        Ok(wiki_page)
    }
}
