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
                }
                Err(e) => {
                    warn!("Failed to refresh feed '{}': {}", feed.title, e);
                }
            }
        }

        Ok(())
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

#[derive(Debug, Clone, serde::Serialize)]
pub struct RefreshStats {
    pub total_new: usize,
    pub errors: usize,
}
