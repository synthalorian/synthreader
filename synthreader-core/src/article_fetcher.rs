use reqwest;
use crate::readability;

/// Fetch article HTML and extract readable content.
pub async fn fetch_and_extract(url: &str) -> anyhow::Result<readability::ExtractedContent> {
    let client = reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let response = client.get(url).send().await?;
    let html = response.text().await?;

    readability::extract_content(&html, Some(url))
}
