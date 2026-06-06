use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Feed {
    pub id: i64,
    pub title: String,
    pub url: String,
    pub site_url: Option<String>,
    pub description: Option<String>,
    pub folder_id: Option<i64>,
    pub last_fetched: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub consecutive_errors: i64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Article {
    pub id: i64,
    pub feed_id: i64,
    pub title: String,
    pub url: String,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub read: bool,
    pub starred: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Folder {
    pub id: i64,
    pub name: String,
    pub parent_id: Option<i64>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: Option<String>,
}

/// Health status of a feed based on recent fetch attempts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedHealth {
    Healthy,
    Warning,
    Error,
}

/// Statistics for a single feed.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FeedStats {
    pub feed_id: i64,
    pub feed_title: String,
    pub total_articles: i64,
    pub unread_count: i64,
    pub starred_count: i64,
    pub last_fetched: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub consecutive_errors: i64,
}

/// Summary of overall library statistics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibraryStats {
    pub total_feeds: i64,
    pub total_articles: i64,
    pub total_unread: i64,
    pub total_starred: i64,
    pub feeds_with_errors: i64,
}
