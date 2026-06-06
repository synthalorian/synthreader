use std::path::{Path, PathBuf};
use tokio::fs;

/// Result of a database backup operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BackupResult {
    pub success: bool,
    pub path: String,
    pub size_bytes: u64,
    pub timestamp: String,
}

/// Result of a database restore operation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RestoreResult {
    pub success: bool,
    pub message: String,
}

/// Create a timestamped backup of the database.
///
/// The backup is created by copying the SQLite database file to a
/// `.backup` subdirectory with a timestamped filename.
pub async fn backup_database(db_path: &Path) -> anyhow::Result<BackupResult> {
    let db_path = db_path.to_path_buf();

    // Verify source exists
    if !db_path.exists() {
        anyhow::bail!("Database file does not exist: {}", db_path.display());
    }

    // Create backup directory
    let backup_dir = db_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("backups");
    fs::create_dir_all(&backup_dir).await?;

    // Generate timestamped filename with millisecond precision to avoid collisions
    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S_%3f");
    let db_name = db_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("library");
    let backup_filename = format!("{}_{}.db", db_name, timestamp);
    let backup_path = backup_dir.join(&backup_filename);

    // Copy database file
    fs::copy(&db_path, &backup_path).await?;

    // Get file size
    let metadata = fs::metadata(&backup_path).await?;
    let size_bytes = metadata.len();

    Ok(BackupResult {
        success: true,
        path: backup_path.to_string_lossy().to_string(),
        size_bytes,
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}

/// Restore the database from a backup file.
///
/// This replaces the current database with the backup. The current
/// database is first copied to a `.pre-restore` backup for safety.
pub async fn restore_database(db_path: &Path, backup_path: &Path) -> anyhow::Result<RestoreResult> {
    // Verify backup exists
    if !backup_path.exists() {
        anyhow::bail!("Backup file does not exist: {}", backup_path.display());
    }

    // Verify it's a valid SQLite file
    let header = fs::read_to_string(backup_path).await?;
    if !header.starts_with("SQLite format 3") {
        anyhow::bail!("Backup file is not a valid SQLite database");
    }

    let db_path = db_path.to_path_buf();

    // Create safety backup of current database
    if db_path.exists() {
        let safety_path = db_path.with_extension("db.pre-restore");
        fs::copy(&db_path, &safety_path).await?;
    }

    // Replace current database with backup
    fs::copy(backup_path, &db_path).await?;

    Ok(RestoreResult {
        success: true,
        message: format!("Database restored from {}", backup_path.display()),
    })
}

/// List all available database backups.
pub async fn list_backups(db_path: &Path) -> anyhow::Result<Vec<BackupResult>> {
    let backup_dir = db_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("backups");

    if !backup_dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    let mut entries = fs::read_dir(&backup_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("db") {
            let metadata = fs::metadata(&path).await?;
            let size_bytes = metadata.len();
            let modified = metadata.modified()?;
            let timestamp = chrono::DateTime::<chrono::Utc>::from(modified).to_rfc3339();

            backups.push(BackupResult {
                success: true,
                path: path.to_string_lossy().to_string(),
                size_bytes,
                timestamp,
            });
        }
    }

    // Sort by timestamp (newest first)
    backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(backups)
}

/// Delete a specific backup file.
pub async fn delete_backup(backup_path: &Path) -> anyhow::Result<()> {
    if !backup_path.exists() {
        anyhow::bail!("Backup file does not exist");
    }
    fs::remove_file(backup_path).await?;
    Ok(())
}

/// Clean up old backups, keeping only the most recent N.
pub async fn cleanup_old_backups(db_path: &Path, keep_count: usize) -> anyhow::Result<usize> {
    let mut backups = list_backups(db_path).await?;

    if backups.len() <= keep_count {
        return Ok(0);
    }

    // Remove oldest backups
    let to_remove = backups.split_off(keep_count);
    let mut removed = 0;

    for backup in to_remove {
        let path = PathBuf::from(&backup.path);
        if fs::remove_file(&path).await.is_ok() {
            removed += 1;
        }
    }

    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_db(dir: &TempDir) -> PathBuf {
        let db_path = dir.path().join("test.db");
        // Create a minimal valid SQLite file
        fs::write(&db_path, b"SQLite format 3\0")
            .await
            .unwrap();
        // Pad to make it look more like a real database
        let mut content = fs::read(&db_path).await.unwrap();
        content.resize(4096, 0);
        fs::write(&db_path, content).await.unwrap();
        db_path
    }

    #[tokio::test]
    async fn test_backup_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_db(&temp_dir).await;

        let result = backup_database(&db_path).await.unwrap();
        assert!(result.success);
        assert!(result.size_bytes > 0);
        assert!(PathBuf::from(&result.path).exists());
    }

    #[tokio::test]
    async fn test_list_backups() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_db(&temp_dir).await;

        // Create two backups
        backup_database(&db_path).await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        backup_database(&db_path).await.unwrap();

        let backups = list_backups(&db_path).await.unwrap();
        assert_eq!(backups.len(), 2);
    }

    #[tokio::test]
    async fn test_restore_database() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_db(&temp_dir).await;

        let backup = backup_database(&db_path).await.unwrap();
        let backup_path = PathBuf::from(&backup.path);

        let result = restore_database(&db_path, &backup_path).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_cleanup_old_backups() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_db(&temp_dir).await;

        // Create 5 backups
        for _ in 0..5 {
            backup_database(&db_path).await.unwrap();
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        let removed = cleanup_old_backups(&db_path, 2).await.unwrap();
        assert_eq!(removed, 3);

        let remaining = list_backups(&db_path).await.unwrap();
        assert_eq!(remaining.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_backup() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = create_test_db(&temp_dir).await;

        let backup = backup_database(&db_path).await.unwrap();
        let backup_path = PathBuf::from(&backup.path);

        delete_backup(&backup_path).await.unwrap();
        assert!(!backup_path.exists());
    }
}
