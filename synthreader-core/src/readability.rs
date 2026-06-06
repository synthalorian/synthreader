use legible;

/// Extract readable content from HTML using Mozilla's Readability algorithm.
pub fn extract_content(html: &str, url: Option<&str>) -> anyhow::Result<ExtractedContent> {
    let article = legible::parse(html, url, None)
        .map_err(|e| anyhow::anyhow!("Readability extraction failed: {}", e))?;

    Ok(ExtractedContent {
        title: Some(article.title),
        content: article.content,
        text_content: article.text_content,
        author: article.byline,
        excerpt: article.excerpt,
        site_name: article.site_name,
        lang: article.lang,
        published_time: article.published_time,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExtractedContent {
    pub title: Option<String>,
    pub content: String,
    pub text_content: String,
    pub author: Option<String>,
    pub excerpt: Option<String>,
    pub site_name: Option<String>,
    pub lang: Option<String>,
    pub published_time: Option<String>,
}
