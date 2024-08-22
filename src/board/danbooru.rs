use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::BoardResponse;

pub const HOST: &str = "https://danbooru.donmai.us";

pub type Posts = Vec<Post>;

/// Danbooru Post
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    id: i64,
    created_at: String,
    updated_at: String,

    // score
    score: i64,
    source: String,
    up_score: i64,
    down_score: i64,
    fav_count: i64,
    rating: String,

    // image size
    image_width: i64,
    image_height: i64,

    // tag
    tag_count: i64,
    tag_string: String,
    tag_string_general: String,
    tag_string_character: String,
    tag_string_copyright: String,
    tag_string_artist: String,
    tag_string_meta: String,
    tag_count_general: i64,
    tag_count_artist: i64,
    tag_count_character: i64,
    tag_count_copyright: i64,
    tag_count_meta: i64,

    // url
    has_large: bool,
    media_asset: MediaAsset,
    file_url: String,
    large_file_url: String,
    preview_file_url: String,

    // relation
    parent_id: Option<i64>,
    has_children: bool,
    has_active_children: bool,
    has_visible_children: bool,

    // user
    last_commented_at: Option<String>,
    last_comment_bumped_at: Option<String>,
    last_noted_at: Option<String>,

    // file
    file_size: i64,
    file_ext: String,
    md5: String,

    // user
    uploader_id: i64,
    approver_id: Option<String>,
    pixiv_id: Option<i64>,

    // status
    is_pending: bool,
    is_flagged: bool,
    is_deleted: bool,
    is_banned: bool,
    bit_flags: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaAsset {
    id: i64,
    created_at: String,
    updated_at: String,
    md5: String,
    file_ext: String,
    file_size: i64,
    image_width: i64,
    image_height: i64,
    duration: Option<f32>,
    status: String,
    file_key: String,
    is_public: bool,
    pixel_hash: String,
    variants: Vec<Variant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    #[serde(rename = "type")]
    variant_type: VariantType,
    url: String,
    width: i64,
    height: i64,
    file_ext: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VariantType {
    Sample,
    Original,
    #[serde(other)]
    WxH,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileExt {
    Jpg,
    Png,
    Webp,
    Webm,
    Zip,
    Mp4,
    Gif,
    #[serde(other)]
    Other,
}

impl BoardResponse for Post {
    fn from_str(s: &str) -> Result<Self> {
        let post: Post = serde_json::from_str(s)?;
        Ok(post)
    }
}

impl BoardResponse for Posts {
    fn from_str(s: &str) -> Result<Self> {
        let posts: Posts = serde_json::from_str(s)?;
        Ok(posts)
    }
}
