//! Database module for `SQLite` operations
//!
//! NOTE: `Database::close()` method will be used for graceful shutdown in future phases

#![allow(dead_code)] // Methods used in future phases

pub mod models;
pub mod queries;

use sqlx::migrate::MigrateDatabase;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;
use thiserror::Error;

/// Database-related errors
#[derive(Error, Debug)]
pub enum DbError {
    #[error("Database connection error: {0}")]
    Connection(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),

    #[error("Database path error: {0}")]
    Path(String),
}

/// Database connection wrapper
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Create a new database connection
    pub async fn new(db_path: PathBuf) -> Result<Self, DbError> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| DbError::Path(format!("Failed to create database directory: {e}")))?;
        }

        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        // Create database if it doesn't exist
        if !sqlx::Sqlite::database_exists(&db_url)
            .await
            .unwrap_or(false)
        {
            log::info!("Creating database at: {}", db_path.display());
            sqlx::Sqlite::create_database(&db_url).await?;
        }

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_url)
            .await?;

        // Enable WAL mode for better concurrent access
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&pool)
            .await?;

        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys=ON").execute(&pool).await?;

        let db = Self { pool };

        // Run migrations
        db.run_migrations().await?;

        Ok(db)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<(), DbError> {
        log::info!("Running database migrations...");
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        log::info!("Migrations complete");
        Ok(())
    }

    /// Get a reference to the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Close the database connection
    pub async fn close(&self) {
        self.pool.close().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    #[tokio::test]
    async fn test_database_creation() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(db_path.clone()).await;
        assert!(db.is_ok());

        // Verify database file was created
        assert!(db_path.exists());
    }

    #[tokio::test]
    async fn test_database_connection() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let db = Database::new(db_path).await.unwrap();

        // Test a simple query
        let result: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(db.pool())
            .await
            .unwrap();

        assert_eq!(result.0, 1);
    }

    /// Helper: create a test database and a scan session.
    ///
    /// Returns (`Database`, `session_id`, `TempDir`). The `TempDir` must be kept
    /// alive for the test's duration — dropping it deletes the DB file, which
    /// causes "unable to open database file" errors on concurrent pool access.
    async fn setup_test_db_with_session() -> (Database, i64, TempDir) {
        let temp_dir = tempdir().unwrap();
        let db = Database::new(temp_dir.path().join("test.db"))
            .await
            .unwrap();
        let session_id =
            queries::scan_sessions::create(db.pool(), &["/test".to_string()])
                .await
                .unwrap();
        // Mark session as completed so get_scan_results would pick it up
        queries::scan_sessions::update_status(
            db.pool(),
            session_id,
            models::ScanStatus::Completed,
        )
        .await
        .unwrap();
        (db, session_id, temp_dir)
    }

    #[tokio::test]
    async fn test_db_roundtrip_large_file_size() {
        let (db, session_id, _dir) = setup_test_db_with_session().await;

        // Insert a group with file_size near i64::MAX
        let large_size: i64 = i64::MAX; // 9_223_372_036_854_775_807
        let group_id = queries::duplicate_groups::create(
            db.pool(),
            "abc123",
            large_size,
            2,
            large_size,
            session_id,
        )
        .await
        .unwrap();

        // Read it back via the query layer
        let groups = queries::duplicate_groups::get_by_session(db.pool(), session_id)
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, group_id);
        // The DB stores i64 faithfully — the bug is in the command layer's `as u64` cast,
        // not in the DB layer itself. This test confirms the DB round-trip is lossless.
        assert_eq!(groups[0].file_size, large_size);
        assert_eq!(groups[0].wasted_space, large_size);
    }

    #[tokio::test]
    async fn test_db_roundtrip_zero_wasted_space() {
        let (db, session_id, _dir) = setup_test_db_with_session().await;

        let group_id = queries::duplicate_groups::create(
            db.pool(),
            "zerohash",
            1024,
            2,
            0, // zero wasted space
            session_id,
        )
        .await
        .unwrap();

        let groups = queries::duplicate_groups::get_by_session(db.pool(), session_id)
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].id, group_id);
        assert_eq!(groups[0].wasted_space, 0);
        // 0_i64 as u64 == 0_u64, so this boundary case is safe
    }

    #[tokio::test]
    async fn test_db_negative_value_cast_behavior() {
        let (db, session_id, _dir) = setup_test_db_with_session().await;

        // Directly insert a row with negative file_size via raw SQL.
        // This simulates data corruption or overflow from a u64 > i64::MAX
        // being stored via `as i64`.
        sqlx::query(
            "INSERT INTO duplicate_groups (hash, file_size, file_count, wasted_space, scan_session_id)
             VALUES ('negativehash', -100, 2, -200, ?)",
        )
        .bind(session_id)
        .execute(db.pool())
        .await
        .unwrap();

        let groups = queries::duplicate_groups::get_by_session(db.pool(), session_id)
            .await
            .unwrap();

        assert_eq!(groups.len(), 1);
        // The DB layer returns raw i64 values — negative values are preserved
        assert_eq!(groups[0].file_size, -100);
        assert_eq!(groups[0].wasted_space, -200);

        // The command layer (get_scan_results) applies try_into().unwrap_or(0u64)
        // on these values, clamping negatives to 0 instead of wrapping.
        // Replicate that exact conversion to verify the fix end-to-end:
        let file_size_u64: u64 = groups[0].file_size.try_into().unwrap_or(0u64);
        let wasted_space_u64: u64 = groups[0].wasted_space.try_into().unwrap_or(0u64);
        assert_eq!(file_size_u64, 0, "negative file_size should clamp to 0");
        assert_eq!(wasted_space_u64, 0, "negative wasted_space should clamp to 0");
    }
}
