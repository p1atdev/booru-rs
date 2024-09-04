use crate::board::danbooru::{FileExt, Rating};
use crate::board::BoardResponse;
use anyhow::Result;
use serde::{Deserialize, Serialize};

/// response type for /post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub id: i64,
    pub created_at: String,
    pub updated_at: String,

    // score
    pub score: i64,
    pub source: String,
    pub up_score: i64,
    pub down_score: i64,
    pub fav_count: i64,
    pub rating: Rating,

    // image size
    pub image_width: i64,
    pub image_height: i64,

    // tag
    pub tag_count: i64,
    pub tag_string: String,
    pub tag_string_general: String,
    pub tag_string_character: String,
    pub tag_string_copyright: String,
    pub tag_string_artist: String,
    pub tag_string_meta: String,
    pub tag_count_general: i64,
    pub tag_count_artist: i64,
    pub tag_count_character: i64,
    pub tag_count_copyright: i64,
    pub tag_count_meta: i64,

    // url
    pub has_large: bool,
    pub media_asset: MediaAsset,
    pub file_url: Option<String>, // missing if posts are banned
    pub large_file_url: Option<String>,
    pub preview_file_url: Option<String>,

    // relation
    pub parent_id: Option<i64>,
    pub has_children: bool,
    pub has_active_children: bool,
    pub has_visible_children: bool,

    // user
    pub last_commented_at: Option<String>,
    pub last_comment_bumped_at: Option<String>,
    pub last_noted_at: Option<String>,

    // file
    pub file_size: i64,
    pub file_ext: FileExt,
    pub md5: Option<String>,

    // user
    pub uploader_id: i64,
    pub approver_id: Option<i64>,
    pub pixiv_id: Option<i64>,

    // status
    pub is_pending: bool,
    pub is_flagged: bool,
    pub is_deleted: bool,
    pub is_banned: bool,
    pub bit_flags: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: i64,
    pub created_at: String,
    pub updated_at: String,
    pub md5: Option<String>,
    pub file_ext: FileExt,
    pub file_size: i64,
    pub image_width: i64,
    pub image_height: i64,
    pub duration: Option<f32>,
    pub status: String,
    pub file_key: Option<String>,
    pub is_public: bool,
    pub pixel_hash: String,
    pub variants: Option<Vec<Variant>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    #[serde(rename = "type")]
    pub variant_type: String,
    pub url: String,
    pub width: i64,
    pub height: i64,
    pub file_ext: FileExt,
}

impl BoardResponse for Post {
    fn from_str(s: &str) -> Result<Self> {
        let post: Post = serde_json::from_str(s)?;
        Ok(post)
    }
}
