//! Directory traversal implementation

use super::types::{FileInfo, ScanConfig, ScanError, ScanProgress, ScanResult, ScanStats};
use crossbeam_channel::{bounded, Receiver};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use walkdir::WalkDir;

/// Result type for the channel-based walker: either a file info or an error with path and message
type WalkChannelItem = Result<FileInfo, (PathBuf, String)>;

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

    /// Walk directories and collect file information
    #[allow(clippy::cast_possible_truncation)]
    pub fn walk(&self) -> Result<ScanResult, ScanError> {
        let start_time = std::time::Instant::now();
        let mut files: Vec<FileInfo> = Vec::new();
        let mut skipped: Vec<(String, String)> = Vec::new();

        for path in &self.config.paths {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(ScanError::Cancelled);
            }

            if !path.exists() {
                skipped.push((
                    path.display().to_string(),
                    "Path does not exist".to_string(),
                ));
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
                                skipped.push((entry.path().display().to_string(), e.to_string()));
                            }
                        }
                    }
                    Err(e) => {
                        self.progress.increment_skipped();
                        if let Some(path) = e.path() {
                            skipped.push((path.display().to_string(), e.to_string()));
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
    #[allow(clippy::cast_possible_truncation)]
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
                                    log::error!("Callback error: {e}");
                                    self.progress.increment_skipped();
                                } else {
                                    self.progress.increment_processed();
                                }
                            }
                            Err(e) => {
                                log::debug!(
                                    "Failed to get file info for {}: {e}",
                                    entry.path().display()
                                );
                                self.progress.increment_skipped();
                            }
                        }
                    }
                    Err(e) => {
                        log::debug!("Walker error: {e}");
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

    /// Create a `WalkDir` iterator for a path
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
    #[allow(clippy::cast_possible_truncation)]
    pub fn walk_channel(&self) -> (Receiver<WalkChannelItem>, thread::JoinHandle<ScanStats>) {
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
                                    let _ = sender
                                        .send(Err((entry.path().to_path_buf(), e.to_string())));
                                }
                            }
                        }
                        Err(e) => {
                            progress.increment_skipped();
                            if let Some(path) = e.path() {
                                let _ = sender.send(Err((path.to_path_buf(), e.to_string())));
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

        let stats = walker
            .walk_with_callback(|_info| {
                count += 1;
                Ok(())
            })
            .unwrap();

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
