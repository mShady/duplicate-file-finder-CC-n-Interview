# File 02: Database Foundation

## Overview

This file covers setting up SQLite with the sqlx crate, creating the database schema, and implementing migrations. By the end of this file, you'll have a working database layer for storing scan results, file indexes, settings, and deletion history.

## Prerequisites

- Completed File 01 (Project Foundation)
- Working Tauri project with Rust backend

---

## Phase 2.1: Add SQLite Dependencies

### Overview
Add the sqlx crate and related dependencies to the Rust project for database operations.

### Changes Required

#### 2.1.1 Update Cargo.toml

**File**: `src-tauri/Cargo.toml`

Add the following dependencies:

```toml
[package]
name = "duplifind"
version = "0.1.0"
description = "Cross-platform duplicate file finder"
authors = ["DupliFind Team"]
edition = "2021"
license = "MIT"

[lib]
name = "duplifind_lib"
crate-type = ["staticlib", "cdylib", "lib"]

[[bin]]
name = "duplifind"
path = "src/main.rs"

[build-dependencies]
tauri-build = { version = "2.0", features = [] }

[dependencies]
tauri = { version = "2.0", features = [] }
tauri-plugin-shell = "2.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Database
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite"] }
tokio = { version = "1.0", features = ["rt-multi-thread", "macros"] }

# Async runtime
futures = "0.3"

# Error handling
thiserror = "2.0"

# Logging
log = "0.4"
env_logger = "0.11"

[dev-dependencies]
tempfile = "3.14"

[profile.dev]
incremental = true

[profile.release]
panic = "abort"
codegen-units = 1
lto = true
opt-level = "s"
strip = true
```

#### 2.1.2 Create .env file for sqlx

**File**: `src-tauri/.env`

```
DATABASE_URL=sqlite:duplifind.db
```

Note: This is only used during development for sqlx compile-time checking. The actual database path is determined at runtime.

### Success Criteria

#### Automated Verification
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `ls src-tauri/.env` shows env file exists

#### Manual Verification
- [ ] Dependencies are appropriate versions

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

### Code Review
Run code-review-fix-loop agent on `src-tauri/Cargo.toml`.
---

## Phase 2.2: Create Database Module Structure

### Overview
Create the database module with connection management and initialization logic.

### Changes Required

#### 2.2.1 Create Database Module

**File**: `src-tauri/src/db/mod.rs`

```rust
//! Database module for SQLite operations

pub mod models;
mod schema;

use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::migrate::MigrateDatabase;
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
                .map_err(|e| DbError::Path(format!("Failed to create database directory: {}", e)))?;
        }

        let db_url = format!("sqlite:{}?mode=rwc", db_path.display());

        // Create database if it doesn't exist
        if !sqlx::Sqlite::database_exists(&db_url).await.unwrap_or(false) {
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
        sqlx::query("PRAGMA foreign_keys=ON")
            .execute(&pool)
            .await?;

        let db = Self { pool };

        // Run migrations
        db.run_migrations().await?;

        Ok(db)
    }

    /// Run database migrations
    async fn run_migrations(&self) -> Result<(), DbError> {
        log::info!("Running database migrations...");
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await?;
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
    use tempfile::tempdir;

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
}
```

#### 2.2.2 Create Models Module

**File**: `src-tauri/src/db/models.rs`

```rust
//! Database models representing table structures

use serde::{Deserialize, Serialize};

/// Represents a scanned file in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedFile {
    pub id: i64,
    pub path: String,
    pub size: i64,
    pub partial_hash: Option<String>,
    pub full_hash: Option<String>,
    pub created_at: i64,
    pub modified_at: i64,
    pub scanned_at: i64,
    pub group_id: Option<i64>,
}

/// Represents a group of duplicate files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub id: i64,
    pub hash: String,
    pub file_size: i64,
    pub file_count: i32,
    pub wasted_space: i64,
    pub created_at: i64,
}

/// Represents a scan session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession {
    pub id: i64,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub status: String,
    pub scanned_paths: String,
    pub total_files: i64,
    pub total_size: i64,
    pub duplicate_groups: i32,
    pub wasted_space: i64,
}

/// Scan session status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

impl ScanStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScanStatus::Running => "running",
            ScanStatus::Paused => "paused",
            ScanStatus::Completed => "completed",
            ScanStatus::Cancelled => "cancelled",
            ScanStatus::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "running" => Some(ScanStatus::Running),
            "paused" => Some(ScanStatus::Paused),
            "completed" => Some(ScanStatus::Completed),
            "cancelled" => Some(ScanStatus::Cancelled),
            "failed" => Some(ScanStatus::Failed),
            _ => None,
        }
    }
}

/// Represents a user setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

/// Represents a protected folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedFolder {
    pub id: i64,
    pub path: String,
    pub added_at: i64,
}

/// Represents a deletion record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionRecord {
    pub id: i64,
    pub file_path: String,
    pub file_size: i64,
    pub file_hash: String,
    pub deleted_at: i64,
    pub group_id: i64,
}

/// Represents a file in the hash cache (for incremental scanning)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCache {
    pub id: i64,
    pub path: String,
    pub size: i64,
    pub modified_at: i64,
    pub partial_hash: String,
    pub full_hash: Option<String>,
    pub cached_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_status_conversion() {
        assert_eq!(ScanStatus::Running.as_str(), "running");
        assert_eq!(ScanStatus::from_str("running"), Some(ScanStatus::Running));
        assert_eq!(ScanStatus::from_str("invalid"), None);
    }
}
```

#### 2.2.3 Create Schema Module (placeholder)

**File**: `src-tauri/src/db/schema.rs`

```rust
//! Database schema constants and helpers
//!
//! Note: The actual schema is defined in migrations.
//! This module provides schema-related constants and helpers.

/// Current schema version
pub const SCHEMA_VERSION: i32 = 1;

/// Table names
pub mod tables {
    pub const SCANNED_FILES: &str = "scanned_files";
    pub const DUPLICATE_GROUPS: &str = "duplicate_groups";
    pub const SCAN_SESSIONS: &str = "scan_sessions";
    pub const SETTINGS: &str = "settings";
    pub const PROTECTED_FOLDERS: &str = "protected_folders";
    pub const DELETION_HISTORY: &str = "deletion_history";
    pub const FILE_CACHE: &str = "file_cache";
}
```

#### 2.2.4 Update lib.rs to include db module

**File**: `src-tauri/src/lib.rs`

```rust
// DupliFind - Main library entry point

mod commands;
mod db;
mod state;

use state::AppState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize application state
            let state = AppState::new();
            app.manage(Mutex::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `ls src-tauri/src/db/mod.rs src-tauri/src/db/models.rs src-tauri/src/db/schema.rs` shows all files

#### Manual Verification
- [ ] Module structure is clean and follows Rust conventions

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

### Code Review
Run code-review-fix-loop agent on all new Rust files in db module.
---

## Phase 2.3: Create Database Migrations

### Overview
Create SQL migrations for the database schema.

### Changes Required

#### 2.3.1 Create Migrations Directory

Create the migrations directory structure.

#### 2.3.2 Create Initial Migration

**File**: `src-tauri/migrations/20240101000000_initial_schema.sql`

```sql
-- Initial database schema for DupliFind

-- Scan sessions track individual scan operations
CREATE TABLE IF NOT EXISTS scan_sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL DEFAULT 'running',
    scanned_paths TEXT NOT NULL DEFAULT '[]',
    total_files INTEGER NOT NULL DEFAULT 0,
    total_size INTEGER NOT NULL DEFAULT 0,
    duplicate_groups INTEGER NOT NULL DEFAULT 0,
    wasted_space INTEGER NOT NULL DEFAULT 0
);

-- Duplicate groups contain files with identical content
CREATE TABLE IF NOT EXISTS duplicate_groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    hash TEXT NOT NULL UNIQUE,
    file_size INTEGER NOT NULL,
    file_count INTEGER NOT NULL DEFAULT 0,
    wasted_space INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    scan_session_id INTEGER,
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- Scanned files with their metadata and hashes
CREATE TABLE IF NOT EXISTS scanned_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    size INTEGER NOT NULL,
    partial_hash TEXT,
    full_hash TEXT,
    created_at INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    scanned_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    group_id INTEGER,
    scan_session_id INTEGER,
    FOREIGN KEY (group_id) REFERENCES duplicate_groups(id) ON DELETE SET NULL,
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_scanned_files_size ON scanned_files(size);
CREATE INDEX IF NOT EXISTS idx_scanned_files_partial_hash ON scanned_files(partial_hash);
CREATE INDEX IF NOT EXISTS idx_scanned_files_full_hash ON scanned_files(full_hash);
CREATE INDEX IF NOT EXISTS idx_scanned_files_group_id ON scanned_files(group_id);
CREATE INDEX IF NOT EXISTS idx_duplicate_groups_hash ON duplicate_groups(hash);
CREATE INDEX IF NOT EXISTS idx_duplicate_groups_wasted_space ON duplicate_groups(wasted_space DESC);

-- User settings (key-value store)
CREATE TABLE IF NOT EXISTS settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Protected folders that cannot be deleted from
CREATE TABLE IF NOT EXISTS protected_folders (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    added_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Deletion history for audit and undo information
CREATE TABLE IF NOT EXISTS deletion_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,
    file_size INTEGER NOT NULL,
    file_hash TEXT NOT NULL,
    deleted_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    group_id INTEGER,
    original_created_at INTEGER,
    original_modified_at INTEGER
);

CREATE INDEX IF NOT EXISTS idx_deletion_history_deleted_at ON deletion_history(deleted_at DESC);

-- File cache for incremental scanning
CREATE TABLE IF NOT EXISTS file_cache (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    path TEXT NOT NULL UNIQUE,
    size INTEGER NOT NULL,
    modified_at INTEGER NOT NULL,
    partial_hash TEXT NOT NULL,
    full_hash TEXT,
    cached_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_file_cache_path ON file_cache(path);
CREATE INDEX IF NOT EXISTS idx_file_cache_size_mtime ON file_cache(size, modified_at);

-- Scan progress tracking (for pause/resume)
CREATE TABLE IF NOT EXISTS scan_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    scan_session_id INTEGER NOT NULL UNIQUE,
    current_path TEXT,
    pending_paths TEXT NOT NULL DEFAULT '[]',
    processed_count INTEGER NOT NULL DEFAULT 0,
    skipped_count INTEGER NOT NULL DEFAULT 0,
    error_count INTEGER NOT NULL DEFAULT 0,
    skipped_files TEXT NOT NULL DEFAULT '[]',
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    FOREIGN KEY (scan_session_id) REFERENCES scan_sessions(id) ON DELETE CASCADE
);

-- Insert default settings
INSERT OR IGNORE INTO settings (key, value) VALUES
    ('theme', 'system'),
    ('parallelism', 'normal'),
    ('last_scan_paths', '[]'),
    ('window_width', '1200'),
    ('window_height', '800');
```

### Success Criteria

#### Automated Verification
- [ ] `ls src-tauri/migrations/` shows migration file exists
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes

#### Manual Verification
- [ ] Schema covers all required tables from specification
- [ ] Indexes are appropriate for expected query patterns

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

### Code Review
Run code-review-fix-loop agent on migration SQL.
---

## Phase 2.4: Implement Database Queries

### Overview
Implement the query layer for common database operations.

### Changes Required

#### 2.4.1 Create Queries Module

**File**: `src-tauri/src/db/queries.rs`

```rust
//! Database query functions

use super::models::*;
use sqlx::SqlitePool;

/// Settings-related queries
pub mod settings {
    use super::*;

    /// Get a setting value by key
    pub async fn get(pool: &SqlitePool, key: &str) -> Result<Option<String>, sqlx::Error> {
        let result: Option<(String,)> = sqlx::query_as(
            "SELECT value FROM settings WHERE key = ?"
        )
        .bind(key)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|r| r.0))
    }

    /// Set a setting value
    pub async fn set(pool: &SqlitePool, key: &str, value: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO settings (key, value) VALUES (?, ?)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value"
        )
        .bind(key)
        .bind(value)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Get all settings
    pub async fn get_all(pool: &SqlitePool) -> Result<Vec<Setting>, sqlx::Error> {
        let results: Vec<(String, String)> = sqlx::query_as(
            "SELECT key, value FROM settings"
        )
        .fetch_all(pool)
        .await?;

        Ok(results.into_iter().map(|(key, value)| Setting { key, value }).collect())
    }
}

/// Protected folder queries
pub mod protected_folders {
    use super::*;

    /// Add a protected folder
    pub async fn add(pool: &SqlitePool, path: &str) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO protected_folders (path) VALUES (?) RETURNING id"
        )
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
        let results: Vec<(i64, String, i64)> = sqlx::query_as(
            "SELECT id, path, added_at FROM protected_folders ORDER BY path"
        )
        .fetch_all(pool)
        .await?;

        Ok(results.into_iter().map(|(id, path, added_at)| {
            ProtectedFolder { id, path, added_at }
        }).collect())
    }

    /// Check if a path is protected
    pub async fn is_protected(pool: &SqlitePool, path: &str) -> Result<bool, sqlx::Error> {
        // Check if the path or any of its parents is protected
        let result: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM protected_folders WHERE ? LIKE path || '%' LIMIT 1"
        )
        .bind(path)
        .fetch_optional(pool)
        .await?;

        Ok(result.is_some())
    }
}

/// Scan session queries
pub mod scan_sessions {
    use super::*;

    /// Create a new scan session
    pub async fn create(
        pool: &SqlitePool,
        paths: &[String],
    ) -> Result<i64, sqlx::Error> {
        let paths_json = serde_json::to_string(paths).unwrap_or_else(|_| "[]".to_string());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        let result = sqlx::query(
            "INSERT INTO scan_sessions (started_at, status, scanned_paths)
             VALUES (?, 'running', ?) RETURNING id"
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
        let now = if status == ScanStatus::Completed || status == ScanStatus::Cancelled {
            Some(std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64)
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
             WHERE id = ?"
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
        let result: Option<(i64, i64, Option<i64>, String, String, i64, i64, i32, i64)> =
            sqlx::query_as(
                "SELECT id, started_at, completed_at, status, scanned_paths,
                        total_files, total_size, duplicate_groups, wasted_space
                 FROM scan_sessions
                 ORDER BY started_at DESC
                 LIMIT 1"
            )
            .fetch_optional(pool)
            .await?;

        Ok(result.map(|(id, started_at, completed_at, status, scanned_paths,
                        total_files, total_size, duplicate_groups, wasted_space)| {
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
        }))
    }

    /// Get a paused scan session (for resume)
    pub async fn get_paused(pool: &SqlitePool) -> Result<Option<ScanSession>, sqlx::Error> {
        let result: Option<(i64, i64, Option<i64>, String, String, i64, i64, i32, i64)> =
            sqlx::query_as(
                "SELECT id, started_at, completed_at, status, scanned_paths,
                        total_files, total_size, duplicate_groups, wasted_space
                 FROM scan_sessions
                 WHERE status = 'paused'
                 ORDER BY started_at DESC
                 LIMIT 1"
            )
            .fetch_optional(pool)
            .await?;

        Ok(result.map(|(id, started_at, completed_at, status, scanned_paths,
                        total_files, total_size, duplicate_groups, wasted_space)| {
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
        }))
    }
}

/// Deletion history queries
pub mod deletion_history {
    use super::*;

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
             VALUES (?, ?, ?, ?, ?, ?) RETURNING id"
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
             LIMIT ? OFFSET ?"
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(results.into_iter().map(|(id, file_path, file_size, file_hash, deleted_at, group_id)| {
            DeletionRecord {
                id,
                file_path,
                file_size,
                file_hash,
                deleted_at,
                group_id: group_id.unwrap_or(0),
            }
        }).collect())
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
    use tempfile::tempdir;

    async fn setup_test_db() -> Database {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        Database::new(db_path).await.unwrap()
    }

    #[tokio::test]
    async fn test_settings_get_set() {
        let db = setup_test_db().await;

        // Test setting a value
        settings::set(db.pool(), "test_key", "test_value").await.unwrap();

        // Test getting the value
        let value = settings::get(db.pool(), "test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Test getting non-existent key
        let missing = settings::get(db.pool(), "missing_key").await.unwrap();
        assert_eq!(missing, None);
    }

    #[tokio::test]
    async fn test_protected_folders() {
        let db = setup_test_db().await;

        // Add a protected folder
        let id = protected_folders::add(db.pool(), "/test/path").await.unwrap();
        assert!(id > 0);

        // Check if path is protected
        let is_protected = protected_folders::is_protected(db.pool(), "/test/path/subdir").await.unwrap();
        assert!(is_protected);

        // Check non-protected path
        let not_protected = protected_folders::is_protected(db.pool(), "/other/path").await.unwrap();
        assert!(!not_protected);

        // Remove and verify
        let removed = protected_folders::remove(db.pool(), id).await.unwrap();
        assert!(removed);
    }

    #[tokio::test]
    async fn test_scan_sessions() {
        let db = setup_test_db().await;

        // Create a scan session
        let paths = vec!["/test/path".to_string()];
        let id = scan_sessions::create(db.pool(), &paths).await.unwrap();
        assert!(id > 0);

        // Get latest session
        let session = scan_sessions::get_latest(db.pool()).await.unwrap();
        assert!(session.is_some());
        assert_eq!(session.unwrap().status, "running");

        // Update status
        scan_sessions::update_status(db.pool(), id, ScanStatus::Completed).await.unwrap();

        let session = scan_sessions::get_latest(db.pool()).await.unwrap();
        assert_eq!(session.unwrap().status, "completed");
    }
}
```

#### 2.4.2 Update db/mod.rs to export queries

**File**: `src-tauri/src/db/mod.rs`

Add to the top of the file:

```rust
//! Database module for SQLite operations

pub mod models;
pub mod queries;
mod schema;

// ... rest of the file remains the same
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes (all tests including new query tests)

#### Manual Verification
- [ ] Query functions cover the main use cases
- [ ] Tests validate core functionality

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

### Code Review
Run code-review-fix-loop agent on `src-tauri/src/db/queries.rs`.
---

## Phase 2.5: Integrate Database with Application State

### Overview
Connect the database to the application state and make it available throughout the app.

### Changes Required

#### 2.5.1 Update State Module

**File**: `src-tauri/src/state.rs`

```rust
//! Application state management

use crate::db::Database;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

/// Global application state
pub struct AppState {
    /// Database connection
    pub db: Option<Arc<AsyncMutex<Database>>>,
    /// Flag indicating if a scan is currently running
    pub is_scanning: bool,
    /// Current scan session ID
    pub current_scan_id: Option<i64>,
}

impl AppState {
    /// Create a new application state (without database)
    pub fn new() -> Self {
        Self {
            db: None,
            is_scanning: false,
            current_scan_id: None,
        }
    }

    /// Initialize the database connection
    pub async fn init_database(&mut self, app_data_dir: PathBuf) -> Result<(), crate::db::DbError> {
        let db_path = app_data_dir.join("duplifind.db");
        log::info!("Initializing database at: {}", db_path.display());

        let db = Database::new(db_path).await?;
        self.db = Some(Arc::new(AsyncMutex::new(db)));

        Ok(())
    }

    /// Get a reference to the database
    pub fn database(&self) -> Option<Arc<AsyncMutex<Database>>> {
        self.db.clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert!(!state.is_scanning);
        assert!(state.db.is_none());
        assert!(state.current_scan_id.is_none());
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.is_scanning);
    }

    #[tokio::test]
    async fn test_init_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();

        let result = state.init_database(temp_dir.path().to_path_buf()).await;
        assert!(result.is_ok());
        assert!(state.db.is_some());
    }
}
```

#### 2.5.2 Update lib.rs for Database Initialization

**File**: `src-tauri/src/lib.rs`

```rust
// DupliFind - Main library entry point

mod commands;
mod db;
mod state;

use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Get app data directory
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");

            log::info!("App data directory: {}", app_data_dir.display());

            // Initialize application state
            let mut state = AppState::new();

            // Initialize database asynchronously
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut state_guard = app_handle.state::<Mutex<AppState>>();
                let mut state = state_guard.lock().unwrap();

                if let Err(e) = state.init_database(app_data_dir).await {
                    log::error!("Failed to initialize database: {}", e);
                }
            });

            app.manage(Mutex::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Note: The database initialization approach above has a race condition issue. Let's fix it:

**File**: `src-tauri/src/lib.rs` (corrected version)

```rust
// DupliFind - Main library entry point

mod commands;
mod db;
mod state;

use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Get app data directory
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");

            log::info!("App data directory: {}", app_data_dir.display());

            // Initialize application state with database
            // We use block_on here since setup is synchronous
            let state = tauri::async_runtime::block_on(async {
                let mut state = AppState::new();
                if let Err(e) = state.init_database(app_data_dir).await {
                    log::error!("Failed to initialize database: {}", e);
                }
                state
            });

            app.manage(Mutex::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes

#### Manual Verification
- [ ] `npm run tauri dev` starts without errors
- [ ] Database file is created in app data directory

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

### Code Review
Run code-review-fix-loop agent on `src-tauri/src/state.rs` and `src-tauri/src/lib.rs`.
---

## Phase 2.6: Add Database Commands for Frontend

### Overview
Create Tauri commands to expose database operations to the frontend.

### Changes Required

#### 2.6.1 Create Settings Commands

**File**: `src-tauri/src/commands/settings.rs`

```rust
//! Settings-related Tauri commands

use crate::db::queries;
use crate::state::AppState;
use std::sync::Mutex;
use tauri::State;

/// Get a setting value
#[tauri::command]
pub async fn get_setting(
    key: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<String>, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::settings::get(db.pool(), &key)
        .await
        .map_err(|e| e.to_string())
}

/// Set a setting value
#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::settings::set(db.pool(), &key, &value)
        .await
        .map_err(|e| e.to_string())
}

/// Get all settings
#[tauri::command]
pub async fn get_all_settings(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<crate::db::models::Setting>, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::settings::get_all(db.pool())
        .await
        .map_err(|e| e.to_string())
}
```

#### 2.6.2 Create Protected Folders Commands

**File**: `src-tauri/src/commands/protected.rs`

```rust
//! Protected folders Tauri commands

use crate::db::models::ProtectedFolder;
use crate::db::queries;
use crate::state::AppState;
use std::sync::Mutex;
use tauri::State;

/// Add a protected folder
#[tauri::command]
pub async fn add_protected_folder(
    path: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<i64, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::protected_folders::add(db.pool(), &path)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a protected folder
#[tauri::command]
pub async fn remove_protected_folder(
    id: i64,
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::protected_folders::remove(db.pool(), id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all protected folders
#[tauri::command]
pub async fn get_protected_folders(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<ProtectedFolder>, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::protected_folders::get_all(db.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Check if a path is protected
#[tauri::command]
pub async fn is_path_protected(
    path: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::protected_folders::is_protected(db.pool(), &path)
        .await
        .map_err(|e| e.to_string())
}
```

#### 2.6.3 Update Commands Module

**File**: `src-tauri/src/commands/mod.rs`

```rust
//! Tauri command handlers

pub mod protected;
pub mod settings;

/// Simple greet command for testing
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to DupliFind.", name)
}

// Re-export command functions for convenience
pub use protected::*;
pub use settings::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let result = greet("World");
        assert_eq!(result, "Hello, World! Welcome to DupliFind.");
    }

    #[test]
    fn test_greet_empty() {
        let result = greet("");
        assert_eq!(result, "Hello, ! Welcome to DupliFind.");
    }
}
```

#### 2.6.4 Update lib.rs to Register Commands

**File**: `src-tauri/src/lib.rs`

```rust
// DupliFind - Main library entry point

mod commands;
mod db;
mod state;

use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Get app data directory
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");

            log::info!("App data directory: {}", app_data_dir.display());

            // Initialize application state with database
            let state = tauri::async_runtime::block_on(async {
                let mut state = AppState::new();
                if let Err(e) = state.init_database(app_data_dir).await {
                    log::error!("Failed to initialize database: {}", e);
                }
                state
            });

            app.manage(Mutex::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            // Settings commands
            commands::get_setting,
            commands::set_setting,
            commands::get_all_settings,
            // Protected folders commands
            commands::add_protected_folder,
            commands::remove_protected_folder,
            commands::get_protected_folders,
            commands::is_path_protected,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml` shows no warnings

#### Manual Verification
- [ ] `npm run tauri dev` starts without errors
- [ ] Commands are callable from the frontend (test via browser console)

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

### Code Review
Run code-review-fix-loop agent on all command files.
---

## End of File 02

After completing all phases in this file, you should have:

1. SQLite database with sqlx integration
2. Complete database schema with all required tables
3. Query functions for settings, protected folders, scan sessions, and deletion history
4. Database integrated with application state
5. Tauri commands for frontend access to settings and protected folders

**Next**: Proceed to [03-file-scanning.md](./03-file-scanning.md) to implement the file scanning core functionality.
