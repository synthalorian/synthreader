use crate::models::Article;

/// Export an article to Markdown format.
///
/// Produces a clean Markdown document with YAML frontmatter
/// containing metadata, followed by the article content.
pub fn export_article_to_markdown(article: &Article) -> String {
    let mut md = String::new();

    // YAML frontmatter
    md.push_str("---\n");
    md.push_str(&format!("title: \"{}\"\n", escape_yaml(&article.title)));
    if let Some(author) = &article.author {
        md.push_str(&format!("author: \"{}\"\n", escape_yaml(author)));
    }
    md.push_str(&format!("url: \"{}\"\n", article.url));
    if let Some(published) = article.published_at {
        md.push_str(&format!("date: \"{}\"\n", published.to_rfc3339()));
    }
    md.push_str(&format!("starred: {}\n", article.starred));
    md.push_str(&format!("read: {}\n", article.read));
    md.push_str("---\n\n");

    // Title
    md.push_str(&format!("# {}\n\n", article.title));

    // Metadata line
    if let Some(author) = &article.author {
        md.push_str(&format!("**By:** {} ", author));
    }
    if let Some(published) = article.published_at {
        md.push_str(&format!("**Published:** {} ", published.format("%Y-%m-%d %H:%M")));
    }
    md.push_str(&format!("**URL:** <{}>\n\n", article.url));

    // Content
    if let Some(content) = &article.content {
        // Convert basic HTML to Markdown if content looks like HTML
        if content.contains('<') && content.contains('>') {
            md.push_str(&html_to_markdown(content));
        } else {
            md.push_str(content);
        }
    } else if let Some(summary) = &article.summary {
        md.push_str(summary);
    }

    md.push_str("\n\n---\n");
    md.push_str(&format!("*Exported from Synthreader on {}*\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

    md
}

/// Export multiple articles as a single Markdown document.
pub fn export_articles_to_markdown(articles: &[Article]) -> String {
    let mut md = String::new();
    md.push_str(&format!("# Exported Articles ({})\n\n", articles.len()));
    md.push_str(&format!("*Exported from Synthreader on {}*\n\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));
    md.push_str("---\n\n");

    for (i, article) in articles.iter().enumerate() {
        if i > 0 {
            md.push_str("\n---\n\n");
        }
        md.push_str(&export_article_to_markdown(article));
    }

    md
}

/// Export articles as an OPML reading list.
pub fn export_articles_to_opml(articles: &[Article]) -> String {
    use crate::opml::{generate_opml, OpmlFeed};

    let feeds: Vec<OpmlFeed> = articles
        .iter()
        .map(|a| OpmlFeed {
            title: a.title.clone(),
            url: a.url.clone(),
            site_url: None,
        })
        .collect();

    generate_opml("Synthreader Reading List", &[], &feeds)
}

/// Simple HTML-to-Markdown converter for article content.
fn html_to_markdown(html: &str) -> String {
    let mut md = html.to_string();

    md = md.replace("<br>", "\n").replace("<br/>", "\n").replace("<br />", "\n");

    md = replace_tag(&md, "h1", "# ", "\n\n");
    md = replace_tag(&md, "h2", "## ", "\n\n");
    md = replace_tag(&md, "h3", "### ", "\n\n");
    md = replace_tag(&md, "h4", "#### ", "\n\n");
    md = replace_tag(&md, "p", "", "\n\n");
    md = replace_tag(&md, "div", "", "\n\n");
    md = replace_tag(&md, "li", "- ", "\n");

    md = replace_inline_tag(&md, "strong", "**");
    md = replace_inline_tag(&md, "b", "**");
    md = replace_inline_tag(&md, "em", "*");
    md = replace_inline_tag(&md, "i", "*");
    md = replace_inline_tag(&md, "code", "`");

    md = strip_remaining_tags(&md);

    md = md
        .replace("\n\n\n\n", "\n\n")
        .replace("\n\n\n", "\n\n");

    md.trim().to_string()
}

fn replace_tag(html: &str, tag: &str, prefix: &str, suffix: &str) -> String {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    html.replace(&open, prefix).replace(&close, suffix)
}

fn replace_inline_tag(html: &str, tag: &str, marker: &str) -> String {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    html.replace(&open, marker).replace(&close, marker)
}

fn strip_remaining_tags(html: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }

    result
}

fn escape_yaml(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_article() -> Article {
        Article {
            id: 1,
            feed_id: 1,
            title: "Test Article".to_string(),
            url: "https://example.com/article".to_string(),
            content: Some("<p>This is the article content.</p>".to_string()),
            summary: Some("A summary".to_string()),
            author: Some("John Doe".to_string()),
            published_at: Some(Utc::now()),
            read: false,
            starred: true,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_export_article_to_markdown() {
        let article = create_test_article();
        let md = export_article_to_markdown(&article);

        assert!(md.contains("# Test Article"));
        assert!(md.contains("John Doe"));
        assert!(md.contains("https://example.com/article"));
        assert!(md.contains("starred: true"));
        assert!(md.contains("read: false"));
        assert!(md.contains("Exported from Synthreader"));
    }

    #[test]
    fn test_export_multiple_articles() {
        let articles = vec![create_test_article(), create_test_article()];
        let md = export_articles_to_markdown(&articles);

        assert!(md.contains("# Exported Articles (2)"));
        assert!(md.contains("# Test Article"));
    }

    #[test]
    fn test_export_article_without_content() {
        let article = Article {
            content: None,
            ..create_test_article()
        };
        let md = export_article_to_markdown(&article);

        assert!(md.contains("A summary"));
    }

    #[test]
    fn test_html_to_markdown_basic() {
        let html = "<h1>Title</h1><p>Paragraph with <strong>bold</strong> and <em>italic</em>.</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.contains("**bold**"));
        assert!(md.contains("*italic*"));
    }

    #[test]
    fn test_export_articles_to_opml() {
        let articles = vec![create_test_article()];
        let opml = export_articles_to_opml(&articles);
        assert!(opml.contains("<opml"));
        assert!(opml.contains("Test Article"));
        assert!(opml.contains("https://example.com/article"));
    }
}
