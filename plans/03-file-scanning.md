# File 03: File Scanning Core

## Overview

This file covers the core file scanning functionality, including directory traversal, file metadata collection, and scan orchestration. By the end of this file, you'll have a working scanner that can walk directories and collect file information.

## Prerequisites

- Completed File 01 (Project Foundation)
- Completed File 02 (Database Foundation)

---

## Phase 3.1: Add Scanning Dependencies

### Overview
Add the required dependencies for file system operations and parallel processing.

### Changes Required

#### 3.1.1 Update Cargo.toml

**File**: `src-tauri/Cargo.toml`

Add the following dependencies to the `[dependencies]` section:

```toml
[dependencies]
# ... existing dependencies ...

# File system operations
walkdir = "2.5"
ignore = "0.4"

# Time handling
chrono = { version = "0.4", features = ["serde"] }

# Parallel processing
rayon = "1.10"
num_cpus = "1.16"

# Channel for async communication
crossbeam-channel = "0.5"
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes

#### Manual Verification
- [ ] Dependencies are appropriate versions

### Code Review
Run background code-reviewer agent on `src-tauri/Cargo.toml`. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 3.2: Create Scanner Module Structure

### Overview
Create the scanner module with the basic directory walker.

### Changes Required

#### 3.2.1 Create Scanner Module

**File**: `src-tauri/src/scanner/mod.rs`

```rust
//! File scanning module

pub mod walker;
pub mod types;

pub use types::*;
pub use walker::DirectoryWalker;
```

#### 3.2.2 Create Scanner Types

**File**: `src-tauri/src/scanner/types.rs`

```rust
//! Scanner type definitions

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
    /// Create FileInfo from a path
    pub fn from_path(path: PathBuf) -> Result<Self, ScanError> {
        let metadata = std::fs::symlink_metadata(&path)?;

        let is_symlink = metadata.file_type().is_symlink();

        // Get actual file metadata (following symlinks for size)
        let file_metadata = if is_symlink {
            metadata.clone()
        } else {
            metadata
        };

        let created_at = file_metadata
            .created()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
            .unwrap_or(0);

        let modified_at = file_metadata
            .modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64)
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl Default for ScanProgress {
    fn default() -> Self {
        Self {
            total_files: 0,
            processed_files: 0,
            total_bytes: 0,
            current_path: None,
            skipped_files: 0,
            estimated_total: None,
        }
    }
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
    pub fn thread_count(&self) -> usize {
        let cpus = num_cpus::get();
        match self {
            ParallelismMode::Light => cpus.min(2).max(1),
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
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes

#### Manual Verification
- [ ] Types are comprehensive and well-documented

### Code Review
Run background code-reviewer agent on scanner type files. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 3.3: Implement Directory Walker

### Overview
Implement the core directory walking logic that traverses the file system.

### Changes Required

#### 3.3.1 Create Directory Walker

**File**: `src-tauri/src/scanner/walker.rs`

```rust
//! Directory traversal implementation

use super::types::*;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use walkdir::{DirEntry, WalkDir};

/// Directory walker for file system traversal
pub struct DirectoryWalker {
    config: ScanConfig,
    cancelled: Arc<AtomicBool>,
    progress: Arc<ScanProgressTracker>,
}

/// Thread-safe progress tracker
pub struct ScanProgressTracker {
    total_files: AtomicU64,
    processed_files: AtomicU64,
    total_bytes: AtomicU64,
    skipped_files: AtomicU64,
    directories: AtomicU64,
    symlinks_skipped: AtomicU64,
}

impl ScanProgressTracker {
    pub fn new() -> Self {
        Self {
            total_files: AtomicU64::new(0),
            processed_files: AtomicU64::new(0),
            total_bytes: AtomicU64::new(0),
            skipped_files: AtomicU64::new(0),
            directories: AtomicU64::new(0),
            symlinks_skipped: AtomicU64::new(0),
        }
    }

    pub fn increment_files(&self) {
        self.total_files.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_processed(&self) {
        self.processed_files.fetch_add(1, Ordering::Relaxed);
    }

    pub fn add_bytes(&self, bytes: u64) {
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn increment_skipped(&self) {
        self.skipped_files.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_directories(&self) {
        self.directories.fetch_add(1, Ordering::Relaxed);
    }

    pub fn increment_symlinks(&self) {
        self.symlinks_skipped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_progress(&self) -> ScanProgress {
        ScanProgress {
            total_files: self.total_files.load(Ordering::Relaxed),
            processed_files: self.processed_files.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            skipped_files: self.skipped_files.load(Ordering::Relaxed),
            current_path: None,
            estimated_total: None,
        }
    }

    pub fn get_stats(&self) -> ScanStats {
        ScanStats {
            total_files: self.total_files.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            directories: self.directories.load(Ordering::Relaxed),
            symlinks_skipped: self.symlinks_skipped.load(Ordering::Relaxed),
            errors: self.skipped_files.load(Ordering::Relaxed),
            duration_ms: 0, // Set by caller
        }
    }
}

impl Default for ScanProgressTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DirectoryWalker {
    /// Create a new directory walker with the given configuration
    pub fn new(config: ScanConfig) -> Self {
        Self {
            config,
            cancelled: Arc::new(AtomicBool::new(false)),
            progress: Arc::new(ScanProgressTracker::new()),
        }
    }

    /// Get a handle to cancel the scan
    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    /// Get the current progress
    pub fn progress(&self) -> ScanProgress {
        self.progress.get_progress()
    }

    /// Check if a directory entry should be skipped
    fn should_skip(entry: &DirEntry, follow_symlinks: bool) -> bool {
        // Skip symlinks if not following them
        if !follow_symlinks && entry.path_is_symlink() {
            return true;
        }

        // Get the file type
        let file_type = entry.file_type();

        // Skip anything that's not a file or directory
        if !file_type.is_file() && !file_type.is_dir() {
            return true;
        }

        false
    }

    /// Walk directories and collect file information
    pub fn walk(&self) -> Result<ScanResult, ScanError> {
        let start_time = std::time::Instant::now();
        let mut files: Vec<FileInfo> = Vec::new();
        let mut skipped: Vec<(String, String)> = Vec::new();

        for path in &self.config.paths {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(ScanError::Cancelled);
            }

            if !path.exists() {
                skipped.push((path.display().to_string(), "Path does not exist".to_string()));
                continue;
            }

            let walker = self.create_walker(path);

            for entry_result in walker {
                // Check for cancellation
                if self.cancelled.load(Ordering::Relaxed) {
                    return Err(ScanError::Cancelled);
                }

                match entry_result {
                    Ok(entry) => {
                        // Handle directories
                        if entry.file_type().is_dir() {
                            self.progress.increment_directories();
                            continue;
                        }

                        // Skip symlinks
                        if entry.path_is_symlink() {
                            self.progress.increment_symlinks();
                            continue;
                        }

                        // Skip non-files
                        if !entry.file_type().is_file() {
                            continue;
                        }

                        // Collect file info
                        match FileInfo::from_path(entry.path().to_path_buf()) {
                            Ok(info) => {
                                self.progress.increment_files();
                                self.progress.add_bytes(info.size);
                                self.progress.increment_processed();
                                files.push(info);
                            }
                            Err(e) => {
                                self.progress.increment_skipped();
                                skipped.push((
                                    entry.path().display().to_string(),
                                    e.to_string(),
                                ));
                            }
                        }
                    }
                    Err(e) => {
                        self.progress.increment_skipped();
                        if let Some(path) = e.path() {
                            skipped.push((
                                path.display().to_string(),
                                e.to_string(),
                            ));
                        }
                    }
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let mut stats = self.progress.get_stats();
        stats.duration_ms = duration_ms;

        Ok(ScanResult {
            files,
            stats,
            skipped,
        })
    }

    /// Walk directories with a callback for each file (streaming)
    pub fn walk_with_callback<F>(&self, mut callback: F) -> Result<ScanStats, ScanError>
    where
        F: FnMut(FileInfo) -> Result<(), ScanError>,
    {
        let start_time = std::time::Instant::now();

        for path in &self.config.paths {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(ScanError::Cancelled);
            }

            if !path.exists() {
                log::warn!("Path does not exist: {}", path.display());
                continue;
            }

            let walker = self.create_walker(path);

            for entry_result in walker {
                if self.cancelled.load(Ordering::Relaxed) {
                    return Err(ScanError::Cancelled);
                }

                match entry_result {
                    Ok(entry) => {
                        if entry.file_type().is_dir() {
                            self.progress.increment_directories();
                            continue;
                        }

                        if entry.path_is_symlink() {
                            self.progress.increment_symlinks();
                            continue;
                        }

                        if !entry.file_type().is_file() {
                            continue;
                        }

                        match FileInfo::from_path(entry.path().to_path_buf()) {
                            Ok(info) => {
                                self.progress.increment_files();
                                self.progress.add_bytes(info.size);

                                if let Err(e) = callback(info) {
                                    log::error!("Callback error: {}", e);
                                    self.progress.increment_skipped();
                                } else {
                                    self.progress.increment_processed();
                                }
                            }
                            Err(e) => {
                                log::debug!("Failed to get file info for {}: {}", entry.path().display(), e);
                                self.progress.increment_skipped();
                            }
                        }
                    }
                    Err(e) => {
                        log::debug!("Walker error: {}", e);
                        self.progress.increment_skipped();
                    }
                }
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;
        let mut stats = self.progress.get_stats();
        stats.duration_ms = duration_ms;

        Ok(stats)
    }

    /// Create a WalkDir iterator for a path
    fn create_walker(&self, path: &Path) -> WalkDir {
        let mut walker = WalkDir::new(path)
            .follow_links(self.config.follow_symlinks)
            .same_file_system(false); // Scan across mount points

        if let Some(max_depth) = self.config.max_depth {
            walker = walker.max_depth(max_depth);
        }

        walker
    }

    /// Walk directories using a channel for streaming results
    pub fn walk_channel(&self) -> (Receiver<Result<FileInfo, (PathBuf, String)>>, thread::JoinHandle<ScanStats>) {
        let (sender, receiver) = bounded(1000);
        let config = self.config.clone();
        let cancelled = Arc::clone(&self.cancelled);
        let progress = Arc::clone(&self.progress);

        let handle = thread::spawn(move || {
            let start_time = std::time::Instant::now();

            for path in &config.paths {
                if cancelled.load(Ordering::Relaxed) {
                    break;
                }

                if !path.exists() {
                    let _ = sender.send(Err((path.clone(), "Path does not exist".to_string())));
                    continue;
                }

                let mut walker = WalkDir::new(path)
                    .follow_links(config.follow_symlinks)
                    .same_file_system(false);

                if let Some(max_depth) = config.max_depth {
                    walker = walker.max_depth(max_depth);
                }

                for entry_result in walker {
                    if cancelled.load(Ordering::Relaxed) {
                        break;
                    }

                    match entry_result {
                        Ok(entry) => {
                            if entry.file_type().is_dir() {
                                progress.increment_directories();
                                continue;
                            }

                            if entry.path_is_symlink() {
                                progress.increment_symlinks();
                                continue;
                            }

                            if !entry.file_type().is_file() {
                                continue;
                            }

                            match FileInfo::from_path(entry.path().to_path_buf()) {
                                Ok(info) => {
                                    progress.increment_files();
                                    progress.add_bytes(info.size);
                                    progress.increment_processed();
                                    if sender.send(Ok(info)).is_err() {
                                        // Receiver dropped, stop walking
                                        break;
                                    }
                                }
                                Err(e) => {
                                    progress.increment_skipped();
                                    let _ = sender.send(Err((
                                        entry.path().to_path_buf(),
                                        e.to_string(),
                                    )));
                                }
                            }
                        }
                        Err(e) => {
                            progress.increment_skipped();
                            if let Some(path) = e.path() {
                                let _ = sender.send(Err((
                                    path.to_path_buf(),
                                    e.to_string(),
                                )));
                            }
                        }
                    }
                }
            }

            drop(sender); // Signal completion

            let duration_ms = start_time.elapsed().as_millis() as u64;
            let mut stats = progress.get_stats();
            stats.duration_ms = duration_ms;
            stats
        });

        (receiver, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn setup_test_dir() -> tempfile::TempDir {
        let dir = tempdir().unwrap();

        // Create some test files
        std::fs::write(dir.path().join("file1.txt"), "content1").unwrap();
        std::fs::write(dir.path().join("file2.txt"), "content2").unwrap();

        // Create a subdirectory with files
        let subdir = dir.path().join("subdir");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("file3.txt"), "content3").unwrap();

        dir
    }

    #[test]
    fn test_walk_directory() {
        let dir = setup_test_dir();

        let config = ScanConfig {
            paths: vec![dir.path().to_path_buf()],
            ..Default::default()
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        assert_eq!(result.files.len(), 3);
        assert_eq!(result.stats.total_files, 3);
    }

    #[test]
    fn test_walk_with_callback() {
        let dir = setup_test_dir();

        let config = ScanConfig {
            paths: vec![dir.path().to_path_buf()],
            ..Default::default()
        };

        let walker = DirectoryWalker::new(config);
        let mut count = 0;

        let stats = walker.walk_with_callback(|_info| {
            count += 1;
            Ok(())
        }).unwrap();

        assert_eq!(count, 3);
        assert_eq!(stats.total_files, 3);
    }

    #[test]
    fn test_walk_channel() {
        let dir = setup_test_dir();

        let config = ScanConfig {
            paths: vec![dir.path().to_path_buf()],
            ..Default::default()
        };

        let walker = DirectoryWalker::new(config);
        let (receiver, handle) = walker.walk_channel();

        let mut count = 0;
        for result in receiver {
            if result.is_ok() {
                count += 1;
            }
        }

        let stats = handle.join().unwrap();
        assert_eq!(count, 3);
        assert_eq!(stats.total_files, 3);
    }

    #[test]
    fn test_cancel_walk() {
        let dir = setup_test_dir();

        let config = ScanConfig {
            paths: vec![dir.path().to_path_buf()],
            ..Default::default()
        };

        let walker = DirectoryWalker::new(config);
        let cancel = walker.cancel_handle();

        // Cancel immediately
        cancel.store(true, Ordering::Relaxed);

        let result = walker.walk();
        assert!(matches!(result, Err(ScanError::Cancelled)));
    }

    #[test]
    fn test_nonexistent_path() {
        let config = ScanConfig {
            paths: vec![PathBuf::from("/nonexistent/path/12345")],
            ..Default::default()
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        // Should skip the nonexistent path
        assert_eq!(result.files.len(), 0);
        assert_eq!(result.skipped.len(), 1);
    }

    #[test]
    fn test_progress_tracking() {
        let tracker = ScanProgressTracker::new();

        tracker.increment_files();
        tracker.increment_files();
        tracker.increment_processed();
        tracker.add_bytes(100);

        let progress = tracker.get_progress();
        assert_eq!(progress.total_files, 2);
        assert_eq!(progress.processed_files, 1);
        assert_eq!(progress.total_bytes, 100);
    }
}
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes (including new walker tests)
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml` shows no warnings

#### Manual Verification
- [ ] Walker correctly traverses directories
- [ ] Symlinks are skipped per specification
- [ ] Progress tracking is accurate

### Code Review
Run background code-reviewer agent on `src-tauri/src/scanner/walker.rs`. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 3.4: Update lib.rs to Include Scanner

### Overview
Add the scanner module to the main library and ensure it compiles correctly.

### Changes Required

#### 3.4.1 Update lib.rs

**File**: `src-tauri/src/lib.rs`

```rust
// DupliFind - Main library entry point

mod commands;
mod db;
mod scanner;
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

#### Manual Verification
- [ ] Application starts without errors

### Code Review
Run background code-reviewer agent on `src-tauri/src/lib.rs`. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 3.5: Create Scan Commands

### Overview
Create Tauri commands to start, pause, resume, and cancel scans.

### Changes Required

#### 3.5.1 Create Scan Commands

**File**: `src-tauri/src/commands/scan.rs`

```rust
//! Scan-related Tauri commands

use crate::db::models::ScanStatus;
use crate::db::queries;
use crate::scanner::{DirectoryWalker, ScanConfig, ScanProgress, ParallelismMode};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

/// Scan request from frontend
#[derive(Debug, Clone, Deserialize)]
pub struct ScanRequest {
    pub paths: Vec<String>,
    pub parallelism: Option<String>,
}

/// Scan response for frontend
#[derive(Debug, Clone, Serialize)]
pub struct ScanResponse {
    pub session_id: i64,
    pub message: String,
}

/// Global scan state (separate from AppState for cancellation)
pub struct ScanState {
    pub cancel_flag: Option<Arc<AtomicBool>>,
}

impl ScanState {
    pub fn new() -> Self {
        Self { cancel_flag: None }
    }
}

impl Default for ScanState {
    fn default() -> Self {
        Self::new()
    }
}

/// Start a new scan
#[tauri::command]
pub async fn start_scan(
    request: ScanRequest,
    app_handle: AppHandle,
    state: State<'_, Mutex<AppState>>,
    scan_state: State<'_, Mutex<ScanState>>,
) -> Result<ScanResponse, String> {
    // Check if a scan is already running
    {
        let state = state.lock().map_err(|e| e.to_string())?;
        if state.is_scanning {
            return Err("A scan is already in progress".to_string());
        }
    }

    // Parse paths
    let paths: Vec<PathBuf> = request.paths
        .into_iter()
        .map(PathBuf::from)
        .collect();

    if paths.is_empty() {
        return Err("No paths provided for scanning".to_string());
    }

    // Parse parallelism mode
    let parallelism = match request.parallelism.as_deref() {
        Some("light") => ParallelismMode::Light,
        Some("aggressive") => ParallelismMode::Aggressive,
        _ => ParallelismMode::Normal,
    };

    // Create scan session in database
    let session_id = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let db = state.database().ok_or("Database not initialized")?;
        let db = tauri::async_runtime::block_on(async { db.lock().await });

        let path_strings: Vec<String> = paths.iter()
            .map(|p| p.display().to_string())
            .collect();

        tauri::async_runtime::block_on(async {
            queries::scan_sessions::create(db.pool(), &path_strings).await
        }).map_err(|e| e.to_string())?
    };

    // Update state to indicate scanning
    {
        let mut state = state.lock().map_err(|e| e.to_string())?;
        state.is_scanning = true;
        state.current_scan_id = Some(session_id);
    }

    // Create scan configuration
    let config = ScanConfig {
        paths,
        follow_symlinks: false,
        max_depth: None,
        parallelism,
    };

    // Create walker and get cancel handle
    let walker = DirectoryWalker::new(config);
    let cancel_handle = walker.cancel_handle();

    // Store cancel handle
    {
        let mut scan_state = scan_state.lock().map_err(|e| e.to_string())?;
        scan_state.cancel_flag = Some(cancel_handle);
    }

    // Spawn scan task
    let state_clone = state.inner().clone();
    let scan_state_clone = scan_state.inner().clone();

    tauri::async_runtime::spawn(async move {
        // Get channel for streaming results
        let (receiver, walker_handle) = walker.walk_channel();

        let mut file_count: u64 = 0;
        let mut total_size: u64 = 0;

        // Process files as they come in
        for result in receiver {
            match result {
                Ok(file_info) => {
                    file_count += 1;
                    total_size += file_info.size;

                    // Emit progress event every 100 files
                    if file_count % 100 == 0 {
                        let progress = ScanProgress {
                            total_files: file_count,
                            processed_files: file_count,
                            total_bytes: total_size,
                            current_path: Some(file_info.path.display().to_string()),
                            skipped_files: 0,
                            estimated_total: None,
                        };
                        let _ = app_handle.emit("scan-progress", progress);
                    }

                    // TODO: Store file info and process for duplicates
                    // This will be implemented in the duplicate detection phase
                }
                Err((path, error)) => {
                    log::debug!("Skipped file {}: {}", path.display(), error);
                }
            }
        }

        // Wait for walker to complete
        let stats = walker_handle.join().unwrap_or_default();

        // Update database with final stats
        if let Ok(state) = state_clone.lock() {
            if let Some(db) = state.database() {
                let db = db.blocking_lock();
                let _ = tauri::async_runtime::block_on(async {
                    queries::scan_sessions::update_stats(
                        db.pool(),
                        session_id,
                        stats.total_files as i64,
                        stats.total_bytes as i64,
                        0, // duplicate_groups - set later
                        0, // wasted_space - set later
                    ).await
                });

                let _ = tauri::async_runtime::block_on(async {
                    queries::scan_sessions::update_status(
                        db.pool(),
                        session_id,
                        ScanStatus::Completed,
                    ).await
                });
            }
        }

        // Clear scanning state
        if let Ok(mut state) = state_clone.lock() {
            state.is_scanning = false;
            state.current_scan_id = None;
        }

        // Clear cancel flag
        if let Ok(mut scan_state) = scan_state_clone.lock() {
            scan_state.cancel_flag = None;
        }

        // Emit completion event
        let _ = app_handle.emit("scan-complete", serde_json::json!({
            "session_id": session_id,
            "stats": stats,
        }));
    });

    Ok(ScanResponse {
        session_id,
        message: "Scan started".to_string(),
    })
}

/// Cancel the current scan
#[tauri::command]
pub async fn cancel_scan(
    state: State<'_, Mutex<AppState>>,
    scan_state: State<'_, Mutex<ScanState>>,
) -> Result<(), String> {
    // Set cancel flag
    {
        let scan_state = scan_state.lock().map_err(|e| e.to_string())?;
        if let Some(cancel_flag) = &scan_state.cancel_flag {
            cancel_flag.store(true, Ordering::Relaxed);
        } else {
            return Err("No scan in progress".to_string());
        }
    }

    // Update database status
    {
        let state = state.lock().map_err(|e| e.to_string())?;
        if let (Some(db), Some(session_id)) = (state.database(), state.current_scan_id) {
            let db = db.blocking_lock();
            tauri::async_runtime::block_on(async {
                queries::scan_sessions::update_status(
                    db.pool(),
                    session_id,
                    ScanStatus::Cancelled,
                ).await
            }).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Get current scan progress
#[tauri::command]
pub async fn get_scan_progress(
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<ScanProgress>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;

    if !state.is_scanning {
        return Ok(None);
    }

    // Return a basic progress - actual progress is emitted via events
    Ok(Some(ScanProgress::default()))
}

/// Check if a scan is currently running
#[tauri::command]
pub async fn is_scanning(
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(state.is_scanning)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_state_new() {
        let state = ScanState::new();
        assert!(state.cancel_flag.is_none());
    }

    #[test]
    fn test_scan_state_default() {
        let state = ScanState::default();
        assert!(state.cancel_flag.is_none());
    }
}
```

#### 3.5.2 Update Commands Module

**File**: `src-tauri/src/commands/mod.rs`

```rust
//! Tauri command handlers

pub mod protected;
pub mod scan;
pub mod settings;

/// Simple greet command for testing
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to DupliFind.", name)
}

// Re-export command functions for convenience
pub use protected::*;
pub use scan::*;
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

### Success Criteria

#### Automated Verification
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes

#### Manual Verification
- [ ] Scan commands compile correctly

### Code Review
Run background code-reviewer agent on `src-tauri/src/commands/scan.rs`. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 3.6: Register Scan Commands and State

### Overview
Register the scan commands and scan state in the Tauri application.

### Changes Required

#### 3.6.1 Update lib.rs

**File**: `src-tauri/src/lib.rs`

```rust
// DupliFind - Main library entry point

mod commands;
mod db;
mod scanner;
mod state;

use commands::scan::ScanState;
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

            // Initialize scan state
            app.manage(Mutex::new(ScanState::new()));

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
            // Scan commands
            commands::start_scan,
            commands::cancel_scan,
            commands::get_scan_progress,
            commands::is_scanning,
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
- [ ] Application starts without errors

### Code Review
Run background code-reviewer agent on `src-tauri/src/lib.rs`. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 3.7: Add Rust Unit Tests for Scanner

### Overview
Add comprehensive unit tests for the scanner module.

### Changes Required

#### 3.7.1 Create Scanner Integration Tests

**File**: `src-tauri/src/scanner/tests.rs`

```rust
//! Integration tests for the scanner module

#[cfg(test)]
mod integration_tests {
    use crate::scanner::{DirectoryWalker, FileInfo, ScanConfig, ParallelismMode};
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    fn create_test_structure() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create directory structure
        // root/
        //   file1.txt (10 bytes)
        //   file2.txt (20 bytes)
        //   subdir1/
        //     file3.txt (15 bytes)
        //     file4.txt (25 bytes)
        //   subdir2/
        //     deep/
        //       file5.txt (30 bytes)
        //   empty_dir/

        // Root files
        let mut f1 = File::create(root.join("file1.txt")).unwrap();
        f1.write_all(&[0u8; 10]).unwrap();

        let mut f2 = File::create(root.join("file2.txt")).unwrap();
        f2.write_all(&[0u8; 20]).unwrap();

        // subdir1
        fs::create_dir(root.join("subdir1")).unwrap();
        let mut f3 = File::create(root.join("subdir1/file3.txt")).unwrap();
        f3.write_all(&[0u8; 15]).unwrap();

        let mut f4 = File::create(root.join("subdir1/file4.txt")).unwrap();
        f4.write_all(&[0u8; 25]).unwrap();

        // subdir2/deep
        fs::create_dir_all(root.join("subdir2/deep")).unwrap();
        let mut f5 = File::create(root.join("subdir2/deep/file5.txt")).unwrap();
        f5.write_all(&[0u8; 30]).unwrap();

        // empty_dir
        fs::create_dir(root.join("empty_dir")).unwrap();

        dir
    }

    #[test]
    fn test_walk_all_files() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        assert_eq!(result.files.len(), 5, "Should find 5 files");
        assert_eq!(result.stats.total_files, 5);
        assert_eq!(result.stats.total_bytes, 100); // 10 + 20 + 15 + 25 + 30
    }

    #[test]
    fn test_walk_with_depth_limit() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: Some(2), // Only root and immediate subdirs
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        // Should find root files and subdir1 files, but not subdir2/deep/file5.txt
        assert_eq!(result.files.len(), 4, "Should find 4 files with depth limit 2");
    }

    #[test]
    fn test_walk_multiple_paths() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![
                test_dir.path().join("subdir1"),
                test_dir.path().join("subdir2"),
            ],
            follow_symlinks: false,
            max_depth: None,
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        assert_eq!(result.files.len(), 3, "Should find 3 files in subdir1 and subdir2");
    }

    #[test]
    fn test_file_sizes_correct() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().join("file1.txt")],
            follow_symlinks: false,
            max_depth: Some(1),
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        // When pointing directly to a file, walkdir treats it differently
        // So let's scan the root and verify sizes
        let config2 = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: Some(1),
            parallelism: ParallelismMode::Light,
        };

        let walker2 = DirectoryWalker::new(config2);
        let result2 = walker2.walk().unwrap();

        // Find file1.txt
        let file1 = result2.files.iter()
            .find(|f| f.path.file_name().unwrap() == "file1.txt")
            .unwrap();

        assert_eq!(file1.size, 10);
    }

    #[test]
    fn test_progress_tracking() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);

        // Check initial progress
        let progress = walker.progress();
        assert_eq!(progress.total_files, 0);

        // Walk and check final progress matches result
        let result = walker.walk().unwrap();
        assert_eq!(result.stats.total_files, 5);
    }

    #[test]
    fn test_symlink_skipping() {
        let test_dir = create_test_structure();

        // Create a symlink (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_path = test_dir.path().join("link_to_file1");
            symlink(test_dir.path().join("file1.txt"), &link_path).unwrap();

            let config = ScanConfig {
                paths: vec![test_dir.path().to_path_buf()],
                follow_symlinks: false,
                max_depth: Some(1),
                parallelism: ParallelismMode::Light,
            };

            let walker = DirectoryWalker::new(config);
            let result = walker.walk().unwrap();

            // Should not include the symlink
            let has_symlink = result.files.iter()
                .any(|f| f.path.file_name().unwrap() == "link_to_file1");
            assert!(!has_symlink, "Symlink should be skipped");

            // Stats should track skipped symlinks
            assert_eq!(result.stats.symlinks_skipped, 1);
        }
    }

    #[test]
    fn test_empty_directory() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().join("empty_dir")],
            follow_symlinks: false,
            max_depth: None,
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        assert_eq!(result.files.len(), 0, "Empty directory should have no files");
    }

    #[test]
    fn test_parallelism_modes() {
        // Just verify the thread counts are reasonable
        let light = ParallelismMode::Light.thread_count();
        let normal = ParallelismMode::Normal.thread_count();
        let aggressive = ParallelismMode::Aggressive.thread_count();

        assert!(light <= 2);
        assert!(normal >= light);
        assert!(aggressive >= normal);
    }

    #[test]
    fn test_file_info_metadata() {
        let test_dir = create_test_structure();
        let file_path = test_dir.path().join("file1.txt");

        let info = FileInfo::from_path(file_path).unwrap();

        assert_eq!(info.size, 10);
        assert!(!info.is_symlink);
        assert!(info.created_at > 0 || info.modified_at > 0);
    }
}
```

#### 3.7.2 Update Scanner Module to Include Tests

**File**: `src-tauri/src/scanner/mod.rs`

```rust
//! File scanning module

pub mod types;
pub mod walker;

#[cfg(test)]
mod tests;

pub use types::*;
pub use walker::DirectoryWalker;
```

### Success Criteria

#### Automated Verification
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes all tests
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml -- --nocapture scanner` shows scanner tests passing

#### Manual Verification
- [ ] Tests cover the main scanner functionality

### Code Review
Run background code-reviewer agent on `src-tauri/src/scanner/tests.rs`. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 3.8: Add Folder Picker UI for Scan Scope Selection

### Overview
Add a folder picker UI that allows users to select drives/folders to scan, with persistence of last scan settings.

### Changes Required

#### 3.8.1 Add Dialog Plugin

```bash
npm run tauri add dialog
```

Update capabilities to include dialog permissions:

**File**: `src-tauri/capabilities/default.json`

Add to permissions array:
```json
"dialog:allow-open"
```

#### 3.8.2 Create Folder Picker Component

**File**: `src/lib/components/FolderPicker.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';

  interface Props {
    selectedPaths: string[];
    onPathsChange: (paths: string[]) => void;
  }

  let { selectedPaths, onPathsChange }: Props = $props();

  async function addFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: true,
        title: 'Select folders to scan',
      });

      if (selected) {
        const newPaths = Array.isArray(selected) ? selected : [selected];
        const uniquePaths = [...new Set([...selectedPaths, ...newPaths])];
        onPathsChange(uniquePaths);
      }
    } catch (e) {
      console.error('Failed to select folder:', e);
    }
  }

  function removePath(path: string) {
    onPathsChange(selectedPaths.filter((p) => p !== path));
  }

  function clearAll() {
    onPathsChange([]);
  }

  function truncatePath(path: string): string {
    if (path.length <= 50) return path;
    const parts = path.split('/');
    if (parts.length <= 3) return path;
    return `${parts[0]}/${parts[1]}/.../${parts.slice(-2).join('/')}`;
  }
</script>

<div class="folder-picker">
  <div class="header">
    <h3>Scan Locations</h3>
    <div class="actions">
      {#if selectedPaths.length > 0}
        <button class="clear-btn" onclick={clearAll}>Clear All</button>
      {/if}
      <button class="add-btn" onclick={addFolder}>Add Folder</button>
    </div>
  </div>

  {#if selectedPaths.length === 0}
    <div class="empty-state">
      <p>No folders selected</p>
      <p class="hint">Click "Add Folder" to select folders to scan for duplicates</p>
    </div>
  {:else}
    <ul class="path-list">
      {#each selectedPaths as path}
        <li>
          <span class="path" title={path}>{truncatePath(path)}</span>
          <button class="remove-btn" onclick={() => removePath(path)} aria-label="Remove {path}">
            ×
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .folder-picker {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  h3 {
    margin: 0;
    font-size: 1rem;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .add-btn {
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .clear-btn {
    padding: 0.5rem 1rem;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .empty-state {
    text-align: center;
    padding: 2rem;
    color: var(--text-secondary);
  }

  .empty-state p {
    margin: 0.25rem 0;
  }

  .empty-state .hint {
    font-size: 0.85rem;
  }

  .path-list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 200px;
    overflow-y: auto;
  }

  .path-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background: var(--background);
    border-radius: 4px;
    margin-bottom: 0.5rem;
  }

  .path {
    font-family: var(--font-mono);
    font-size: 0.85rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 1.25rem;
    cursor: pointer;
    padding: 0 0.25rem;
    line-height: 1;
  }

  .remove-btn:hover {
    color: var(--error);
  }
</style>
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

#### Manual Verification
- [ ] Folder picker opens native file dialog
- [ ] Multiple folders can be selected
- [ ] Folders can be removed individually
- [ ] "Clear All" removes all selected folders

### Code Review
Run background code-reviewer agent on `src/lib/components/FolderPicker.svelte`.

### Commit
Execute `/cl:commit`

---

## Phase 3.9: Create Scan Button with Folder Picker Integration

### Overview
Create the scan button component that integrates with the folder picker, persists last scan settings, and **automatically restores them on app launch**.

### Key Behavior: App Launch Restoration
When the app starts, the following scan settings MUST be automatically restored:
1. **Last scanned paths** - The folders/drives selected in the previous scan session
2. **Parallelism mode** - The CPU usage setting from the previous scan

This ensures users can quickly re-run their last scan without re-selecting folders.

### Changes Required

#### 3.9.1 Create Scan Component

**File**: `src/lib/components/ScanButton.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import FolderPicker from './FolderPicker.svelte';

  interface ScanProgress {
    total_files: number;
    processed_files: number;
    total_bytes: number;
    current_path: string | null;
    skipped_files: number;
  }

  interface ScanStats {
    total_files: number;
    total_bytes: number;
    directories: number;
    symlinks_skipped: number;
    errors: number;
    duration_ms: number;
  }

  let isScanning = $state(false);
  let progress = $state<ScanProgress | null>(null);
  let scanResult = $state<{ session_id: number; stats: ScanStats } | null>(null);
  let error = $state<string | null>(null);
  let selectedPaths = $state<string[]>([]);

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;

  onMount(async () => {
    // Load last scan paths from settings
    await loadLastScanPaths();

    // Listen for progress events
    unlistenProgress = await listen<ScanProgress>('scan-progress', (event) => {
      progress = event.payload;
    });

    // Listen for completion events
    unlistenComplete = await listen<{ session_id: number; stats: ScanStats }>('scan-complete', (event) => {
      scanResult = event.payload;
      isScanning = false;
      progress = null;
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenComplete?.();
  });

  async function loadLastScanPaths() {
    try {
      const setting = await invoke<{ value: string } | null>('get_setting', { key: 'last_scan_paths' });
      if (setting?.value) {
        selectedPaths = JSON.parse(setting.value);
      }
    } catch (e) {
      console.error('Failed to load last scan paths:', e);
    }
  }

  async function saveLastScanPaths() {
    try {
      await invoke('set_setting', {
        key: 'last_scan_paths',
        value: JSON.stringify(selectedPaths),
      });
    } catch (e) {
      console.error('Failed to save scan paths:', e);
    }
  }

  function handlePathsChange(paths: string[]) {
    selectedPaths = paths;
  }

  async function startScan() {
    if (selectedPaths.length === 0) {
      error = 'Please select at least one folder to scan';
      return;
    }

    error = null;
    scanResult = null;

    try {
      // Save paths for next time
      await saveLastScanPaths();

      isScanning = true;
      await invoke('start_scan', {
        request: {
          paths: selectedPaths,
          parallelism: 'normal',
        },
      });
    } catch (e) {
      error = String(e);
      isScanning = false;
    }
  }

  async function cancelScan() {
    try {
      await invoke('cancel_scan');
      isScanning = false;
      progress = null;
    } catch (e) {
      error = String(e);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const seconds = Math.floor(ms / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  }
</script>

<div class="scan-container">
  {#if !isScanning}
    <FolderPicker {selectedPaths} onPathsChange={handlePathsChange} />
  {/if}

  <div class="scan-controls">
    {#if isScanning}
      <button class="cancel-button" onclick={cancelScan}>
        Cancel Scan
      </button>
    {:else}
      <button class="scan-button" onclick={startScan} disabled={selectedPaths.length === 0}>
        Start Scan
      </button>
    {/if}
  </div>

  {#if error}
    <div class="error-message">
      Error: {error}
    </div>
  {/if}

  {#if isScanning && progress}
    <div class="progress-container">
      <div class="progress-header">Scanning...</div>
      <div class="progress-stats">
        <div class="stat">
          <span class="label">Files:</span>
          <span class="value">{progress.total_files.toLocaleString()}</span>
        </div>
        <div class="stat">
          <span class="label">Size:</span>
          <span class="value">{formatBytes(progress.total_bytes)}</span>
        </div>
        <div class="stat">
          <span class="label">Skipped:</span>
          <span class="value">{progress.skipped_files}</span>
        </div>
      </div>
      {#if progress.current_path}
        <div class="current-path" title={progress.current_path}>
          {progress.current_path}
        </div>
      {/if}
    </div>
  {/if}

  {#if scanResult}
    <div class="result-container">
      <div class="result-header">Scan Complete</div>
      <div class="result-stats">
        <div class="stat">
          <span class="label">Total Files:</span>
          <span class="value">{scanResult.stats.total_files.toLocaleString()}</span>
        </div>
        <div class="stat">
          <span class="label">Total Size:</span>
          <span class="value">{formatBytes(scanResult.stats.total_bytes)}</span>
        </div>
        <div class="stat">
          <span class="label">Directories:</span>
          <span class="value">{scanResult.stats.directories.toLocaleString()}</span>
        </div>
        <div class="stat">
          <span class="label">Duration:</span>
          <span class="value">{formatDuration(scanResult.stats.duration_ms)}</span>
        </div>
        <div class="stat">
          <span class="label">Symlinks Skipped:</span>
          <span class="value">{scanResult.stats.symlinks_skipped}</span>
        </div>
        <div class="stat">
          <span class="label">Errors:</span>
          <span class="value">{scanResult.stats.errors}</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .scan-container {
    width: 100%;
    max-width: 500px;
    padding: 1rem;
  }

  .scan-controls {
    margin-bottom: 1rem;
  }

  .scan-button,
  .cancel-button {
    width: 100%;
    padding: 0.75rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .scan-button {
    background: var(--primary);
    color: white;
  }

  .cancel-button {
    background: var(--error);
    color: white;
  }

  .scan-button:hover,
  .cancel-button:hover {
    opacity: 0.9;
  }

  .error-message {
    padding: 0.75rem;
    background: var(--error-bg);
    color: var(--error);
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .progress-container,
  .result-container {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
  }

  .progress-header,
  .result-header {
    font-weight: 600;
    margin-bottom: 0.75rem;
    font-size: 1.1rem;
  }

  .progress-stats,
  .result-stats {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.5rem;
  }

  .stat {
    display: flex;
    justify-content: space-between;
    padding: 0.25rem 0;
  }

  .label {
    color: var(--text-secondary);
  }

  .value {
    font-weight: 500;
  }

  .current-path {
    margin-top: 0.75rem;
    padding: 0.5rem;
    background: var(--background);
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.8rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }

  .result-container {
    background: var(--success-bg);
  }

  .result-header {
    color: var(--success);
  }
</style>
```

#### 3.8.2 Update App.svelte

**File**: `src/App.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import ScanButton from './lib/components/ScanButton.svelte';

  let name = $state('');
  let greeting = $state('');

  async function greet() {
    greeting = await invoke('greet', { name });
  }
</script>

<main>
  <h1>DupliFind</h1>
  <p class="subtitle">Find and remove duplicate files</p>

  <div class="scan-section">
    <ScanButton />
  </div>

  <div class="test-section">
    <h2>Backend Connection Test</h2>
    <form onsubmit={(e) => { e.preventDefault(); greet(); }}>
      <input
        type="text"
        bind:value={name}
        placeholder="Enter your name"
      />
      <button type="submit">Test</button>
    </form>
    {#if greeting}
      <p class="greeting">{greeting}</p>
    {/if}
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 2rem;
    min-height: 100vh;
  }

  h1 {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
  }

  .subtitle {
    color: var(--text-secondary);
    margin-bottom: 2rem;
  }

  .scan-section {
    width: 100%;
    max-width: 500px;
    margin-bottom: 2rem;
  }

  .test-section {
    background: var(--surface);
    padding: 1.5rem;
    border-radius: 8px;
    width: 100%;
    max-width: 400px;
  }

  .test-section h2 {
    font-size: 1rem;
    margin-bottom: 1rem;
    color: var(--text-secondary);
  }

  form {
    display: flex;
    gap: 0.5rem;
  }

  input {
    flex: 1;
    padding: 0.5rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--background);
    color: var(--text);
  }

  button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    background: var(--primary);
    color: white;
    cursor: pointer;
  }

  button:hover {
    opacity: 0.9;
  }

  .greeting {
    margin-top: 1rem;
    padding: 0.75rem;
    background: var(--success-bg);
    border-radius: 4px;
    color: var(--success);
  }
</style>
```

#### 3.8.3 Create lib directory structure

Ensure the lib/components directory exists:

```bash
mkdir -p src/lib/components
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes
- [ ] `ls src/lib/components/ScanButton.svelte` shows component exists

#### Manual Verification
- [ ] `npm run tauri dev` starts without errors
- [ ] Clicking "Start Test Scan" initiates a scan
- [ ] Progress updates are displayed during scan
- [ ] Scan can be cancelled
- [ ] Completion results are shown
- [ ] **App launch restoration**: On app startup, previously selected scan paths are automatically loaded and displayed
- [ ] **Persistence verification**: Close app, reopen, and verify paths are still shown

### Code Review
Run background code-reviewer agent on Svelte components. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## End of File 03

After completing all phases in this file, you should have:

1. Scanner dependencies added (walkdir, rayon, etc.)
2. Scanner module with types and directory walker
3. Progress tracking for scans
4. Scan commands (start, cancel, progress)
5. Folder picker UI for scan scope selection
6. Last scan paths persistence (remembers settings)
7. Comprehensive unit tests for scanner

**Next**: Proceed to [04-duplicate-detection.md](./04-duplicate-detection.md) to implement the duplicate detection algorithm.
