use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Default keyboard shortcuts for the application.
///
/// These can be customized by the user and persisted in the database settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KeyboardShortcuts {
    /// Map of action names to their key combinations.
    pub bindings: HashMap<String, String>,
}

impl Default for KeyboardShortcuts {
    fn default() -> Self {
        let mut bindings = HashMap::new();

        // Navigation
        bindings.insert("next_article".to_string(), "j".to_string());
        bindings.insert("prev_article".to_string(), "k".to_string());
        bindings.insert("open_article".to_string(), "o".to_string());
        bindings.insert("open_article_enter".to_string(), "Enter".to_string());
        bindings.insert("star_article".to_string(), "s".to_string());
        bindings.insert("mark_read".to_string(), "m".to_string());
        bindings.insert("refresh_feeds".to_string(), "r".to_string());
        bindings.insert("search".to_string(), "/".to_string());
        bindings.insert("focus_feeds".to_string(), "g f".to_string());
        bindings.insert("focus_articles".to_string(), "g a".to_string());

        // Article reading
        bindings.insert("scroll_down".to_string(), "Space".to_string());
        bindings.insert("scroll_up".to_string(), "Shift+Space".to_string());
        bindings.insert("close_reader".to_string(), "Escape".to_string());
        bindings.insert("next_chapter".to_string(), "n".to_string());
        bindings.insert("prev_chapter".to_string(), "p".to_string());

        // Global
        bindings.insert("toggle_theme".to_string(), "t".to_string());
        bindings.insert("import_opml".to_string(), "Ctrl+Shift+O".to_string());
        bindings.insert("export_opml".to_string(), "Ctrl+Shift+E".to_string());
        bindings.insert("quit".to_string(), "Ctrl+Q".to_string());

        Self { bindings }
    }
}

impl KeyboardShortcuts {
    /// Create with all default bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get the shortcut for a given action.
    pub fn get(&self, action: &str) -> Option<&str> {
        self.bindings.get(action).map(|s| s.as_str())
    }

    /// Set a custom shortcut for an action.
    pub fn set(&mut self, action: String, shortcut: String) {
        self.bindings.insert(action, shortcut);
    }

    /// Reset a single action to its default.
    pub fn reset_to_default(&mut self, action: &str) {
        let defaults = Self::default();
        if let Some(default) = defaults.bindings.get(action) {
            self.bindings.insert(action.to_string(), default.clone());
        }
    }

    /// Reset all bindings to defaults.
    pub fn reset_all(&mut self) {
        *self = Self::default();
    }

    /// Serialize to JSON string for storage.
    pub fn to_json(&self) -> anyhow::Result<String> {
        Ok(serde_json::to_string(self)?)
    }

    /// Deserialize from JSON string.
    pub fn from_json(json: &str) -> anyhow::Result<Self> {
        Ok(serde_json::from_str(json)?)
    }

    /// Validate that a shortcut string is well-formed.
    ///
    /// Accepts formats like: "j", "Ctrl+O", "Shift+Enter", "Ctrl+Shift+K"
    pub fn validate_shortcut(shortcut: &str) -> bool {
        if shortcut.is_empty() {
            return false;
        }

        // Single character or named key is fine
        if shortcut.len() == 1 || is_named_key(shortcut) {
            return true;
        }

        // Check modifier combinations
        let parts: Vec<&str> = shortcut.split('+').collect();
        if parts.len() > 3 {
            return false;
        }

        let modifiers: Vec<&str> = parts[..parts.len() - 1].to_vec();
        let key = parts.last().unwrap();

        let valid_modifiers = ["Ctrl", "Alt", "Shift", "Cmd", "Meta"];
        for modifier in modifiers {
            if !valid_modifiers.contains(&modifier) {
                return false;
            }
        }

        key.len() == 1 || is_named_key(key)
    }

    /// List all available actions that can be bound.
    pub fn available_actions() -> Vec<&'static str> {
        vec![
            "next_article",
            "prev_article",
            "open_article",
            "open_article_enter",
            "star_article",
            "mark_read",
            "refresh_feeds",
            "search",
            "focus_feeds",
            "focus_articles",
            "scroll_down",
            "scroll_up",
            "close_reader",
            "next_chapter",
            "prev_chapter",
            "toggle_theme",
            "import_opml",
            "export_opml",
            "quit",
        ]
    }
}

fn is_named_key(key: &str) -> bool {
    const NAMED_KEYS: &[&str] = &[
        "Enter",
        "Escape",
        "Space",
        "Tab",
        "Backspace",
        "Delete",
        "Home",
        "End",
        "PageUp",
        "PageDown",
        "ArrowUp",
        "ArrowDown",
        "ArrowLeft",
        "ArrowRight",
        "F1",
        "F2",
        "F3",
        "F4",
        "F5",
        "F6",
        "F7",
        "F8",
        "F9",
        "F10",
        "F11",
        "F12",
    ];
    NAMED_KEYS.contains(&key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_shortcuts() {
        let shortcuts = KeyboardShortcuts::default();
        assert_eq!(shortcuts.get("next_article"), Some("j"));
        assert_eq!(shortcuts.get("prev_article"), Some("k"));
        assert_eq!(shortcuts.get("open_article"), Some("o"));
        assert_eq!(shortcuts.get("star_article"), Some("s"));
        assert_eq!(shortcuts.get("refresh_feeds"), Some("r"));
    }

    #[test]
    fn test_customize_shortcut() {
        let mut shortcuts = KeyboardShortcuts::default();
        shortcuts.set("next_article".to_string(), "n".to_string());
        assert_eq!(shortcuts.get("next_article"), Some("n"));
    }

    #[test]
    fn test_reset_to_default() {
        let mut shortcuts = KeyboardShortcuts::default();
        shortcuts.set("next_article".to_string(), "n".to_string());
        shortcuts.reset_to_default("next_article");
        assert_eq!(shortcuts.get("next_article"), Some("j"));
    }

    #[test]
    fn test_reset_all() {
        let mut shortcuts = KeyboardShortcuts::default();
        shortcuts.set("next_article".to_string(), "n".to_string());
        shortcuts.set("prev_article".to_string(), "p".to_string());
        shortcuts.reset_all();
        assert_eq!(shortcuts.get("next_article"), Some("j"));
        assert_eq!(shortcuts.get("prev_article"), Some("k"));
    }

    #[test]
    fn test_serde_roundtrip() {
        let shortcuts = KeyboardShortcuts::default();
        let json = shortcuts.to_json().unwrap();
        let restored = KeyboardShortcuts::from_json(&json).unwrap();
        assert_eq!(shortcuts, restored);
    }

    #[test]
    fn test_validate_shortcut() {
        assert!(KeyboardShortcuts::validate_shortcut("j"));
        assert!(KeyboardShortcuts::validate_shortcut("Ctrl+O"));
        assert!(KeyboardShortcuts::validate_shortcut("Shift+Enter"));
        assert!(KeyboardShortcuts::validate_shortcut("Ctrl+Shift+K"));
        assert!(KeyboardShortcuts::validate_shortcut("Space"));
        assert!(KeyboardShortcuts::validate_shortcut("F5"));
        assert!(!KeyboardShortcuts::validate_shortcut(""));
        assert!(!KeyboardShortcuts::validate_shortcut("Ctrl+Alt+Shift+K"));
        assert!(!KeyboardShortcuts::validate_shortcut("Invalid+Key"));
    }

    #[test]
    fn test_available_actions() {
        let actions = KeyboardShortcuts::available_actions();
        assert!(actions.contains(&"next_article"));
        assert!(actions.contains(&"star_article"));
        assert!(actions.contains(&"quit"));
    }
}
