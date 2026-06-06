use std::sync::Arc;
use std::time::Duration;
use tokio::time::interval;
use tracing::{error, info, warn};

use crate::db::LibraryDb;
use crate::fetch::FeedFetcher;

pub struct FeedRefreshService {
    db: Arc<LibraryDb>,
    fetcher: FeedFetcher,
}

impl FeedRefreshService {
    pub fn new(db: Arc<LibraryDb>) -> Self {
        Self {
            db,
            fetcher: FeedFetcher::new(),
        }
    }

    /// Refresh all feeds once.
    pub async fn refresh_all(&self) -> anyhow::Result<()> {
        let feeds = self.db.get_feeds().await?;
        info!("Refreshing {} feeds", feeds.len());

        for feed in feeds {
            match self.refresh_feed(feed.id, &feed.url).await {
                Ok(new_count) => {
                    info!("Feed '{}' refreshed, {} new articles", feed.title, new_count);
                    if let Err(e) = self.db.clear_feed_error(feed.id).await {
                        warn!("Failed to clear error for feed '{}': {}", feed.title, e);
                    }
                }
                Err(e) => {
                    warn!("Failed to refresh feed '{}': {}", feed.title, e);
                    if let Err(db_err) = self.db.record_feed_error(feed.id, &e.to_string()).await {
                        warn!("Failed to record error for feed '{}': {}", feed.title, db_err);
                    }
                }
            }
        }

        Ok(())
    }

    /// Refresh all feeds with progress reporting.
    pub async fn refresh_all_with_progress(
        &self,
        progress_callback: impl Fn(RefreshProgress),
    ) -> anyhow::Result<RefreshStats> {
        let feeds = self.db.get_feeds().await?;
        let total_feeds = feeds.len();
        info!("Refreshing {} feeds", total_feeds);

        let mut total_new = 0;
        let mut errors = 0;

        for (index, feed) in feeds.iter().enumerate() {
            progress_callback(RefreshProgress {
                current_feed: feed.title.clone(),
                current_index: index + 1,
                total_feeds,
                status: RefreshStatus::InProgress,
            });

            match self.refresh_feed(feed.id, &feed.url).await {
                Ok(new_count) => {
                    total_new += new_count;
                    info!("Feed '{}' refreshed, {} new articles", feed.title, new_count);
                    if let Err(e) = self.db.clear_feed_error(feed.id).await {
                        warn!("Failed to clear error for feed '{}': {}", feed.title, e);
                    }
                }
                Err(e) => {
                    errors += 1;
                    warn!("Failed to refresh feed '{}': {}", feed.title, e);
                    if let Err(db_err) = self.db.record_feed_error(feed.id, &e.to_string()).await {
                        warn!("Failed to record error for feed '{}': {}", feed.title, db_err);
                    }
                }
            }
        }

        progress_callback(RefreshProgress {
            current_feed: String::new(),
            current_index: total_feeds,
            total_feeds,
            status: RefreshStatus::Complete,
        });

        Ok(RefreshStats { total_new, errors })
    }

    /// Refresh a single feed by ID and URL.
    async fn refresh_feed(
        &self,
        feed_id: i64,
        url: &str,
    ) -> anyhow::Result<usize> {
        let parsed = self.fetcher.fetch_and_parse(url).await?;
        let mut new_count = 0;

        for article in parsed.articles {
            // Skip if article already exists (by URL)
            let existing: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM articles WHERE feed_id = ?1 AND url = ?2 LIMIT 1"
            )
            .bind(feed_id)
            .bind(&article.url)
            .fetch_optional(self.db.pool())
            .await?;

            if existing.is_none() {
                self.db
                    .add_article(
                        feed_id,
                        &article.title,
                        &article.url,
                        article.content.as_deref(),
                        article.summary.as_deref(),
                        article.author.as_deref(),
                        article.published_at,
                    )
                    .await?;
                new_count += 1;
            }
        }

        self.db.update_feed_last_fetched(feed_id).await?;

        Ok(new_count)
    }

    /// Start a background task that refreshes feeds at the given interval.
    pub fn start_background(self: Arc<Self>, period: Duration) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = interval(period);
            ticker.tick().await; // First tick fires immediately

            loop {
                ticker.tick().await;
                if let Err(e) = self.refresh_all().await {
                    error!("Background refresh error: {}", e);
                }
            }
        })
    }

    /// Auto-refresh feeds on startup with progress reporting.
    ///
    /// Checks connectivity first, then refreshes all feeds if online.
    pub async fn auto_refresh_on_startup(
        &self,
        progress_callback: impl Fn(RefreshProgress),
    ) -> anyhow::Result<AutoRefreshResult> {
        let connectivity = crate::offline::check_connectivity().await;

        match connectivity {
            crate::offline::ConnectivityStatus::Online => {
                let stats = self.refresh_all_with_progress(progress_callback).await?;
                Ok(AutoRefreshResult {
                    connectivity,
                    stats,
                    skipped: false,
                })
            }
            crate::offline::ConnectivityStatus::Offline => {
                info!("Skipping auto-refresh: device is offline");
                Ok(AutoRefreshResult {
                    connectivity,
                    stats: RefreshStats { total_new: 0, errors: 0 },
                    skipped: true,
                })
            }
            crate::offline::ConnectivityStatus::Unknown => {
                warn!("Connectivity unknown, attempting refresh anyway");
                let stats = self.refresh_all_with_progress(progress_callback).await?;
                Ok(AutoRefreshResult {
                    connectivity,
                    stats,
                    skipped: false,
                })
            }
        }
    }
}

/// Progress information during a feed refresh operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefreshProgress {
    pub current_feed: String,
    pub current_index: usize,
    pub total_feeds: usize,
    pub status: RefreshStatus,
}

/// Status of a refresh operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum RefreshStatus {
    InProgress,
    Complete,
    Error,
}

/// Result of an auto-refresh on startup.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AutoRefreshResult {
    pub connectivity: crate::offline::ConnectivityStatus,
    pub stats: RefreshStats,
    pub skipped: bool,
}

/// Convenience: run a one-off refresh and return stats.
pub async fn run_refresh(db: Arc<LibraryDb>) -> anyhow::Result<RefreshStats> {
    let service = FeedRefreshService::new(db);
    let feeds = service.db.get_feeds().await?;
    let mut total_new = 0;
    let mut errors = 0;

    for feed in feeds {
        match service.refresh_feed(feed.id, &feed.url).await {
            Ok(new_count) => total_new += new_count,
            Err(_) => errors += 1,
        }
    }

    Ok(RefreshStats { total_new, errors })
}

/// Run a one-off refresh with progress reporting.
pub async fn run_refresh_with_progress(
    db: Arc<LibraryDb>,
    progress_callback: impl Fn(RefreshProgress),
) -> anyhow::Result<RefreshStats> {
    let service = FeedRefreshService::new(db);
    service.refresh_all_with_progress(progress_callback).await
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RefreshStats {
    pub total_new: usize,
    pub errors: usize,
}
