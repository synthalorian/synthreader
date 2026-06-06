use std::collections::{HashMap, HashSet};

use crate::models::Article;

/// A related article with a similarity score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RelatedArticle {
    pub article_id: i64,
    pub title: String,
    pub url: String,
    pub similarity_score: f64,
    pub reason: String,
}

/// Find related articles to a given article based on content similarity.
///
/// Uses TF-IDF weighted cosine similarity between article text content.
/// Returns the top `limit` most similar articles, excluding the source article.
pub fn find_related_articles(
    source_article: &Article,
    all_articles: &[Article],
    limit: usize,
) -> Vec<RelatedArticle> {
    if all_articles.len() < 2 {
        return Vec::new();
    }

    let source_tokens = tokenize_article(source_article);
    if source_tokens.is_empty() {
        return Vec::new();
    }

    let corpus = build_corpus(all_articles);
    let idf = compute_idf(&corpus);
    let source_vector = build_vector(&source_tokens, &idf);

    let mut scored: Vec<RelatedArticle> = all_articles
        .iter()
        .filter(|a| a.id != source_article.id)
        .map(|article| {
            let article_tokens = tokenize_article(article);
            let article_vector = build_vector(&article_tokens, &idf);
            let similarity = cosine_similarity(&source_vector, &article_vector);

            let common_terms = find_common_terms(&source_tokens, &article_tokens);
            let reason = if common_terms.is_empty() {
                "Similar content".to_string()
            } else {
                format!("Shared topics: {}", common_terms.join(", "))
            };

            RelatedArticle {
                article_id: article.id,
                title: article.title.clone(),
                url: article.url.clone(),
                similarity_score: similarity,
                reason,
            }
        })
        .filter(|r| r.similarity_score > 0.0)
        .collect();

    scored.sort_by(|a, b| b.similarity_score.partial_cmp(&a.similarity_score).unwrap());
    scored.into_iter().take(limit).collect()
}

/// Tokenize an article into a set of meaningful terms.
fn tokenize_article(article: &Article) -> Vec<String> {
    let text = format!(
        "{} {} {}",
        article.title,
        article.summary.as_deref().unwrap_or(""),
        article.content.as_deref().unwrap_or("")
    );

    crate::search::tokenize(&text)
}

/// Build corpus: map of article id to its tokens.
fn build_corpus(articles: &[Article]) -> HashMap<i64, Vec<String>> {
    articles
        .iter()
        .map(|a| (a.id, tokenize_article(a)))
        .collect()
}

/// Compute IDF (inverse document frequency) for each term in the corpus.
fn compute_idf(corpus: &HashMap<i64, Vec<String>>) -> HashMap<String, f64> {
    let total_docs = corpus.len() as f64;
    let mut doc_freq: HashMap<String, i64> = HashMap::new();

    for tokens in corpus.values() {
        let unique_tokens: HashSet<String> = tokens.iter().cloned().collect();
        for token in unique_tokens {
            *doc_freq.entry(token).or_insert(0) += 1;
        }
    }

    doc_freq
        .into_iter()
        .map(|(term, df)| {
            let idf = (total_docs / (1.0 + df as f64)).ln();
            (term, idf)
        })
        .collect()
}

/// Build a TF-IDF weighted vector for a document.
fn build_vector(tokens: &[String], idf: &HashMap<String, f64>) -> HashMap<String, f64> {
    let mut tf: HashMap<String, f64> = HashMap::new();

    for token in tokens {
        *tf.entry(token.clone()).or_insert(0.0) += 1.0;
    }

    let max_tf = tf.values().copied().fold(0.0, f64::max);

    tf.into_iter()
        .map(|(term, count)| {
            let normalized_tf = count / max_tf;
            let weight = normalized_tf * idf.get(&term).copied().unwrap_or(0.0);
            (term, weight)
        })
        .collect()
}

/// Compute cosine similarity between two sparse vectors.
fn cosine_similarity(a: &HashMap<String, f64>, b: &HashMap<String, f64>) -> f64 {
    let mut dot_product = 0.0;
    let mut a_norm_sq = 0.0;
    let mut b_norm_sq = 0.0;

    for (term, weight_a) in a {
        a_norm_sq += weight_a * weight_a;
        if let Some(weight_b) = b.get(term) {
            dot_product += weight_a * weight_b;
        }
    }

    for weight_b in b.values() {
        b_norm_sq += weight_b * weight_b;
    }

    let norm = (a_norm_sq.sqrt() * b_norm_sq.sqrt()).max(f64::EPSILON);
    dot_product / norm
}

/// Find common terms between two token lists.
fn find_common_terms(a: &[String], b: &[String]) -> Vec<String> {
    let set_a: HashSet<String> = a.iter().cloned().collect();
    let set_b: HashSet<String> = b.iter().cloned().collect();

    let mut common: Vec<String> = set_a.intersection(&set_b).cloned().collect();
    common.sort();
    common
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_article(id: i64, title: &str, content: Option<&str>) -> Article {
        Article {
            id,
            feed_id: 1,
            title: title.to_string(),
            url: format!("https://example.com/{}", id),
            content: content.map(String::from),
            summary: None,
            author: None,
            published_at: Some(Utc::now()),
            read: false,
            starred: false,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_find_related_articles_basic() {
        let source = create_article(1, "Rust programming basics", Some("Learn Rust programming language from scratch with tutorials and examples"));

        let articles = vec![
            source.clone(),
            create_article(2, "Advanced Rust techniques", Some("Master Rust programming language with advanced tutorials and patterns")),
            create_article(3, "Python for beginners", Some("Learn Python programming language from scratch")),
            create_article(4, "Rust concurrency patterns", Some("Advanced Rust programming patterns and concurrency techniques")),
        ];

        let related = find_related_articles(&source, &articles, 3);

        assert!(!related.is_empty());
        assert!(related[0].similarity_score > 0.0);
        assert!(related.iter().any(|r| r.title.contains("Rust")));
    }

    #[test]
    fn test_find_related_articles_excludes_source() {
        let source = create_article(1, "Test", None);
        let articles = vec![source.clone()];

        let related = find_related_articles(&source, &articles, 3);
        assert!(related.is_empty());
    }

    #[test]
    fn test_find_related_articles_empty_corpus() {
        let source = create_article(1, "Test", None);
        let articles: Vec<Article> = vec![];

        let related = find_related_articles(&source, &articles, 3);
        assert!(related.is_empty());
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let mut a = HashMap::new();
        a.insert("rust".to_string(), 1.0);
        a.insert("programming".to_string(), 0.5);

        let mut b = HashMap::new();
        b.insert("rust".to_string(), 1.0);
        b.insert("programming".to_string(), 0.5);

        let sim = cosine_similarity(&a, &b);
        assert!((sim - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let mut a = HashMap::new();
        a.insert("rust".to_string(), 1.0);

        let mut b = HashMap::new();
        b.insert("python".to_string(), 1.0);

        let sim = cosine_similarity(&a, &b);
        assert!(sim.abs() < 0.001);
    }

    #[test]
    fn test_find_common_terms() {
        let a = vec!["rust".to_string(), "programming".to_string()];
        let b = vec!["rust".to_string(), "python".to_string()];

        let common = find_common_terms(&a, &b);
        assert_eq!(common, vec!["rust"]);
    }
}
