use crate::db::LibraryDb;
use crate::keyboard::KeyboardShortcuts;
use crate::notifications::NotificationSettings;

/// Manages application settings stored in the database.
pub struct SettingsManager<'a> {
    db: &'a LibraryDb,
}

impl<'a> SettingsManager<'a> {
    pub fn new(db: &'a LibraryDb) -> Self {
        Self { db }
    }

    /// Get keyboard shortcuts, returning defaults if not set.
    pub async fn get_keyboard_shortcuts(&self) -> anyhow::Result<KeyboardShortcuts> {
        match self.db.get_setting("keyboard_shortcuts").await? {
            Some(json) => KeyboardShortcuts::from_json(&json),
            None => Ok(KeyboardShortcuts::default()),
        }
    }

    /// Save keyboard shortcuts.
    pub async fn set_keyboard_shortcuts(
        &self,
        shortcuts: &KeyboardShortcuts,
    ) -> anyhow::Result<()> {
        let json = shortcuts.to_json()?;
        self.db.set_setting("keyboard_shortcuts", &json).await?;
        Ok(())
    }

    /// Get notification settings, returning defaults if not set.
    pub async fn get_notification_settings(&self) -> anyhow::Result<NotificationSettings> {
        match self.db.get_setting("notification_settings").await? {
            Some(json) => NotificationSettings::from_json(&json),
            None => Ok(NotificationSettings::default()),
        }
    }

    /// Save notification settings.
    pub async fn set_notification_settings(
        &self,
        settings: &NotificationSettings,
    ) -> anyhow::Result<()> {
        let json = settings.to_json()?;
        self.db.set_setting("notification_settings", &json).await?;
        Ok(())
    }

    /// Check if this is the first run of the application.
    pub async fn is_first_run(&self) -> anyhow::Result<bool> {
        match self.db.get_setting("first_run_complete").await? {
            Some(value) => Ok(value != "true"),
            None => Ok(true),
        }
    }

    /// Mark the first run as complete.
    pub async fn set_first_run_complete(&self) -> anyhow::Result<()> {
        self.db.set_setting("first_run_complete", "true").await?;
        Ok(())
    }

    /// Get the last version that was run (for migration checks).
    pub async fn get_last_version(&self) -> anyhow::Result<Option<String>> {
        self.db.get_setting("last_version").await
    }

    /// Set the last run version.
    pub async fn set_last_version(&self, version: &str) -> anyhow::Result<()> {
        self.db.set_setting("last_version", version).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_db() -> LibraryDb {
        use sqlx::sqlite::SqlitePoolOptions;
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect("sqlite::memory:")
            .await
            .unwrap();
        LibraryDb::init_schema(&pool).await.unwrap();
        LibraryDb::from_pool(pool)
    }

    #[tokio::test]
    async fn test_keyboard_shortcuts_roundtrip() {
        let db = create_test_db().await;
        let manager = SettingsManager::new(&db);

        let shortcuts = KeyboardShortcuts::default();
        manager.set_keyboard_shortcuts(&shortcuts).await.unwrap();

        let retrieved = manager.get_keyboard_shortcuts().await.unwrap();
        assert_eq!(shortcuts, retrieved);
    }

    #[tokio::test]
    async fn test_notification_settings_roundtrip() {
        let db = create_test_db().await;
        let manager = SettingsManager::new(&db);

        let settings = NotificationSettings::default();
        manager.set_notification_settings(&settings).await.unwrap();

        let retrieved = manager.get_notification_settings().await.unwrap();
        assert_eq!(settings, retrieved);
    }

    #[tokio::test]
    async fn test_first_run() {
        let db = create_test_db().await;
        let manager = SettingsManager::new(&db);

        assert!(manager.is_first_run().await.unwrap());
        manager.set_first_run_complete().await.unwrap();
        assert!(!manager.is_first_run().await.unwrap());
    }

    #[tokio::test]
    async fn test_version_tracking() {
        let db = create_test_db().await;
        let manager = SettingsManager::new(&db);

        assert!(manager.get_last_version().await.unwrap().is_none());
        manager.set_last_version("0.7.0").await.unwrap();
        assert_eq!(manager.get_last_version().await.unwrap(), Some("0.7.0".to_string()));
    }
}
