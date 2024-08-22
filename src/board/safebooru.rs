use crate::board::danbooru::{Post as DPost, Posts as DPosts};

pub const HOST: &str = "https://safebooru.donmai.us";

/// Safebooru Post
pub type Post = DPost;

/// Safebooru Posts
pub type Posts = DPosts;
