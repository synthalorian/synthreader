use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpmlFeed {
    pub title: String,
    pub url: String,
    pub site_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpmlFolder {
    pub title: String,
    pub feeds: Vec<OpmlFeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpmlDocument {
    pub title: String,
    pub folders: Vec<OpmlFolder>,
    pub feeds: Vec<OpmlFeed>,
}

/// Parse Pocket or Instapaper OPML export.
///
/// Pocket and Instapaper exports use slightly different conventions:
/// - They may use `type="link"` for bookmarks
/// - They may use `url` instead of `xmlUrl` for some entries
/// - Tags/folders are often represented as parent outlines
///
/// This function normalizes these formats into our standard OpmlDocument.
pub fn parse_pocket_opml(xml: &str) -> anyhow::Result<OpmlDocument> {
    let doc = roxmltree::Document::parse(xml)?;
    let root = doc.root_element();

    let opml_elem = if root.tag_name().name() == "opml" {
        root
    } else {
        root.children()
            .find(|n| n.is_element() && n.tag_name().name() == "opml")
            .ok_or_else(|| anyhow::anyhow!("No <opml> root element found"))?
    };

    let head = opml_elem
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "head");

    let title = head
        .and_then(|h| {
            h.children()
                .find(|n| n.is_element() && n.tag_name().name() == "title")
        })
        .and_then(|t| t.text())
        .unwrap_or("Pocket Export")
        .to_string();

    let body = opml_elem
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "body")
        .ok_or_else(|| anyhow::anyhow!("No <body> element found"))?;

    let mut folders = Vec::new();
    let mut feeds = Vec::new();

    for child in body.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "outline" {
            let outline_type = child.attribute("type").unwrap_or("");
            
            if outline_type == "link" || child.has_attribute("url") {
                if let Some(feed) = parse_pocket_outline_feed(&child) {
                    feeds.push(feed);
                }
            } else if child.has_attribute("xmlUrl") {
                if let Some(feed) = parse_outline_feed(&child) {
                    feeds.push(feed);
                }
            } else {
                if let Some(folder) = parse_pocket_outline_folder(&child)
                    && !folder.feeds.is_empty()
                {
                    folders.push(folder);
                }
            }
        }
    }

    Ok(OpmlDocument {
        title,
        folders,
        feeds,
    })
}

fn parse_pocket_outline_feed(node: &roxmltree::Node) -> Option<OpmlFeed> {
    let url = node.attribute("xmlUrl")
        .or_else(|| node.attribute("url"))?;
    
    let title = node
        .attribute("text")
        .or_else(|| node.attribute("title"))
        .unwrap_or(url)
        .to_string();
    
    let site_url = node.attribute("htmlUrl")
        .or_else(|| node.attribute("url"))
        .map(String::from);

    Some(OpmlFeed {
        title,
        url: url.to_string(),
        site_url,
    })
}

fn parse_pocket_outline_folder(node: &roxmltree::Node) -> Option<OpmlFolder> {
    let title = node
        .attribute("text")
        .or_else(|| node.attribute("title"))
        .unwrap_or("Untitled")
        .to_string();

    let mut feeds = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "outline" {
            let outline_type = child.attribute("type").unwrap_or("");
            if outline_type == "link" || child.has_attribute("url") {
                if let Some(feed) = parse_pocket_outline_feed(&child) {
                    feeds.push(feed);
                }
            } else if let Some(feed) = parse_outline_feed(&child) {
                feeds.push(feed);
            }
        }
    }

    Some(OpmlFolder { title, feeds })
}

pub fn parse_opml(xml: &str) -> anyhow::Result<OpmlDocument> {
    let doc = roxmltree::Document::parse(xml)?;
    let root = doc.root_element();

    let opml_elem = if root.tag_name().name() == "opml" {
        root
    } else {
        root.children()
            .find(|n| n.is_element() && n.tag_name().name() == "opml")
            .ok_or_else(|| anyhow::anyhow!("No <opml> root element found"))?
    };

    let head = opml_elem
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "head");

    let title = head
        .and_then(|h| {
            h.children()
                .find(|n| n.is_element() && n.tag_name().name() == "title")
        })
        .and_then(|t| t.text())
        .unwrap_or("Untitled")
        .to_string();

    let body = opml_elem
        .children()
        .find(|n| n.is_element() && n.tag_name().name() == "body")
        .ok_or_else(|| anyhow::anyhow!("No <body> element found"))?;

    let mut folders = Vec::new();
    let mut feeds = Vec::new();

    for child in body.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "outline" {
            if child.has_attribute("xmlUrl") {
                // It's a feed at the root level
                if let Some(feed) = parse_outline_feed(&child) {
                    feeds.push(feed);
                }
            } else {
                // It's a folder
                if let Some(folder) = parse_outline_folder(&child) {
                    folders.push(folder);
                }
            }
        }
    }

    Ok(OpmlDocument {
        title,
        folders,
        feeds,
    })
}

fn parse_outline_feed(node: &roxmltree::Node) -> Option<OpmlFeed> {
    let url = node.attribute("xmlUrl")?;
    let title = node
        .attribute("text")
        .or_else(|| node.attribute("title"))
        .unwrap_or(url)
        .to_string();
    let site_url = node.attribute("htmlUrl").map(String::from);

    Some(OpmlFeed {
        title,
        url: url.to_string(),
        site_url,
    })
}

fn parse_outline_folder(node: &roxmltree::Node) -> Option<OpmlFolder> {
    let title = node
        .attribute("text")
        .or_else(|| node.attribute("title"))
        .unwrap_or("Untitled")
        .to_string();

    let mut feeds = Vec::new();
    for child in node.children().filter(|n| n.is_element()) {
        if child.tag_name().name() == "outline" && let Some(feed) = parse_outline_feed(&child) {
            feeds.push(feed);
        }
    }

    Some(OpmlFolder { title, feeds })
}

pub fn generate_opml(title: &str, folders: &[OpmlFolder], feeds: &[OpmlFeed]) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<opml version="2.0">"#);
    xml.push('\n');
    xml.push_str("  <head>\n");
    xml.push_str(&format!("    <title>{}</title>\n", xml_escape(title)));
    xml.push_str("  </head>\n");
    xml.push_str("  <body>\n");

    for folder in folders {
        xml.push_str(&format!(
            "    <outline text=\"{}\" title=\"{}\">\n",
            xml_escape(&folder.title),
            xml_escape(&folder.title)
        ));
        for feed in &folder.feeds {
            xml.push_str(&format!(
                "      <outline text=\"{}\" title=\"{}\" type=\"rss\" xmlUrl=\"{}\"{} />\n",
                xml_escape(&feed.title),
                xml_escape(&feed.title),
                xml_escape(&feed.url),
                feed.site_url
                    .as_ref()
                    .map(|u| format!(" htmlUrl=\"{}\"", xml_escape(u)))
                    .unwrap_or_default()
            ));
        }
        xml.push_str("    </outline>\n");
    }

    for feed in feeds {
        xml.push_str(&format!(
            "    <outline text=\"{}\" title=\"{}\" type=\"rss\" xmlUrl=\"{}\"{} />\n",
            xml_escape(&feed.title),
            xml_escape(&feed.title),
            xml_escape(&feed.url),
            feed.site_url
                .as_ref()
                .map(|u| format!(" htmlUrl=\"{}\"", xml_escape(u)))
                .unwrap_or_default()
        ));
    }

    xml.push_str("  </body>\n");
    xml.push_str("</opml>\n");

    xml
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_opml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="2.0">
  <head>
    <title>My Feeds</title>
  </head>
  <body>
    <outline text="Tech" title="Tech">
      <outline text="Hacker News" title="Hacker News" type="rss" xmlUrl="https://news.ycombinator.com/rss" htmlUrl="https://news.ycombinator.com" />
    </outline>
    <outline text="xkcd" title="xkcd" type="rss" xmlUrl="https://xkcd.com/rss.xml" />
  </body>
</opml>
"#;

        let doc = parse_opml(xml).unwrap();
        assert_eq!(doc.title, "My Feeds");
        assert_eq!(doc.folders.len(), 1);
        assert_eq!(doc.folders[0].title, "Tech");
        assert_eq!(doc.folders[0].feeds.len(), 1);
        assert_eq!(doc.folders[0].feeds[0].title, "Hacker News");
        assert_eq!(doc.folders[0].feeds[0].url, "https://news.ycombinator.com/rss");
        assert_eq!(doc.feeds.len(), 1);
        assert_eq!(doc.feeds[0].title, "xkcd");
    }

    #[test]
    fn test_generate_opml() {
        let folders = vec![OpmlFolder {
            title: "Tech".to_string(),
            feeds: vec![OpmlFeed {
                title: "HN".to_string(),
                url: "https://hnrss.org/frontpage".to_string(),
                site_url: Some("https://news.ycombinator.com".to_string()),
            }],
        }];

        let feeds = vec![OpmlFeed {
            title: "xkcd".to_string(),
            url: "https://xkcd.com/rss.xml".to_string(),
            site_url: None,
        }];

        let xml = generate_opml("My Feeds", &folders, &feeds);
        assert!(xml.contains("<opml version=\"2.0\""));
        assert!(xml.contains("<title>My Feeds</title>"));
        assert!(xml.contains("xmlUrl=\"https://hnrss.org/frontpage\""));
        assert!(xml.contains("htmlUrl=\"https://news.ycombinator.com\""));
        assert!(xml.contains("xmlUrl=\"https://xkcd.com/rss.xml\""));
    }

    #[test]
    fn test_parse_pocket_opml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="1.0">
  <head>
    <title>Pocket Export</title>
  </head>
  <body>
    <outline text="Unread" title="Unread">
      <outline text="Rust Blog" title="Rust Blog" type="rss" xmlUrl="https://blog.rust-lang.org/feed.xml" htmlUrl="https://blog.rust-lang.org" />
      <outline text="Example Bookmark" title="Example Bookmark" type="link" url="https://example.com" />
    </outline>
    <outline text="Articles" title="Articles">
      <outline text="Tech News" title="Tech News" type="link" url="https://technews.com" />
    </outline>
    <outline text="Direct Feed" title="Direct Feed" type="rss" xmlUrl="https://direct.com/feed.xml" />
  </body>
</opml>
"#;

        let doc = parse_pocket_opml(xml).unwrap();
        assert_eq!(doc.title, "Pocket Export");
        assert_eq!(doc.folders.len(), 2);
        
        let unread_folder = &doc.folders[0];
        assert_eq!(unread_folder.title, "Unread");
        assert_eq!(unread_folder.feeds.len(), 2);
        assert_eq!(unread_folder.feeds[0].title, "Rust Blog");
        assert_eq!(unread_folder.feeds[0].url, "https://blog.rust-lang.org/feed.xml");
        assert_eq!(unread_folder.feeds[1].title, "Example Bookmark");
        assert_eq!(unread_folder.feeds[1].url, "https://example.com");
        
        let articles_folder = &doc.folders[1];
        assert_eq!(articles_folder.title, "Articles");
        assert_eq!(articles_folder.feeds.len(), 1);
        assert_eq!(articles_folder.feeds[0].url, "https://technews.com");
        
        assert_eq!(doc.feeds.len(), 1);
        assert_eq!(doc.feeds[0].title, "Direct Feed");
        assert_eq!(doc.feeds[0].url, "https://direct.com/feed.xml");
    }

    #[test]
    fn test_parse_instapaper_opml() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<opml version="1.0">
  <head>
    <title>Instapaper: Starred</title>
  </head>
  <body>
    <outline text="Starred" title="Starred">
      <outline text="Hacker News" title="Hacker News" type="rss" xmlUrl="https://news.ycombinator.com/rss" htmlUrl="https://news.ycombinator.com" />
      <outline text="Saved Link" title="Saved Link" type="link" url="https://saved.com/article" />
    </outline>
  </body>
</opml>
"#;

        let doc = parse_pocket_opml(xml).unwrap();
        assert!(doc.title.contains("Instapaper"));
        assert_eq!(doc.folders.len(), 1);
        assert_eq!(doc.folders[0].feeds.len(), 2);
        assert_eq!(doc.folders[0].feeds[0].url, "https://news.ycombinator.com/rss");
        assert_eq!(doc.folders[0].feeds[1].url, "https://saved.com/article");
    }
}
