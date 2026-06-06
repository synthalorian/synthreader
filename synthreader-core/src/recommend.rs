use std::collections::{HashMap, HashSet};

use crate::models::Article;

/// A feed recommendation based on reading history.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FeedRecommendation {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub reason: String,
    pub score: f64,
}

/// Generate feed recommendations based on user's reading history.
///
/// Analyzes the titles and content of read/starred articles to extract
/// common topics, then suggests feeds that match those interests.
pub fn recommend_feeds(
    articles: &[Article],
    existing_feed_urls: &HashSet<String>,
    candidate_feeds: &[CandidateFeed],
) -> Vec<FeedRecommendation> {
    if articles.is_empty() || candidate_feeds.is_empty() {
        return Vec::new();
    }

    // Build interest profile from article titles and content
    let interest_profile = build_interest_profile(articles);

    if interest_profile.is_empty() {
        return Vec::new();
    }

    let mut scored: Vec<FeedRecommendation> = candidate_feeds
        .iter()
        .filter(|cf| !existing_feed_urls.contains(&cf.url))
        .map(|cf| {
            let score = score_feed_match(&interest_profile, cf);
            let reason = generate_reason(&interest_profile, cf);

            FeedRecommendation {
                title: cf.title.clone(),
                url: cf.url.clone(),
                description: cf.description.clone(),
                reason,
                score,
            }
        })
        .filter(|r| r.score > 0.0)
        .collect();

    // Sort by score descending
    scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    // Return top recommendations
    scored.into_iter().take(10).collect()
}

/// A candidate feed that could be recommended.
#[derive(Debug, Clone)]
pub struct CandidateFeed {
    pub title: String,
    pub url: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub category: Option<String>,
}

/// Build an interest profile from articles.
///
/// Returns a map of terms to their weighted frequency.
fn build_interest_profile(articles: &[Article]) -> HashMap<String, f64> {
    let mut profile: HashMap<String, f64> = HashMap::new();

    for article in articles {
        let weight = if article.starred {
            2.0
        } else if article.read {
            1.0
        } else {
            0.3
        };

        let text = format!(
            "{} {} {}",
            article.title,
            article.summary.as_deref().unwrap_or(""),
            article.content.as_deref().unwrap_or("")
        );

        let tokens = crate::search::tokenize(&text);
        for token in tokens {
            *profile.entry(token).or_insert(0.0) += weight;
        }
    }

    // Normalize scores
    let max_score = profile.values().copied().fold(0.0, f64::max);
    if max_score > 0.0 {
        for score in profile.values_mut() {
            *score /= max_score;
        }
    }

    profile
}

/// Score how well a feed matches the interest profile.
fn score_feed_match(profile: &HashMap<String, f64>, feed: &CandidateFeed) -> f64 {
    let mut score = 0.0;

    let feed_text = format!(
        "{} {} {} {}",
        feed.title,
        feed.description.as_deref().unwrap_or(""),
        feed.tags.join(" "),
        feed.category.as_deref().unwrap_or("")
    );

    let tokens = crate::search::tokenize(&feed_text);
    let token_set: HashSet<String> = tokens.into_iter().collect();

    for (term, weight) in profile {
        if token_set.contains(term) {
            score += weight;
        }
    }

    score
}

/// Generate a human-readable reason for the recommendation.
fn generate_reason(profile: &HashMap<String, f64>, feed: &CandidateFeed) -> String {
    let feed_text = format!(
        "{} {} {}",
        feed.title,
        feed.description.as_deref().unwrap_or(""),
        feed.tags.join(" ")
    );

    let tokens = crate::search::tokenize(&feed_text);
    let token_set: HashSet<String> = tokens.into_iter().collect();

    let mut matching: Vec<(&String, &f64)> = profile
        .iter()
        .filter(|(term, _)| token_set.contains(term.as_str()))
        .collect();

    matching.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap());

    let top_terms: Vec<String> = matching
        .into_iter()
        .take(3)
        .map(|(term, _)| term.clone())
        .collect();

    if top_terms.is_empty() {
        "Recommended based on your reading history".to_string()
    } else {
        format!("Because you read about: {}", top_terms.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_article(title: &str, read: bool, starred: bool) -> Article {
        Article {
            id: 1,
            feed_id: 1,
            title: title.to_string(),
            url: "https://example.com".to_string(),
            content: None,
            summary: None,
            author: None,
            published_at: Some(Utc::now()),
            read,
            starred,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_recommend_feeds_basic() {
        let articles = vec![
            create_article("Rust programming tutorial", true, false),
            create_article("Advanced Rust techniques", true, true),
            create_article("Python for beginners", true, false),
        ];

        let existing: HashSet<String> = HashSet::new();

        let candidates = vec![
            CandidateFeed {
                title: "Rust Weekly".to_string(),
                url: "https://rustweekly.com".to_string(),
                description: Some("Weekly Rust news and tutorials".to_string()),
                tags: vec!["rust".to_string(), "programming".to_string()],
                category: Some("Technology".to_string()),
            },
            CandidateFeed {
                title: "Gardening Tips".to_string(),
                url: "https://gardening.com".to_string(),
                description: Some("How to grow tomatoes".to_string()),
                tags: vec!["gardening".to_string()],
                category: Some("Hobbies".to_string()),
            },
        ];

        let recs = recommend_feeds(&articles, &existing, &candidates);

        assert!(!recs.is_empty());
        assert_eq!(recs[0].title, "Rust Weekly");
        assert!(recs[0].score > recs.get(1).map_or(0.0, |r| r.score));
    }

    #[test]
    fn test_recommend_feeds_filters_existing() {
        let articles = vec![create_article("Rust tutorial", true, false)];

        let mut existing = HashSet::new();
        existing.insert("https://rustweekly.com".to_string());

        let candidates = vec![CandidateFeed {
            title: "Rust Weekly".to_string(),
            url: "https://rustweekly.com".to_string(),
            description: None,
            tags: vec![],
            category: None,
        }];

        let recs = recommend_feeds(&articles, &existing, &candidates);
        assert!(recs.is_empty());
    }

    #[test]
    fn test_recommend_feeds_empty_articles() {
        let articles: Vec<Article> = vec![];
        let existing: HashSet<String> = HashSet::new();
        let candidates = vec![CandidateFeed {
            title: "Test".to_string(),
            url: "https://test.com".to_string(),
            description: None,
            tags: vec![],
            category: None,
        }];

        let recs = recommend_feeds(&articles, &existing, &candidates);
        assert!(recs.is_empty());
    }

    #[test]
    fn test_generate_reason() {
        let mut profile = HashMap::new();
        profile.insert("rust".to_string(), 1.0);
        profile.insert("programming".to_string(), 0.8);

        let feed = CandidateFeed {
            title: "Rust Weekly".to_string(),
            url: "https://rustweekly.com".to_string(),
            description: Some("Programming news".to_string()),
            tags: vec!["rust".to_string()],
            category: None,
        };

        let reason = generate_reason(&profile, &feed);
        assert!(reason.contains("rust") || reason.contains("programming"));
    }
}
