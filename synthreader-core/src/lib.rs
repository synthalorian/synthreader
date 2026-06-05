pub mod db;
pub mod formats;
pub mod metadata;
pub mod conversion;
pub mod sync;
pub mod ai;
pub mod fonts;

pub mod models;
pub mod feed_parser;
pub mod fetch;
pub mod refresh;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookMetadata {
    pub title: String,
    pub subtitle: Option<String>,
    pub authors: Vec<String>,
    pub description: Option<String>,
    pub publisher: Option<String>,
    pub published_date: Option<String>,
    pub language: Option<String>,
    pub isbn_10: Option<String>,
    pub isbn_13: Option<String>,
    pub page_count: Option<i32>,
    pub cover_data: Option<Vec<u8>>,
    pub cover_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BookFormat {
    Epub,
    Mobi,
    Azw3,
    Pdf,
    Txt,
}

impl BookFormat {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "epub" => Some(Self::Epub),
            "mobi" => Some(Self::Mobi),
            "azw3" => Some(Self::Azw3),
            "pdf" => Some(Self::Pdf),
            "txt" => Some(Self::Txt),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Epub => "epub",
            Self::Mobi => "mobi",
            Self::Azw3 => "azw3",
            Self::Pdf => "pdf",
            Self::Txt => "txt",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TocEntry {
    pub label: String,
    pub href: String,
    pub children: Vec<TocEntry>,
}

// Re-export key types from models
pub use models::{Article, Feed, Folder, Tag};
pub use feed_parser::{ParsedFeed, ParsedArticle};
pub use refresh::{FeedRefreshService, RefreshStats, run_refresh};
