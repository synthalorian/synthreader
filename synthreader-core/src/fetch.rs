use crate::feed_parser::{parse_feed, ParsedFeed};

pub struct FeedFetcher {
    client: reqwest::Client,
}

impl FeedFetcher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn fetch(&self,
        url: &str,
    ) -> anyhow::Result<String> {
        let response = self.client.get(url).send().await?;
        let body = response.text().await?;
        Ok(body)
    }

    pub async fn fetch_and_parse(&self,
        url: &str,
    ) -> anyhow::Result<ParsedFeed> {
        let content = self.fetch(url).await?;
        let feed = parse_feed(url, &content)?;
        Ok(feed)
    }
}
