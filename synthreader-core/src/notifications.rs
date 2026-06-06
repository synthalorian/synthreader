use chrono::{DateTime, Timelike, Utc};
use serde::{Deserialize, Serialize};

/// A notification for new articles.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ArticleNotification {
    pub id: i64,
    pub article_id: i64,
    pub article_title: String,
    pub feed_name: String,
    pub created_at: DateTime<Utc>,
    pub read: bool,
}

/// Settings for the notification system.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NotificationSettings {
    /// Enable desktop notifications for new articles.
    pub enabled: bool,
    /// Only notify for starred feeds (if false, notify for all).
    pub only_starred_feeds: bool,
    /// Minimum time between notifications (in seconds).
    pub cooldown_seconds: u64,
    /// Maximum notifications per batch.
    pub max_per_batch: usize,
    /// Do not notify during these hours (start, end) in 24h format.
    pub quiet_hours: Option<(u8, u8)>,
    /// Show article summary in notification.
    pub show_summary: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            only_starred_feeds: false,
            cooldown_seconds: 300,
            max_per_batch: 5,
            quiet_hours: Some((22, 8)),
            show_summary: true,
        }
    }
}

impl NotificationSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Check if notifications should be sent right now.
    pub fn should_notify_now(&self) -> bool {
        if !self.enabled {
            return false;
        }

        // Check quiet hours
        if let Some((start, end)) = self.quiet_hours {
            let hour = chrono::Local::now().hour() as u8;
            if start > end {
                // Overnight quiet hours (e.g., 22:00 - 08:00)
                if hour >= start || hour < end {
                    return false;
                }
            } else {
                // Daytime quiet hours (e.g., 12:00 - 14:00)
                if hour >= start && hour < end {
                    return false;
                }
            }
        }

        true
    }
}

/// Summary of pending notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSummary {
    pub unread_count: i64,
    pub latest_article_title: Option<String>,
    pub latest_feed_name: Option<String>,
}

/// Format a desktop notification message for a batch of new articles.
pub fn format_notification_message(articles: &[(String, String)]) -> String {
    match articles.len() {
        0 => String::new(),
        1 => format!("New article from {}", articles[0].1),
        n => format!("{} new articles from {}", n, articles[0].1),
    }
}

/// Format a detailed notification body.
pub fn format_notification_body(articles: &[(String, String)], max_shown: usize) -> String {
    if articles.is_empty() {
        return String::new();
    }

    let shown = articles.iter().take(max_shown);
    let body: Vec<String> = shown
        .map(|(title, _feed)| title.clone())
        .collect();

    let mut result = body.join("\n");

    if articles.len() > max_shown {
        result.push_str(&format!("\n...and {} more", articles.len() - max_shown));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_settings_default() {
        let settings = NotificationSettings::default();
        assert!(settings.enabled);
        assert!(!settings.only_starred_feeds);
        assert_eq!(settings.cooldown_seconds, 300);
        assert_eq!(settings.max_per_batch, 5);
        assert_eq!(settings.quiet_hours, Some((22, 8)));
        assert!(settings.show_summary);
    }

    #[test]
    fn test_notification_settings_serde() {
        let settings = NotificationSettings::default();
        let json = settings.to_json().unwrap();
        let restored = NotificationSettings::from_json(&json).unwrap();
        assert_eq!(settings, restored);
    }

    #[test]
    fn test_should_notify_when_disabled() {
        let mut settings = NotificationSettings::default();
        settings.enabled = false;
        assert!(!settings.should_notify_now());
    }

    #[test]
    fn test_format_notification_message() {
        let articles = vec![
            ("Article 1".to_string(), "Feed A".to_string()),
        ];
        assert_eq!(format_notification_message(&articles), "New article from Feed A");

        let articles = vec![
            ("Article 1".to_string(), "Feed A".to_string()),
            ("Article 2".to_string(), "Feed A".to_string()),
        ];
        assert_eq!(format_notification_message(&articles), "2 new articles from Feed A");
    }

    #[test]
    fn test_format_notification_body() {
        let articles = vec![
            ("Article 1".to_string(), "Feed A".to_string()),
            ("Article 2".to_string(), "Feed A".to_string()),
            ("Article 3".to_string(), "Feed A".to_string()),
        ];
        let body = format_notification_body(&articles, 2);
        assert!(body.contains("Article 1"));
        assert!(body.contains("Article 2"));
        assert!(body.contains("...and 1 more"));
    }
}
