use std::time::Duration;
use tokio::time::timeout;

/// Network connectivity status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum ConnectivityStatus {
    Online,
    Offline,
    Unknown,
}

/// Check if the device has network connectivity by making a lightweight HTTP request.
///
/// Attempts to reach a reliable endpoint with a short timeout.
pub async fn check_connectivity() -> ConnectivityStatus {
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(_) => return ConnectivityStatus::Unknown,
    };

    // Try multiple reliable endpoints
    let endpoints = vec![
        "https://1.1.1.1",
        "https://8.8.8.8",
        "https://www.google.com/generate_204",
    ];

    for endpoint in endpoints {
        match timeout(Duration::from_secs(3), client.head(endpoint).send()).await {
            Ok(Ok(response)) if response.status().is_success() || response.status().as_u16() == 204 => {
                return ConnectivityStatus::Online;
            }
            Ok(Ok(_)) => {
                // Got a response but not success - might be captive portal, treat as online
                return ConnectivityStatus::Online;
            }
            _ => continue,
        }
    }

    ConnectivityStatus::Offline
}

/// Information about the offline cache status.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheStatus {
    pub total_articles: i64,
    pub cached_articles: i64,
    pub cached_feeds: i64,
    pub cache_size_mb: f64,
    pub oldest_cached_article: Option<String>,
}

/// Calculate cache status from database information.
///
/// This is a convenience wrapper that the frontend can call to show
/// offline readiness.
pub fn calculate_cache_status(
    total_articles: i64,
    articles_with_content: i64,
    total_feeds: i64,
) -> CacheStatus {
    // Estimate cache size: assume average article content is ~10KB
    let avg_article_size_kb = 10.0;
    let cache_size_mb = (articles_with_content as f64 * avg_article_size_kb) / 1024.0;

    CacheStatus {
        total_articles,
        cached_articles: articles_with_content,
        cached_feeds: total_feeds,
        cache_size_mb,
        oldest_cached_article: None,
    }
}

/// Determine if an article is available for offline reading.
///
/// An article is considered offline-ready if it has cached content.
pub fn is_article_offline_ready(article: &crate::models::Article) -> bool {
    article.content.is_some() && !article.content.as_ref().unwrap().is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_check_connectivity() {
        // This test may pass or fail depending on network, but should not panic
        let status = check_connectivity().await;
        assert!(
            status == ConnectivityStatus::Online
                || status == ConnectivityStatus::Offline
                || status == ConnectivityStatus::Unknown
        );
    }

    #[test]
    fn test_calculate_cache_status() {
        let status = calculate_cache_status(1000, 800, 50);
        assert_eq!(status.total_articles, 1000);
        assert_eq!(status.cached_articles, 800);
        assert_eq!(status.cached_feeds, 50);
        assert!(status.cache_size_mb > 0.0);
    }

    #[test]
    fn test_is_article_offline_ready() {
        let ready = crate::models::Article {
            id: 1,
            feed_id: 1,
            title: "Test".to_string(),
            url: "https://example.com".to_string(),
            content: Some("Content here".to_string()),
            summary: None,
            author: None,
            published_at: Some(Utc::now()),
            read: false,
            starred: false,
            created_at: Utc::now(),
        };

        assert!(is_article_offline_ready(&ready));

        let not_ready = crate::models::Article {
            content: None,
            ..ready.clone()
        };

        assert!(!is_article_offline_ready(&not_ready));

        let empty_content = crate::models::Article {
            content: Some("".to_string()),
            ..ready.clone()
        };

        assert!(!is_article_offline_ready(&empty_content));
    }
}
