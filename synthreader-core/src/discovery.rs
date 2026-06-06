use reqwest;
use scraper::{Html, Selector};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredFeed {
    pub title: String,
    pub url: String,
    pub feed_type: String,
}

pub async fn discover_feeds(url: &str) -> anyhow::Result<Vec<DiscoveredFeed>> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(url).send().await?;
    let body = response.text().await?;

    // First check if the URL itself is a feed
    if looks_like_feed(&body) {
        let title = extract_feed_title(&body).unwrap_or_else(|| "Untitled Feed".to_string());
        let feed_type = if body.contains("application/atom") || body.contains("<feed") {
            "Atom".to_string()
        } else {
            "RSS".to_string()
        };
        return Ok(vec![DiscoveredFeed {
            title,
            url: url.to_string(),
            feed_type,
        }]);
    }

    // Parse HTML synchronously to avoid Send issues with scraper::Html
    let discovered = tokio::task::spawn_blocking({
        let url = url.to_string();
        move || parse_html_for_feeds(&body, &url)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Failed to parse HTML: {}", e))?;

    // Also check for common feed paths if no feeds found in link tags
    if discovered.is_empty() {
        let common_paths = vec![
            "/rss",
            "/rss.xml",
            "/feed",
            "/feed.xml",
            "/atom.xml",
            "/index.xml",
            "/feeds/posts/default",
            "/blog/rss",
            "/blog/feed",
        ];

        let base_url = get_base_url(url)?;
        let mut found = Vec::new();

        for path in common_paths {
            let test_url = format!("{}{}", base_url, path);
            if let Ok(response) = client.head(&test_url).send().await {
                if response.status().is_success() {
                    let content_type = response
                        .headers()
                        .get(reqwest::header::CONTENT_TYPE)
                        .and_then(|v| v.to_str().ok())
                        .unwrap_or("");

                    if content_type.contains("xml") || content_type.contains("rss") || content_type.contains("atom") {
                        found.push(DiscoveredFeed {
                            title: format!("Feed ({})", path),
                            url: test_url,
                            feed_type: "RSS/Atom".to_string(),
                        });
                    }
                }
            }
        }

        // Deduplicate by URL
        let mut all = discovered;
        all.extend(found);
        all.sort_by(|a, b| a.url.cmp(&b.url));
        all.dedup_by(|a, b| a.url == b.url);
        return Ok(all);
    }

    Ok(discovered)
}

fn parse_html_for_feeds(body: &str, url: &str) -> Vec<DiscoveredFeed> {
    let mut discovered = Vec::new();
    let document = Html::parse_document(body);
    let selector = Selector::parse(r#"link[rel="alternate"]"#).unwrap();

    for element in document.select(&selector) {
        let type_attr = element.value().attr("type").unwrap_or("");
        let href = element.value().attr("href").unwrap_or("");
        let title = element.value().attr("title").unwrap_or("").to_string();

        if href.is_empty() {
            continue;
        }

        let feed_type = match type_attr {
            "application/rss+xml" => "RSS",
            "application/atom+xml" => "Atom",
            "application/feed+json" => "JSON Feed",
            "application/json" => {
                if title.to_lowercase().contains("feed") || href.contains("feed") {
                    "JSON Feed"
                } else {
                    continue;
                }
            }
            _ => {
                if href.ends_with(".rss") || href.ends_with("/rss") || href.ends_with("/rss.xml") || href.ends_with("/feed.xml") || href.ends_with("/atom.xml") {
                    "RSS/Atom"
                } else if href.ends_with(".json") {
                    "JSON Feed"
                } else {
                    continue;
                }
            }
        };

        let absolute_url = match resolve_url(url, href) {
            Ok(u) => u,
            Err(_) => continue,
        };

        let feed_title = if title.is_empty() {
            format!("{} Feed", feed_type)
        } else {
            title
        };

        discovered.push(DiscoveredFeed {
            title: feed_title,
            url: absolute_url,
            feed_type: feed_type.to_string(),
        });
    }

    discovered.sort_by(|a, b| a.url.cmp(&b.url));
    discovered.dedup_by(|a, b| a.url == b.url);
    discovered
}

fn looks_like_feed(body: &str) -> bool {
    let trimmed = body.trim_start();
    trimmed.starts_with("<?xml")
        || trimmed.starts_with("<rss")
        || trimmed.starts_with("<feed")
        || trimmed.starts_with("<channel")
}

fn extract_feed_title(body: &str) -> Option<String> {
    // Quick extraction of title from RSS/Atom feed
    if let Some(start) = body.find("<title>") {
        if let Some(end) = body[start + 7..].find("</title>") {
            return Some(body[start + 7..start + 7 + end].trim().to_string());
        }
    }
    None
}

fn resolve_url(base: &str, href: &str) -> anyhow::Result<String> {
    if href.starts_with("http://") || href.starts_with("https://") {
        Ok(href.to_string())
    } else {
        let base_url = reqwest::Url::parse(base)?;
        let resolved = base_url.join(href)?;
        Ok(resolved.to_string())
    }
}

fn get_base_url(url: &str) -> anyhow::Result<String> {
    let parsed = reqwest::Url::parse(url)?;
    let base = format!("{}://{}", parsed.scheme(), parsed.host_str().unwrap_or(""));
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_feed() {
        assert!(looks_like_feed(r#"<?xml version="1.0"?><rss>..."#));
        assert!(looks_like_feed(r#"<feed xmlns="http://www.w3.org/2005/Atom">..."#));
        assert!(!looks_like_feed(r#"<html>...</html>"#));
    }

    #[test]
    fn test_resolve_url() {
        assert_eq!(
            resolve_url("https://example.com/blog", "/feed.xml").unwrap(),
            "https://example.com/feed.xml"
        );
        assert_eq!(
            resolve_url("https://example.com", "https://other.com/feed.xml").unwrap(),
            "https://other.com/feed.xml"
        );
    }
}
