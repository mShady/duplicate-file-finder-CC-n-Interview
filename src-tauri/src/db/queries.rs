//! Database query functions

use super::models::{DeletionRecord, ProtectedFolder, ScanSession, ScanStatus, Setting};
use sqlx::SqlitePool;

/// Row type for scan session queries
type ScanSessionRow = (i64, i64, Option<i64>, String, String, i64, i64, i32, i64);

/// Settings-related queries
pub mod settings {
    use super::{Setting, SqlitePool};

    /// Get a setting value by key
    pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
        let result: Option<(String,)> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(pool)
            .await?;

        Ok(result.map(|r| r.0))
    }

    /// Set a setting value
    pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get all settings
    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Setting>, sqlx::Error> {
        let results: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
            .fetch_all(pool)
            .await?;

        Ok(results
            .into_iter()
            .map(|(key, value)| Setting { key, value })
            .collect())
    }
}

/// Protected folder queries
pub mod protected_folders {
    use super::{ProtectedFolder, SqlitePool};

    /// Add a protected folder
    pub async fn add(pool: &SqlitePool, path: &str) -> Result<i64, sqlx::Error> {
        let result = sqlx::query("INSERT INTO protected_folders (path) VALUES (?) RETURNING id")
            .bind(path)
            .fetch_one(pool)
            .await?;

        Ok(sqlx::Row::get(&result, 0))
    }

    /// Remove a protected folder
    pub async fn remove(pool: &SqlitePool, id: i64) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM protected_folders WHERE id = ?")
            .bind(id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    /// Get all protected folders
    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<ProtectedFolder>, sqlx::Error> {
        let results: Vec<(i64, String, i64)> =
            sqlx::query_as("SELECT id, path, added_at FROM protected_folders ORDER BY path")
                .fetch_all(pool)
                .await?;

        Ok(results
            .into_iter()
            .map(|(id, path, added_at)| ProtectedFolder { id, path, added_at })
            .collect())
    }

    /// Check if a path is protected
    ///
    /// Returns true if the given path exactly matches a protected folder
    /// or is a subdirectory of a protected folder.
    pub async fn is_protected(pool: &SqlitePool, path: &str) -> Result<bool, sqlx::Error> {
        // Check if the path exactly matches or starts with a protected folder path followed by '/'
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM protected_folders WHERE ? = path OR ? LIKE path || '/' || '%' LIMIT 1",
        )
        .bind(path)
        .bind(path)
        .fetch_optional(pool)
        .await?;

        Ok(result.is_some())
    }
}

/// Scan session queries
pub mod scan_sessions {
    use super::{ScanSession, ScanSessionRow, ScanStatus, SqlitePool};

    /// Create a new scan session
    pub async fn create(pool: &SqlitePool, paths: &[String]) -> Result<i64, sqlx::Error> {
        let paths_json = serde_json::to_string(paths).unwrap_or_else(|_| "[]".to_string());
        #[allow(clippy::cast_possible_wrap)]
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let result = sqlx::query(
            "INSERT INTO scan_sessions (started_at, status, scanned_paths)
             VALUES (?, 'running', ?) RETURNING id",
        )
        .bind(now)
        .bind(paths_json)
        .fetch_one(pool)
        .await?;

        Ok(sqlx::Row::get(&result, 0))
    }

    /// Update scan session status
    pub async fn update_status(
        pool: &SqlitePool,
        id: i64,
        status: ScanStatus,
    ) -> Result<(), sqlx::Error> {
        #[allow(clippy::cast_possible_wrap)]
        let now = if status == ScanStatus::Completed || status == ScanStatus::Cancelled {
            Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as i64,
            )
        } else {
            None
        };

        sqlx::query(
            "UPDATE scan_sessions SET status = ?, completed_at = COALESCE(?, completed_at) WHERE id = ?"
        )
        .bind(status.as_str())
        .bind(now)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Update scan session statistics
    pub async fn update_stats(
        pool: &SqlitePool,
        id: i64,
        total_files: i64,
        total_size: i64,
        duplicate_groups: i32,
        wasted_space: i64,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "UPDATE scan_sessions
             SET total_files = ?, total_size = ?, duplicate_groups = ?, wasted_space = ?
             WHERE id = ?",
        )
        .bind(total_files)
        .bind(total_size)
        .bind(duplicate_groups)
        .bind(wasted_space)
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get the latest scan session
    pub async fn get_latest(pool: &SqlitePool) -> Result<Option<ScanSession>, sqlx::Error> {
        let result: Option<ScanSessionRow> = sqlx::query_as(
            "SELECT id, started_at, completed_at, status, scanned_paths,
                        total_files, total_size, duplicate_groups, wasted_space
                 FROM scan_sessions
                 ORDER BY started_at DESC
                 LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.map(
            |(
                id,
                started_at,
                completed_at,
                status,
                scanned_paths,
                total_files,
                total_size,
                duplicate_groups,
                wasted_space,
            )| {
                ScanSession {
                    id,
                    started_at,
                    completed_at,
                    status,
                    scanned_paths,
                    total_files,
                    total_size,
                    duplicate_groups,
                    wasted_space,
                }
            },
        ))
    }

    /// Get a paused scan session (for resume)
    pub async fn get_paused(pool: &SqlitePool) -> Result<Option<ScanSession>, sqlx::Error> {
        let result: Option<ScanSessionRow> = sqlx::query_as(
            "SELECT id, started_at, completed_at, status, scanned_paths,
                        total_files, total_size, duplicate_groups, wasted_space
                 FROM scan_sessions
                 WHERE status = 'paused'
                 ORDER BY started_at DESC
                 LIMIT 1",
        )
        .fetch_optional(pool)
        .await?;

        Ok(result.map(
            |(
                id,
                started_at,
                completed_at,
                status,
                scanned_paths,
                total_files,
                total_size,
                duplicate_groups,
                wasted_space,
            )| {
                ScanSession {
                    id,
                    started_at,
                    completed_at,
                    status,
                    scanned_paths,
                    total_files,
                    total_size,
                    duplicate_groups,
                    wasted_space,
                }
            },
        ))
    }
}

/// Deletion history queries
pub mod deletion_history {
    use super::{DeletionRecord, SqlitePool};

    /// Record a deletion
    pub async fn record(
        pool: &SqlitePool,
        file_path: &str,
        file_size: i64,
        file_hash: &str,
        group_id: Option<i64>,
        original_created_at: Option<i64>,
        original_modified_at: Option<i64>,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO deletion_history
             (file_path, file_size, file_hash, group_id, original_created_at, original_modified_at)
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id",
        )
        .bind(file_path)
        .bind(file_size)
        .bind(file_hash)
        .bind(group_id)
        .bind(original_created_at)
        .bind(original_modified_at)
        .fetch_one(pool)
        .await?;

        Ok(sqlx::Row::get(&result, 0))
    }

    /// Get deletion history with pagination
    pub async fn get_history(
        pool: &SqlitePool,
        limit: i32,
        offset: i32,
    ) -> Result<Vec<DeletionRecord>, sqlx::Error> {
        let results: Vec<(i64, String, i64, String, i64, Option<i64>)> = sqlx::query_as(
            "SELECT id, file_path, file_size, file_hash, deleted_at, group_id
             FROM deletion_history
             ORDER BY deleted_at DESC
             LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(results
            .into_iter()
            .map(
                |(id, file_path, file_size, file_hash, deleted_at, group_id)| DeletionRecord {
                    id,
                    file_path,
                    file_size,
                    file_hash,
                    deleted_at,
                    group_id,
                },
            )
            .collect())
    }

    /// Get total deletion count
    pub async fn get_count(pool: &SqlitePool) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM deletion_history")
            .fetch_one(pool)
            .await?;

        Ok(result.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::{tempdir, TempDir};

    // Return TempDir alongside Database to keep the temp directory alive
    async fn setup_test_db() -> (Database, TempDir) {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path).await.unwrap();
        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_settings_get_set() {
        let (db, _dir) = setup_test_db().await;

        // Test setting a value
        settings::set(db.pool(), "test_key", "test_value")
            .await
            .unwrap();

        // Test getting the value
        let value = settings::get(db.pool(), "test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Test getting non-existent key
        let missing = settings::get(db.pool(), "missing_key").await.unwrap();
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_protected_folders() {
        let (db, _dir) = setup_test_db().await;

        // Add a protected folder
        let id = protected_folders::add(db.pool(), "/test/path")
            .await
            .unwrap();
        assert!(id > 0);

        // Check if path is protected
        let is_protected = protected_folders::is_protected(db.pool(), "/test/path/subdir")
            .await
            .unwrap();
        assert!(is_protected);

        // Check non-protected path
        let not_protected = protected_folders::is_protected(db.pool(), "/other/path")
            .await
            .unwrap();
        assert!(!not_protected);

        // Check exact match is also protected
        let exact_match = protected_folders::is_protected(db.pool(), "/test/path")
            .await
            .unwrap();
        assert!(exact_match);

        // Check that a path with a similar prefix but not a subdirectory is NOT protected
        let false_positive = protected_folders::is_protected(db.pool(), "/test/pathological")
            .await
            .unwrap();
        assert!(!false_positive);

        // Remove and verify
        let removed = protected_folders::remove(db.pool(), id).await.unwrap();
        assert!(removed);
    }

    #[tokio::test]
    async fn test_scan_sessions() {
        let (db, _dir) = setup_test_db().await;

        // Create a scan session
        let paths = vec!["/test/path".to_string()];
        let id = scan_sessions::create(db.pool(), &paths).await.unwrap();
        assert!(id > 0);

        // Get latest session
        let session = scan_sessions::get_latest(db.pool()).await.unwrap();
        assert!(session.is_some());
        assert_eq!(session.unwrap().status, "running");

        // Update status
        scan_sessions::update_status(db.pool(), id, ScanStatus::Completed)
            .await
            .unwrap();

        let session = scan_sessions::get_latest(db.pool()).await.unwrap();
        assert_eq!(session.unwrap().status, "completed");
    }
}
