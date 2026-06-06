use regex::RegexBuilder;

/// Highlight search terms in text by wrapping matches in a marker.
///
/// Returns the text with all occurrences of query terms wrapped in `[[HIGHLIGHT]]...[[/HIGHLIGHT]]`.
/// Matching is case-insensitive and matches whole words when `whole_words` is true.
///
/// # Example
/// ```
/// use synthreader_core::search::highlight_text;
/// let result = highlight_text("Rust is a great language", "great", false);
/// assert_eq!(result, "Rust is a [[HIGHLIGHT]]great[[/HIGHLIGHT]] language");
/// ```
pub fn highlight_text(text: &str, query: &str, whole_words: bool) -> String {
    if query.trim().is_empty() {
        return text.to_string();
    }

    // Escape regex special characters in the query
    let escaped = regex::escape(query);

    let pattern = if whole_words {
        format!(r"\b{}\b", escaped)
    } else {
        escaped
    };

    let re = match RegexBuilder::new(&pattern)
        .case_insensitive(true)
        .build()
    {
        Ok(re) => re,
        Err(_) => return text.to_string(),
    };

    re.replace_all(text, "[[HIGHLIGHT]]$0[[/HIGHLIGHT]]").to_string()
}

/// Highlight multiple search terms at once.
pub fn highlight_text_multi(text: &str, queries: &[&str], whole_words: bool) -> String {
    let mut result = text.to_string();

    for query in queries {
        if query.trim().is_empty() {
            continue;
        }
        result = highlight_text(&result, query, whole_words);
    }

    result
}

/// Extract context snippets around search matches.
///
/// Returns up to `max_snippets` snippets, each containing the match
/// surrounded by up to `context_chars` characters of context.
pub fn extract_snippets(
    text: &str,
    query: &str,
    context_chars: usize,
    max_snippets: usize,
) -> Vec<String> {
    if query.trim().is_empty() || text.is_empty() {
        return Vec::new();
    }

    let escaped = regex::escape(query);
    let re = match RegexBuilder::new(&escaped)
        .case_insensitive(true)
        .build()
    {
        Ok(re) => re,
        Err(_) => return Vec::new(),
    };

    let mut snippets = Vec::new();

    for mat in re.find_iter(text) {
        if snippets.len() >= max_snippets {
            break;
        }

        let start = mat.start().saturating_sub(context_chars);
        let end = (mat.end() + context_chars).min(text.len());

        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < text.len() { "..." } else { "" };

        let snippet = format!(
            "{}{}{}",
            prefix,
            highlight_text(&text[start..end], query, false),
            suffix
        );

        snippets.push(snippet);
    }

    snippets
}

/// Tokenize text into searchable terms.
///
/// Splits on whitespace and punctuation, filters out common stop words,
/// and returns lowercase tokens.
pub fn tokenize(text: &str) -> Vec<String> {
    let stop_words: std::collections::HashSet<&str> = [
        "the", "a", "an", "is", "are", "was", "were", "be", "been",
        "being", "have", "has", "had", "do", "does", "did", "will",
        "would", "could", "should", "may", "might", "must", "shall",
        "can", "need", "dare", "ought", "used", "to", "of", "in",
        "for", "on", "with", "at", "by", "from", "as", "into",
        "through", "during", "before", "after", "above", "below",
        "between", "under", "and", "but", "or", "yet", "so", "if",
        "because", "although", "though", "while", "where", "when",
        "that", "which", "who", "whom", "whose", "what", "this",
        "these", "those", "i", "you", "he", "she", "it", "we", "they",
        "me", "him", "her", "us", "them", "my", "your", "his", "its",
        "our", "their", "mine", "yours", "hers", "ours", "theirs",
        "myself", "yourself", "himself", "herself", "itself", "ourselves",
        "themselves", "what", "which", "who", "whom", "this", "that",
        "these", "those", "am", "is", "are", "was", "were", "be",
        "been", "being", "have", "has", "had", "do", "does", "did",
        "a", "an", "the", "and", "but", "if", "or", "because", "as",
        "until", "while", "of", "at", "by", "for", "with", "about",
        "against", "between", "into", "through", "during", "before",
        "after", "above", "below", "to", "from", "up", "down", "in",
        "out", "on", "off", "over", "under", "again", "further", "then",
        "once", "here", "there", "when", "where", "why", "how", "all",
        "any", "both", "each", "few", "more", "most", "other", "some",
        "such", "no", "nor", "not", "only", "own", "same", "so", "than",
        "too", "very", "s", "t", "just", "don", "now",
    ]
    .iter()
    .copied()
    .collect();

    text.to_lowercase()
        .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
        .filter(|s| !s.is_empty() && s.len() > 2 && !stop_words.contains(s))
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_text() {
        let text = "Rust is a great programming language. Rust is fast.";
        let result = highlight_text(text, "rust", false);
        assert_eq!(
            result,
            "[[HIGHLIGHT]]Rust[[/HIGHLIGHT]] is a great programming language. [[HIGHLIGHT]]Rust[[/HIGHLIGHT]] is fast."
        );
    }

    #[test]
    fn test_highlight_text_empty_query() {
        let text = "Hello world";
        let result = highlight_text(text, "", false);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_highlight_text_no_match() {
        let text = "Hello world";
        let result = highlight_text(text, "xyz", false);
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_highlight_text_whole_words() {
        let text = "The rustlang community loves Rust.";
        let result = highlight_text(text, "rust", true);
        assert!(result.contains("[[HIGHLIGHT]]Rust[[/HIGHLIGHT]]"));
        assert!(result.contains("rustlang"));
    }

    #[test]
    fn test_highlight_text_multi() {
        let text = "Rust and Python are great languages";
        let result = highlight_text_multi(text, &["rust", "python"], false);
        assert!(result.contains("[[HIGHLIGHT]]Rust[[/HIGHLIGHT]]"));
        assert!(result.contains("[[HIGHLIGHT]]Python[[/HIGHLIGHT]]"));
    }

    #[test]
    fn test_extract_snippets() {
        let text = "The quick brown fox jumps over the lazy dog. The quick brown fox was very quick.";
        let snippets = extract_snippets(text, "quick", 10, 2);
        assert_eq!(snippets.len(), 2);
        assert!(snippets[0].contains("[[HIGHLIGHT]]quick[[/HIGHLIGHT]]"));
        assert!(snippets[0].starts_with("The ") || snippets[0].starts_with("..."));
    }

    #[test]
    fn test_extract_snippets_empty() {
        let snippets = extract_snippets("", "test", 10, 2);
        assert!(snippets.is_empty());
    }

    #[test]
    fn test_tokenize() {
        let text = "The quick brown fox jumps over the lazy dog!";
        let tokens = tokenize(text);
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
        assert!(!tokens.contains(&"the".to_string()));
        assert!(!tokens.contains(&"over".to_string()));
    }

    #[test]
    fn test_tokenize_filters_short_words() {
        let text = "a bc def ghij";
        let tokens = tokenize(text);
        assert!(!tokens.contains(&"a".to_string()));
        assert!(!tokens.contains(&"bc".to_string()));
        assert!(tokens.contains(&"def".to_string()));
        assert!(tokens.contains(&"ghij".to_string()));
    }
}
