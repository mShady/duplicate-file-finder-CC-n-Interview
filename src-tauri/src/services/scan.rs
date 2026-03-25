//! Scan orchestration service
//!
//! Extracts the 3-phase scan pipeline (file collection, duplicate detection,
//! DB persistence) out of the Tauri command layer so it can be tested and
//! reused independently.

use crate::db::models::ScanStatus;
use crate::db::queries;
use crate::db::Database;
use crate::scanner::{
    DetectionResult, DirectoryWalker, DuplicateDetector, FileInfo, ScanConfig, ScanProgress,
};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

/// Abstraction for scan event reporting.
///
/// The command layer provides a Tauri-based implementation;
/// tests can use a no-op or collecting implementation.
pub trait ScanEventSink: Send + 'static {
    fn on_progress(&self, progress: &ScanProgress);
    fn on_phase(&self, phase: &str, message: &str);
    fn on_error(&self, session_id: i64, error: &str);
    fn on_complete(&self, completion: &ScanComplete);
    fn on_results(&self, results: &DetectionResult);
}

/// Scan cancellation state (separate from `AppState`)
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

/// Scan completion data emitted to the frontend
#[derive(Debug, Clone, Serialize)]
pub struct ScanComplete {
    pub session_id: i64,
    pub total_files: u64,
    pub total_bytes: u64,
    pub duplicate_groups: usize,
    pub duplicate_files: u64,
    pub wasted_space: u64,
    pub duration_ms: u64,
}

/// Stateless scan orchestrator.
///
/// Runs the full pipeline: walk directories → detect duplicates → persist to DB,
/// reporting progress through the provided [`ScanEventSink`].
pub struct ScanService;

impl ScanService {
    /// Execute the full scan pipeline.
    ///
    /// Designed to run inside `tauri::async_runtime::spawn`.  The caller is
    /// responsible for state cleanup (`is_scanning`, `cancel_flag`) after this
    /// method returns.
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation, clippy::too_many_lines)]
    pub async fn run(
        config: ScanConfig,
        session_id: i64,
        cancel_flag: Arc<AtomicBool>,
        db: Arc<AsyncMutex<Database>>,
        sink: impl ScanEventSink,
    ) {
        let start_time = std::time::Instant::now();

        // Phase 1: Collect all files
        log::info!("Starting file collection...");
        let walker = DirectoryWalker::new(config);

        // Share the cancel flag with the walker
        let walker_cancel = walker.cancel_handle();
        if cancel_flag.load(Ordering::Relaxed) {
            walker_cancel.store(true, Ordering::Relaxed);
        }
        // Link the external cancel flag to the walker's cancel handle
        // by spawning a tiny watcher task
        let external_flag = Arc::clone(&cancel_flag);
        let walker_flag = Arc::clone(&walker_cancel);
        let cancel_linker = tokio::spawn(async move {
            loop {
                if external_flag.load(Ordering::Relaxed) {
                    walker_flag.store(true, Ordering::Relaxed);
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            }
        });

        let (receiver, walker_handle) = walker.walk_channel();

        let mut files: Vec<FileInfo> = Vec::new();
        let mut total_bytes: u64 = 0;

        for result in receiver {
            match result {
                Ok(file_info) => {
                    total_bytes += file_info.size;

                    // Emit progress event every 100 files
                    if (files.len() + 1).is_multiple_of(100) {
                        let progress = ScanProgress {
                            total_files: (files.len() + 1) as u64,
                            processed_files: (files.len() + 1) as u64,
                            total_bytes,
                            current_path: Some(file_info.path.display().to_string()),
                            skipped_files: 0,
                            estimated_total: None,
                        };
                        sink.on_progress(&progress);
                    }

                    files.push(file_info);
                }
                Err((path, error)) => {
                    log::debug!("Skipped file {}: {}", path.display(), error);
                }
            }
        }

        cancel_linker.abort();

        let walker_stats = match walker_handle.join() {
            Ok(stats) => stats,
            Err(panic_payload) => {
                let panic_msg = panic_payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| panic_payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("unknown panic");
                log::error!("Walker thread panicked: {panic_msg}");
                sink.on_error(session_id, &format!("File collection failed: {panic_msg}"));

                let db_guard = db.lock().await;
                let _ = queries::scan_sessions::update_status(
                    db_guard.pool(),
                    session_id,
                    ScanStatus::Failed,
                )
                .await;
                return;
            }
        };
        log::info!(
            "File collection complete: {} files, {} bytes",
            walker_stats.total_files,
            walker_stats.total_bytes
        );

        // Phase 2: Detect duplicates
        log::info!("Starting duplicate detection...");
        sink.on_phase("detecting", "Analyzing files for duplicates...");

        let mut detector = DuplicateDetector::new();
        detector.set_cancel_flag(Arc::clone(&cancel_flag));

        let detection_result = match detector.detect(files) {
            Ok(result) => result,
            Err(e) => {
                log::error!("Detection failed: {e}");

                // Update status to failed
                let db_guard = db.lock().await;
                let _ = queries::scan_sessions::update_status(
                    db_guard.pool(),
                    session_id,
                    ScanStatus::Failed,
                )
                .await;
                drop(db_guard);

                sink.on_error(session_id, &e.to_string());
                return;
            }
        };

        log::info!(
            "Detection complete: {} groups, {} duplicates, {} bytes wasted",
            detection_result.groups.len(),
            detection_result.duplicate_count,
            detection_result.total_wasted_space
        );

        // Phase 3: Store results in database inside a single transaction
        // to avoid thousands of sequential round-trips under the mutex.
        {
            let db_guard = db.lock().await;

            let total_groups = detection_result.groups.len();
            let mut failed_groups: usize = 0;
            let mut persisted_wasted_space: u64 = 0;

            // Use a transaction to batch all inserts into a single disk sync
            let mut tx = match sqlx::pool::Pool::begin(db_guard.pool()).await {
                Ok(tx) => tx,
                Err(e) => {
                    log::error!("Failed to begin transaction: {e}");
                    let _ = queries::scan_sessions::update_status(
                        db_guard.pool(),
                        session_id,
                        ScanStatus::Failed,
                    )
                    .await;
                    sink.on_error(
                        session_id,
                        &format!("Failed to persist scan results: {e}"),
                    );
                    return;
                }
            };

            #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
            for group in &detection_result.groups {
                match queries::duplicate_groups::create(
                    &mut *tx,
                    &group.hash,
                    group.file_size as i64,
                    group.files.len() as i32,
                    group.wasted_space as i64,
                    session_id,
                )
                .await
                {
                    Ok(group_id) => {
                        persisted_wasted_space += group.wasted_space;
                        for file in &group.files {
                            if let Err(e) = queries::scanned_files::insert(
                                &mut *tx,
                                &file.path.display().to_string(),
                                file.size as i64,
                                None,
                                Some(&group.hash),
                                file.created_at,
                                file.modified_at,
                                Some(group_id),
                                session_id,
                            )
                            .await
                            {
                                log::warn!("Failed to insert scanned file: {e}");
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to create duplicate group: {e}");
                        failed_groups += 1;
                    }
                }
            }

            // If all group inserts failed, roll back and treat the scan as failed.
            // Without ON CONFLICT, failures here are real DB errors (disk full, constraint
            // violations from duplicate hashes in detector output, etc.).
            if total_groups > 0 && failed_groups == total_groups {
                log::error!(
                    "All {total_groups} group inserts failed — marking scan as failed"
                );
                let _ = tx.rollback().await;
                let _ = queries::scan_sessions::update_status(
                    db_guard.pool(),
                    session_id,
                    ScanStatus::Failed,
                )
                .await;
                sink.on_error(
                    session_id,
                    &format!(
                        "Failed to persist scan results: all {total_groups} group inserts failed"
                    ),
                );
                return;
            }

            if let Err(e) = tx.commit().await {
                log::error!("Failed to commit scan results transaction: {e}");
                let _ = queries::scan_sessions::update_status(
                    db_guard.pool(),
                    session_id,
                    ScanStatus::Failed,
                )
                .await;
                sink.on_error(
                    session_id,
                    &format!("Failed to commit scan results: {e}"),
                );
                return;
            }

            // Update session stats using actual persisted counts (not detection totals)
            let persisted_groups = total_groups - failed_groups;
            if let Err(e) = queries::scan_sessions::update_stats(
                db_guard.pool(),
                session_id,
                walker_stats.total_files as i64,
                walker_stats.total_bytes as i64,
                persisted_groups as i32,
                persisted_wasted_space as i64,
            )
            .await
            {
                log::warn!("Failed to update session stats: {e}");
            }

            if let Err(e) = queries::scan_sessions::update_status(
                db_guard.pool(),
                session_id,
                ScanStatus::Completed,
            )
            .await
            {
                log::warn!("Failed to update session status: {e}");
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Emit completion event
        sink.on_complete(&ScanComplete {
            session_id,
            total_files: walker_stats.total_files,
            total_bytes: walker_stats.total_bytes,
            duplicate_groups: detection_result.groups.len(),
            duplicate_files: detection_result.duplicate_count,
            wasted_space: detection_result.total_wasted_space,
            duration_ms,
        });

        // Also emit the full detection result for the UI
        sink.on_results(&detection_result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

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

    /// Collects event names for assertion in tests.
    struct MockEventSink {
        events: Arc<Mutex<Vec<String>>>,
    }

    impl MockEventSink {
        fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
            let events = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    events: Arc::clone(&events),
                },
                events,
            )
        }
    }

    impl ScanEventSink for MockEventSink {
        fn on_progress(&self, _progress: &ScanProgress) {
            self.events.lock().unwrap().push("progress".into());
        }
        fn on_phase(&self, _phase: &str, _message: &str) {
            self.events.lock().unwrap().push("phase".into());
        }
        fn on_error(&self, _session_id: i64, _error: &str) {
            self.events.lock().unwrap().push("error".into());
        }
        fn on_complete(&self, _completion: &ScanComplete) {
            self.events.lock().unwrap().push("complete".into());
        }
        fn on_results(&self, _results: &DetectionResult) {
            self.events.lock().unwrap().push("results".into());
        }
    }

    #[tokio::test]
    async fn test_scan_service_empty_directory() {
        let temp_dir = tempfile::tempdir().unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        // Set up a real database
        let db_dir = tempfile::tempdir().unwrap();
        let db = crate::db::Database::new(db_dir.path().join("test.db"))
            .await
            .unwrap();
        let db = Arc::new(AsyncMutex::new(db));

        // Create a scan session
        let session_id = {
            let db_guard = db.lock().await;
            let paths: Vec<String> = config.paths.iter().map(|p| p.display().to_string()).collect();
            queries::scan_sessions::create(db_guard.pool(), &paths)
                .await
                .unwrap()
        };

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (sink, events) = MockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, db, sink).await;

        let events = events.lock().unwrap();
        assert!(events.contains(&"phase".to_string()));
        assert!(events.contains(&"complete".to_string()));
        assert!(events.contains(&"results".to_string()));
        assert!(!events.contains(&"error".to_string()));
    }

    #[tokio::test]
    async fn test_scan_service_with_duplicates() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create duplicate files
        let content = b"duplicate content for service test";
        std::fs::write(temp_dir.path().join("file1.txt"), content).unwrap();
        std::fs::write(temp_dir.path().join("file2.txt"), content).unwrap();
        std::fs::write(temp_dir.path().join("unique.txt"), b"unique").unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let db_dir = tempfile::tempdir().unwrap();
        let db = crate::db::Database::new(db_dir.path().join("test.db"))
            .await
            .unwrap();
        let db = Arc::new(AsyncMutex::new(db));

        let session_id = {
            let db_guard = db.lock().await;
            let paths: Vec<String> = config.paths.iter().map(|p| p.display().to_string()).collect();
            queries::scan_sessions::create(db_guard.pool(), &paths)
                .await
                .unwrap()
        };

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (sink, events) = MockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, db, sink).await;

        let events = events.lock().unwrap();
        assert!(events.contains(&"complete".to_string()));
        assert!(events.contains(&"results".to_string()));
        assert!(!events.contains(&"error".to_string()));
    }

    #[tokio::test]
    async fn test_scan_service_cancellation() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create enough files so the scan doesn't finish instantly
        for i in 0..10 {
            std::fs::write(
                temp_dir.path().join(format!("file{i}.txt")),
                format!("content {i}"),
            )
            .unwrap();
        }

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let db_dir = tempfile::tempdir().unwrap();
        let db = crate::db::Database::new(db_dir.path().join("test.db"))
            .await
            .unwrap();
        let db = Arc::new(AsyncMutex::new(db));

        let session_id = {
            let db_guard = db.lock().await;
            let paths: Vec<String> = config.paths.iter().map(|p| p.display().to_string()).collect();
            queries::scan_sessions::create(db_guard.pool(), &paths)
                .await
                .unwrap()
        };

        // Cancel immediately
        let cancel_flag = Arc::new(AtomicBool::new(true));
        let (sink, events) = MockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, db, sink).await;

        let events = events.lock().unwrap();
        // Cancellation may fire error (if detection gets to run) or complete
        // with 0 files (if walker aborts before any files). Either way, no panic.
        assert!(!events.is_empty());
    }

    #[test]
    fn test_scan_complete_serialization() {
        let complete = ScanComplete {
            session_id: 1,
            total_files: 100,
            total_bytes: 5000,
            duplicate_groups: 3,
            duplicate_files: 10,
            wasted_space: 2000,
            duration_ms: 450,
        };

        let json = serde_json::to_value(&complete).unwrap();
        assert_eq!(json["session_id"], 1);
        assert_eq!(json["total_files"], 100);
        assert_eq!(json["duplicate_groups"], 3);
        assert_eq!(json["wasted_space"], 2000);
    }

    #[tokio::test]
    async fn test_scan_service_walker_stats_populated() {
        // Baseline test: a normal scan produces non-zero walker stats.
        // This establishes that if walker_handle.join() succeeds, we get real data.
        // The bug (finding #3) is that on panic, unwrap_or_default() silently
        // returns zeroed stats instead of reporting the error.
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), b"content").unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let db_dir = tempfile::tempdir().unwrap();
        let db = crate::db::Database::new(db_dir.path().join("test.db"))
            .await
            .unwrap();
        let db = Arc::new(AsyncMutex::new(db));

        let session_id = {
            let db_guard = db.lock().await;
            let paths: Vec<String> = config.paths.iter().map(|p| p.display().to_string()).collect();
            crate::db::queries::scan_sessions::create(db_guard.pool(), &paths)
                .await
                .unwrap()
        };

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (sink, events) = MockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, db, sink).await;

        let events = events.lock().unwrap();
        // A successful scan with 1 file should emit complete (not error)
        assert!(events.contains(&"complete".to_string()));
        assert!(!events.contains(&"error".to_string()));
    }

    #[test]
    fn test_scan_stats_default_is_zeroed() {
        // Documents what unwrap_or_default() returns when a thread panics.
        // BUG: walker_handle.join().unwrap_or_default() uses this on panic,
        // silently producing zeroed stats instead of propagating the error.
        let default = crate::scanner::ScanStats::default();
        assert_eq!(default.total_files, 0);
        assert_eq!(default.total_bytes, 0);
        assert_eq!(default.directories, 0);
        assert_eq!(default.symlinks_skipped, 0);
        assert_eq!(default.errors, 0);
        assert_eq!(default.duration_ms, 0);
    }

    /// Readable handle to captured events from `DetailedMockEventSink`.
    struct CapturedEvents {
        events: Arc<Mutex<Vec<String>>>,
        completions: Arc<Mutex<Vec<ScanComplete>>>,
        errors: Arc<Mutex<Vec<(i64, String)>>>,
    }

    /// Captures event payloads (not just names) for detailed assertion.
    struct DetailedMockEventSink {
        events: Arc<Mutex<Vec<String>>>,
        completions: Arc<Mutex<Vec<ScanComplete>>>,
        errors: Arc<Mutex<Vec<(i64, String)>>>,
    }

    impl DetailedMockEventSink {
        fn new() -> (Self, CapturedEvents) {
            let events = Arc::new(Mutex::new(Vec::new()));
            let completions = Arc::new(Mutex::new(Vec::new()));
            let errors = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    events: Arc::clone(&events),
                    completions: Arc::clone(&completions),
                    errors: Arc::clone(&errors),
                },
                CapturedEvents {
                    events,
                    completions,
                    errors,
                },
            )
        }
    }

    impl ScanEventSink for DetailedMockEventSink {
        fn on_progress(&self, _progress: &ScanProgress) {
            self.events.lock().unwrap().push("progress".into());
        }
        fn on_phase(&self, _phase: &str, _message: &str) {
            self.events.lock().unwrap().push("phase".into());
        }
        fn on_error(&self, session_id: i64, error: &str) {
            self.events.lock().unwrap().push("error".into());
            self.errors
                .lock()
                .unwrap()
                .push((session_id, error.to_string()));
        }
        fn on_complete(&self, completion: &ScanComplete) {
            self.events.lock().unwrap().push("complete".into());
            self.completions.lock().unwrap().push(completion.clone());
        }
        fn on_results(&self, _results: &DetectionResult) {
            self.events.lock().unwrap().push("results".into());
        }
    }

    /// Helper: set up a DB and scan session for a given temp directory.
    async fn setup_scan_db(
        paths: &[std::path::PathBuf],
    ) -> (Arc<AsyncMutex<crate::db::Database>>, i64) {
        let db_dir = tempfile::tempdir().unwrap();
        let db = crate::db::Database::new(db_dir.path().join("test.db"))
            .await
            .unwrap();
        let db = Arc::new(AsyncMutex::new(db));

        let session_id = {
            let db_guard = db.lock().await;
            let path_strings: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();
            queries::scan_sessions::create(db_guard.pool(), &path_strings)
                .await
                .unwrap()
        };

        (db, session_id)
    }

    #[tokio::test]
    async fn test_scan_service_persists_groups_to_db() {
        let temp_dir = tempfile::tempdir().unwrap();
        let content = b"duplicate content for persistence test";
        std::fs::write(temp_dir.path().join("dup1.txt"), content).unwrap();
        std::fs::write(temp_dir.path().join("dup2.txt"), content).unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let (db, session_id) = setup_scan_db(&config.paths).await;

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (sink, _captured) = DetailedMockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, Arc::clone(&db), sink).await;

        // Verify groups were persisted to DB
        let db_guard = db.lock().await;
        let groups =
            queries::duplicate_groups::get_by_session(db_guard.pool(), session_id)
                .await
                .unwrap();

        assert_eq!(groups.len(), 1, "should persist exactly 1 duplicate group");
        assert_eq!(groups[0].file_count, 2);
        assert!(groups[0].wasted_space > 0);
    }

    #[tokio::test]
    async fn test_scan_service_session_status_completed() {
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), b"content").unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let (db, session_id) = setup_scan_db(&config.paths).await;

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (sink, _events) = MockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, Arc::clone(&db), sink).await;

        // Verify session status is "completed" in DB
        let db_guard = db.lock().await;
        let session = queries::scan_sessions::get_latest(db_guard.pool())
            .await
            .unwrap()
            .expect("session should exist");

        assert_eq!(session.status, "completed");
        assert!(session.completed_at.is_some());
    }

    #[tokio::test]
    async fn test_scan_complete_event_has_correct_counts() {
        let temp_dir = tempfile::tempdir().unwrap();
        let content = b"duplicate content for event test";
        std::fs::write(temp_dir.path().join("dup1.txt"), content).unwrap();
        std::fs::write(temp_dir.path().join("dup2.txt"), content).unwrap();
        std::fs::write(temp_dir.path().join("unique.txt"), b"unique content").unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let (db, session_id) = setup_scan_db(&config.paths).await;

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (sink, captured) = DetailedMockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, db, sink).await;

        let completions = captured.completions.lock().unwrap();
        assert_eq!(completions.len(), 1, "should emit exactly 1 complete event");

        let complete = &completions[0];
        assert_eq!(complete.duplicate_groups, 1);
        assert_eq!(complete.duplicate_files, 1); // 2 files in group, 1 is duplicate
        assert_eq!(complete.total_files, 3);
        assert!(complete.wasted_space > 0);

        let errors = captured.errors.lock().unwrap();
        assert!(errors.is_empty(), "no errors on successful scan");
    }

    #[tokio::test]
    async fn test_scan_service_no_error_on_success() {
        // Baseline: a normal scan emits no error events.
        // After fixing finding #4, DB failures WILL emit errors —
        // this test ensures the happy path stays clean.
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), b"some content").unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let (db, session_id) = setup_scan_db(&config.paths).await;

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (sink, captured) = DetailedMockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, db, sink).await;

        let events = captured.events.lock().unwrap();
        assert!(events.contains(&"complete".to_string()));
        assert!(!events.contains(&"error".to_string()));

        let errors = captured.errors.lock().unwrap();
        assert!(errors.is_empty());
    }

    #[tokio::test]
    async fn test_scan_service_cancelled_status() {
        // Pre-cancel: set cancel_flag before scan starts.
        // ScanService::run() should detect cancellation during detection,
        // set session to Failed, and emit an error event.
        // Note: cancel_scan (command layer) also sets Cancelled in DB —
        // documenting what ScanService alone does here.
        let temp_dir = tempfile::tempdir().unwrap();
        let content = b"dup content for cancel test";
        std::fs::write(temp_dir.path().join("dup1.txt"), content).unwrap();
        std::fs::write(temp_dir.path().join("dup2.txt"), content).unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let (db, session_id) = setup_scan_db(&config.paths).await;

        // Pre-cancel
        let cancel_flag = Arc::new(AtomicBool::new(true));
        let (sink, captured) = DetailedMockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, Arc::clone(&db), sink).await;

        // Check what ScanService wrote to DB
        let db_guard = db.lock().await;
        let session = queries::scan_sessions::get_latest(db_guard.pool())
            .await
            .unwrap()
            .expect("session should exist");

        // ScanService sets "failed" when detector returns Err(Cancelled)
        assert_eq!(session.status, "failed");

        // Should have emitted an error event (not complete)
        let events = captured.events.lock().unwrap();
        assert!(
            events.contains(&"error".to_string()),
            "cancelled scan should emit error event"
        );
    }

    #[tokio::test]
    async fn test_scan_cancellation_mid_detection() {
        // Cancel after a short delay to hit the detection phase.
        // Verifies no panic or deadlock occurs.
        let temp_dir = tempfile::tempdir().unwrap();
        for i in 0..20 {
            std::fs::write(
                temp_dir.path().join(format!("file{i}.txt")),
                format!("content {i}"),
            )
            .unwrap();
        }

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let (db, session_id) = setup_scan_db(&config.paths).await;
        let cancel_flag = Arc::new(AtomicBool::new(false));
        let cancel_clone = Arc::clone(&cancel_flag);

        // Cancel after 10ms
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            cancel_clone.store(true, Ordering::Relaxed);
        });

        let (sink, captured) = DetailedMockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, db, sink).await;

        // Should not hang or panic — just verify we got some events
        let events = captured.events.lock().unwrap();
        assert!(!events.is_empty(), "should emit at least one event");
    }

    #[tokio::test]
    async fn test_scan_service_cancelled_emits_error_event() {
        // Pre-cancel and verify exactly one error event is emitted
        // (not both error AND complete).
        let temp_dir = tempfile::tempdir().unwrap();
        std::fs::write(temp_dir.path().join("file.txt"), b"content").unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let (db, session_id) = setup_scan_db(&config.paths).await;
        let cancel_flag = Arc::new(AtomicBool::new(true));
        let (sink, captured) = DetailedMockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, db, sink).await;

        let events = captured.events.lock().unwrap();
        let has_error = events.contains(&"error".to_string());
        let has_complete = events.contains(&"complete".to_string());

        // Should emit error XOR complete, never both
        assert!(
            has_error ^ has_complete,
            "should emit exactly one of error or complete, got error={has_error} complete={has_complete}"
        );
    }

    #[tokio::test]
    async fn test_scan_service_aborts_when_all_group_inserts_fail() {
        let temp_dir = tempfile::tempdir().unwrap();
        let content = b"duplicate for abort test";
        std::fs::write(temp_dir.path().join("dup1.txt"), content).unwrap();
        std::fs::write(temp_dir.path().join("dup2.txt"), content).unwrap();

        let config = ScanConfig {
            paths: vec![temp_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: crate::scanner::ParallelismMode::Light,
        };

        let (db, session_id) = setup_scan_db(&config.paths).await;

        // Drop the duplicate_groups table so all INSERTs fail in Phase 3
        {
            let db_guard = db.lock().await;
            sqlx::query("DROP TABLE duplicate_groups")
                .execute(db_guard.pool())
                .await
                .unwrap();
        }

        let cancel_flag = Arc::new(AtomicBool::new(false));
        let (sink, captured) = DetailedMockEventSink::new();

        ScanService::run(config, session_id, cancel_flag, db, sink).await;

        // The scan should emit an error (not complete) because all inserts failed
        let events = captured.events.lock().unwrap();
        assert!(
            events.contains(&"error".to_string()),
            "should emit error when all group inserts fail"
        );
        assert!(
            !events.contains(&"complete".to_string()),
            "should NOT emit complete when all group inserts fail"
        );

        let errors = captured.errors.lock().unwrap();
        assert!(!errors.is_empty(), "should have at least one error");
        assert!(
            errors[0].1.contains("Failed to persist") || errors[0].1.contains("Failed to commit"),
            "error message should mention persistence failure, got: {}",
            errors[0].1
        );
    }

    /// Extracts a human-readable message from a panic payload,
    /// using the same logic as `ScanService::run()`.
    fn extract_panic_message(payload: &(dyn std::any::Any + Send)) -> String {
        payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("unknown panic")
            .to_string()
    }

    #[test]
    fn test_panic_payload_extraction_str() {
        // panic!("message") produces a &str payload
        let result = std::panic::catch_unwind(|| panic!("walker exploded"));
        let msg = extract_panic_message(&*result.unwrap_err());
        assert_eq!(msg, "walker exploded");
    }

    #[test]
    fn test_panic_payload_extraction_string() {
        // panic!("{}", var) or .unwrap() on Err produces a String payload
        let result = std::panic::catch_unwind(|| {
            let reason = "formatted reason".to_string();
            panic!("{reason}");
        });
        let msg = extract_panic_message(&*result.unwrap_err());
        assert_eq!(msg, "formatted reason");
    }

    #[test]
    fn test_panic_payload_extraction_unknown() {
        // panic with a non-string type falls back to "unknown panic"
        let result = std::panic::catch_unwind(|| std::panic::panic_any(42_i32));
        let msg = extract_panic_message(&*result.unwrap_err());
        assert_eq!(msg, "unknown panic");
    }
}
