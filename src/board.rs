use anyhow::Result;

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
