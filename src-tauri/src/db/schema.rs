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
