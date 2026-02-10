//! Scanner type definitions
//!
//! NOTE: Some types and methods are used in tests or future phases:
//! - `ScanResult`, `walk()`, `walk_with_callback()`: Used in scanner tests
//! - `thread_count()`: Will be used when parallel hashing is implemented
//! - Various error variants: Reserved for comprehensive error handling

#![allow(dead_code)] // Some items used only in tests or future phases

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during scanning
#[derive(Error, Debug)]
pub enum ScanError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Path error: {0}")]
    Path(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Scan cancelled")]
    Cancelled,

    #[error("Database error: {0}")]
    Database(String),
}

/// File metadata collected during scanning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    /// Absolute path to the file
    pub path: PathBuf,
    /// File size in bytes
    pub size: u64,
    /// File creation timestamp (Unix epoch seconds)
    pub created_at: i64,
    /// File modification timestamp (Unix epoch seconds)
    pub modified_at: i64,
    /// Whether this is a symbolic link
    pub is_symlink: bool,
}

impl FileInfo {
    /// Create `FileInfo` from a path
    #[allow(clippy::cast_possible_wrap)]
    pub fn from_path(path: PathBuf) -> Result<Self, ScanError> {
        let metadata = std::fs::symlink_metadata(&path)?;
        let is_symlink = metadata.file_type().is_symlink();

        // For symlinks, follow them to get actual file size; otherwise use metadata directly
        let file_metadata = if is_symlink {
            std::fs::metadata(&path)?
        } else {
            metadata
        };

        let created_at = file_metadata
            .created()
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
            })
            .unwrap_or(0);

        let modified_at = file_metadata
            .modified()
            .map(|t| {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
            })
            .unwrap_or(0);

        Ok(Self {
            path,
            size: file_metadata.len(),
            created_at,
            modified_at,
            is_symlink,
        })
    }
}

/// Scan progress information
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScanProgress {
    /// Total files discovered so far
    pub total_files: u64,
    /// Files processed (metadata collected)
    pub processed_files: u64,
    /// Total bytes scanned
    pub total_bytes: u64,
    /// Current file/directory being processed
    pub current_path: Option<String>,
    /// Number of files skipped due to errors
    pub skipped_files: u64,
    /// Estimated total files (if known)
    pub estimated_total: Option<u64>,
}

/// Scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Paths to scan
    pub paths: Vec<PathBuf>,
    /// Whether to follow symbolic links
    pub follow_symlinks: bool,
    /// Maximum depth to scan (None for unlimited)
    pub max_depth: Option<usize>,
    /// Parallelism mode
    pub parallelism: ParallelismMode,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            paths: Vec::new(),
            follow_symlinks: false, // Per spec: symlinks are skipped
            max_depth: None,
            parallelism: ParallelismMode::Normal,
        }
    }
}

/// Parallelism modes for scanning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParallelismMode {
    /// Light mode: 1-2 threads
    Light,
    /// Normal mode: ~75% of available cores (default)
    Normal,
    /// Aggressive mode: all cores
    Aggressive,
}

impl ParallelismMode {
    /// Get the number of threads for this mode
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    pub fn thread_count(self) -> usize {
        let cpus = num_cpus::get();
        match self {
            ParallelismMode::Light => cpus.clamp(1, 2),
            ParallelismMode::Normal => ((cpus as f64 * 0.75).ceil() as usize).max(1),
            ParallelismMode::Aggressive => cpus,
        }
    }
}

/// Result of a scan operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// All discovered files
    pub files: Vec<FileInfo>,
    /// Scan statistics
    pub stats: ScanStats,
    /// List of skipped files (path, reason)
    pub skipped: Vec<(String, String)>,
}

/// Scan statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanStats {
    /// Total files scanned
    pub total_files: u64,
    /// Total bytes scanned
    pub total_bytes: u64,
    /// Number of directories traversed
    pub directories: u64,
    /// Number of symlinks skipped
    pub symlinks_skipped: u64,
    /// Number of files skipped due to errors
    pub errors: u64,
    /// Scan duration in milliseconds
    pub duration_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_file_info_from_path() {
        let temp_dir = tempdir().unwrap();
        let file_path = temp_dir.path().join("test.txt");
        std::fs::write(&file_path, "test content").unwrap();

        let info = FileInfo::from_path(file_path).unwrap();
        assert_eq!(info.size, 12); // "test content" is 12 bytes
        assert!(!info.is_symlink);
    }

    #[test]
    fn test_parallelism_thread_count() {
        let light = ParallelismMode::Light.thread_count();
        assert!(light >= 1 && light <= 2);

        let normal = ParallelismMode::Normal.thread_count();
        assert!(normal >= 1);

        let aggressive = ParallelismMode::Aggressive.thread_count();
        assert!(aggressive >= 1);
        assert!(aggressive >= normal);
    }

    #[test]
    fn test_scan_config_default() {
        let config = ScanConfig::default();
        assert!(config.paths.is_empty());
        assert!(!config.follow_symlinks);
        assert_eq!(config.parallelism, ParallelismMode::Normal);
    }
}
