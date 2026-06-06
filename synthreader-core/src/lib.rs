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
pub mod readability;
pub mod article_fetcher;
pub mod opml;
pub mod discovery;
pub mod search;
pub mod recommend;
pub mod similarity;
pub mod offline;
pub mod keyboard;
pub mod export;
pub mod backup;
pub mod notifications;
pub mod settings;

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
pub use refresh::{FeedRefreshService, RefreshStats, RefreshProgress, run_refresh, run_refresh_with_progress};
pub use readability::ExtractedContent;
pub use article_fetcher::fetch_and_extract;
pub use opml::{OpmlDocument, OpmlFeed, OpmlFolder, parse_opml, generate_opml, parse_pocket_opml, parse_feedly_opml, parse_inoreader_opml, parse_newsblur_opml};
pub use discovery::{DiscoveredFeed, discover_feeds};
pub use search::{highlight_text, highlight_text_multi, extract_snippets, tokenize};
pub use recommend::{recommend_feeds, FeedRecommendation, CandidateFeed};
pub use similarity::{find_related_articles, RelatedArticle};
pub use offline::{check_connectivity, ConnectivityStatus, CacheStatus, calculate_cache_status, is_article_offline_ready};
pub use keyboard::KeyboardShortcuts;
pub use export::{export_article_to_markdown, export_articles_to_markdown, export_articles_to_opml};
pub use backup::{backup_database, restore_database, list_backups, delete_backup, cleanup_old_backups, BackupResult, RestoreResult};
pub use notifications::{NotificationSettings, ArticleNotification, NotificationSummary, format_notification_message, format_notification_body};
pub use settings::SettingsManager;
