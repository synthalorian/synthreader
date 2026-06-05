use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedArticle {
    pub title: String,
    pub url: String,
    pub content: Option<String>,
    pub summary: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedFeed {
    pub title: String,
    pub url: String,
    pub site_url: Option<String>,
    pub description: Option<String>,
    pub articles: Vec<ParsedArticle>,
}

pub fn parse_feed(url: &str, content: &str) -> anyhow::Result<ParsedFeed> {
    if content.trim_start().starts_with("<?xml") {
        if content.contains("<feed") {
            parse_atom(url, content)
        } else if content.contains("<rss") || content.contains("<channel") {
            parse_rss(url, content)
        } else {
            anyhow::bail!("Unknown XML feed format")
        }
    } else if content.trim_start().starts_with("<feed") {
        parse_atom(url, content)
    } else if content.trim_start().starts_with("<rss") || content.trim_start().starts_with("<channel") {
        parse_rss(url, content)
    } else {
        anyhow::bail!("Unknown feed format")
    }
}

fn parse_rss(url: &str, content: &str) -> anyhow::Result<ParsedFeed> {
    let channel = rss::Channel::read_from(content.as_bytes())?;

    let site_url = if channel.link().is_empty() {
        None
    } else {
        Some(channel.link().to_string())
    };

    let articles = channel
        .items()
        .iter()
        .map(|item| ParsedArticle {
            title: item.title().unwrap_or("Untitled").to_string(),
            url: item.link().unwrap_or(url).to_string(),
            content: item.content().map(String::from),
            summary: item.description().map(String::from),
            author: item.author().map(String::from),
            published_at: item.pub_date().and_then(parse_rfc822),
        })
        .collect();

    Ok(ParsedFeed {
        title: channel.title().to_string(),
        url: url.to_string(),
        site_url,
        description: Some(channel.description().to_string()),
        articles,
    })
}

fn parse_atom(url: &str, content: &str) -> anyhow::Result<ParsedFeed> {
    let feed = atom_syndication::Feed::read_from(content.as_bytes())?;

    let site_url = feed
        .links()
        .iter()
        .find(|link| link.rel() == "alternate" || link.rel() == "self")
        .map(|link| link.href().to_string())
        .or_else(|| feed.id().to_string().into());

    let articles = feed
        .entries()
        .iter()
        .map(|entry| {
            let url = entry
                .links()
                .iter()
                .find(|link| link.rel() == "alternate")
                .map(|link| link.href().to_string())
                .unwrap_or_else(|| entry.id().to_string());

            let author = if entry.authors().is_empty() {
                feed.authors()
                    .first()
                    .map(|a| a.name().to_string())
            } else {
                Some(entry.authors()[0].name().to_string())
            };

            let content = entry.content().and_then(|c| c.value().map(String::from));
            let summary = entry.summary().map(|s| s.value.to_string());

            ParsedArticle {
                title: entry.title().value.to_string(),
                url,
                content,
                summary,
                author,
                published_at: entry.published().map(|d| d.clone().with_timezone(&Utc)).or_else(|| Some(entry.updated().clone().with_timezone(&Utc))),
            }
        })
        .collect();

    Ok(ParsedFeed {
        title: feed.title().value.to_string(),
        url: url.to_string(),
        site_url,
        description: feed.subtitle().map(|s| s.value.to_string()),
        articles,
    })
}

fn parse_rfc822(date_str: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc2822(date_str)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}
