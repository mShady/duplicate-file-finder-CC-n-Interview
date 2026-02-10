//! Scan-related Tauri commands

use crate::db::models::ScanStatus;
use crate::db::queries;
use crate::scanner::{
    DetectionResult, DirectoryWalker, DuplicateDetector, FileInfo, ParallelismMode, ScanConfig,
    ScanProgress,
};
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
    pub quick_scan: Option<bool>,
}

/// Scan response for frontend
#[derive(Debug, Clone, Serialize)]
pub struct ScanResponse {
    pub session_id: i64,
    pub message: String,
}

/// Scan completion data
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

/// Global scan state (separate from `AppState` for cancellation)
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
#[allow(clippy::too_many_lines)]
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
    let paths: Vec<PathBuf> = request.paths.into_iter().map(PathBuf::from).collect();

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
        let db = {
            let state = state.lock().map_err(|e| e.to_string())?;
            state.database().ok_or("Database not initialized")?
        };
        let db = db.lock().await;

        let path_strings: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();

        queries::scan_sessions::create(db.pool(), &path_strings)
            .await
            .map_err(|e| e.to_string())?
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

    // Clone the AppHandle for use in the spawned task
    let handle = app_handle.clone();

    // Spawn scan task
    tauri::async_runtime::spawn(async move {
        let start_time = std::time::Instant::now();

        // Phase 1: Collect all files
        log::info!("Starting file collection...");
        let (receiver, walker_handle) = walker.walk_channel();

        let mut files: Vec<FileInfo> = Vec::new();
        let mut file_count: u64 = 0;
        let mut total_size: u64 = 0;

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
                        let _ = handle.emit("scan-progress", &progress);
                    }

                    files.push(file_info);
                }
                Err((path, error)) => {
                    log::debug!("Skipped file {}: {}", path.display(), error);
                }
            }
        }

        let scan_stats = walker_handle.join().unwrap_or_default();
        log::info!(
            "File collection complete: {} files, {} bytes",
            file_count,
            total_size
        );

        // Phase 2: Detect duplicates
        log::info!("Starting duplicate detection...");
        let _ = handle.emit(
            "scan-phase",
            serde_json::json!({
                "phase": "detecting",
                "message": "Analyzing files for duplicates..."
            }),
        );

        let mut detector = DuplicateDetector::new();
        let detection_result = match detector.detect(files) {
            Ok(result) => result,
            Err(e) => {
                log::error!("Detection failed: {}", e);

                // Update status to failed
                let app_state = handle.state::<Mutex<AppState>>();
                let db_arc = app_state.lock().ok().and_then(|s| s.database());
                if let Some(db_arc) = db_arc {
                    let db = db_arc.lock().await;
                    let _ = queries::scan_sessions::update_status(
                        db.pool(),
                        session_id,
                        ScanStatus::Failed,
                    )
                    .await;
                }

                // Clear scanning state
                let app_state = handle.state::<Mutex<AppState>>();
                if let Ok(mut state) = app_state.lock() {
                    state.is_scanning = false;
                    state.current_scan_id = None;
                }

                let _ = handle.emit(
                    "scan-error",
                    serde_json::json!({
                        "session_id": session_id,
                        "error": e.to_string()
                    }),
                );
                return;
            }
        };

        log::info!(
            "Detection complete: {} groups, {} duplicates, {} bytes wasted",
            detection_result.groups.len(),
            detection_result.duplicate_count,
            detection_result.total_wasted_space
        );

        // Phase 3: Store results in database
        let app_state = handle.state::<Mutex<AppState>>();
        let db_arc = app_state.lock().ok().and_then(|s| s.database());

        #[allow(clippy::cast_possible_wrap)]
        if let Some(db_arc) = db_arc {
            let db = db_arc.lock().await;

            // Store duplicate groups and files
            for group in &detection_result.groups {
                let group_id = queries::duplicate_groups::create(
                    db.pool(),
                    &group.hash,
                    group.file_size as i64,
                    group.files.len() as i32,
                    group.wasted_space as i64,
                    Some(session_id),
                )
                .await;

                if let Ok(group_id) = group_id {
                    for file in &group.files {
                        let _ = queries::scanned_files::insert(
                            db.pool(),
                            &file.path.display().to_string(),
                            file.size as i64,
                            None, // partial_hash stored separately
                            Some(&group.hash),
                            file.created_at,
                            file.modified_at,
                            Some(group_id),
                            Some(session_id),
                        )
                        .await;
                    }
                }
            }

            // Update session stats
            let _ = queries::scan_sessions::update_stats(
                db.pool(),
                session_id,
                scan_stats.total_files as i64,
                scan_stats.total_bytes as i64,
                detection_result.groups.len() as i32,
                detection_result.total_wasted_space as i64,
            )
            .await;

            let _ = queries::scan_sessions::update_status(
                db.pool(),
                session_id,
                ScanStatus::Completed,
            )
            .await;
        }

        // Clear scanning state
        let app_state = handle.state::<Mutex<AppState>>();
        if let Ok(mut state) = app_state.lock() {
            state.is_scanning = false;
            state.current_scan_id = None;
        }

        // Clear cancel flag
        let scan_state = handle.state::<Mutex<ScanState>>();
        if let Ok(mut scan_state) = scan_state.lock() {
            scan_state.cancel_flag = None;
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Emit completion event
        let _ = handle.emit(
            "scan-complete",
            ScanComplete {
                session_id,
                total_files: scan_stats.total_files,
                total_bytes: scan_stats.total_bytes,
                duplicate_groups: detection_result.groups.len(),
                duplicate_files: detection_result.duplicate_count,
                wasted_space: detection_result.total_wasted_space,
                duration_ms,
            },
        );

        // Also emit the full detection result for the UI
        let _ = handle.emit("scan-results", &detection_result);
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
        let db = {
            let state = state.lock().map_err(|e| e.to_string())?;
            let session_id = state.current_scan_id;
            let db = state.database();
            match (db, session_id) {
                (Some(db), Some(id)) => Some((db, id)),
                _ => None,
            }
        };

        if let Some((db, session_id)) = db {
            let db = db.lock().await;
            queries::scan_sessions::update_status(db.pool(), session_id, ScanStatus::Cancelled)
                .await
                .map_err(|e| e.to_string())?;
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
pub async fn is_scanning(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(state.is_scanning)
}

/// Get the latest scan results
#[tauri::command]
pub async fn get_scan_results(
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<DetectionResult>, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };
    let db = db.lock().await;

    // Get latest completed session
    let session = queries::scan_sessions::get_latest(db.pool())
        .await
        .map_err(|e| e.to_string())?;

    let session = match session {
        Some(s) if s.status == "completed" => s,
        _ => return Ok(None),
    };

    // Get duplicate groups
    let db_groups = queries::duplicate_groups::get_by_session(db.pool(), session.id)
        .await
        .map_err(|e| e.to_string())?;

    // Build detection result from database
    let mut groups = Vec::new();
    let mut total_duplicate_count: u64 = 0;

    for db_group in db_groups {
        let db_files = queries::scanned_files::get_by_group(db.pool(), db_group.id)
            .await
            .map_err(|e| e.to_string())?;

        let files: Vec<crate::scanner::DuplicateFile> = db_files
            .into_iter()
            .enumerate()
            .map(|(i, f)| crate::scanner::DuplicateFile {
                path: PathBuf::from(&f.path),
                size: f.size as u64,
                created_at: f.created_at,
                modified_at: f.modified_at,
                is_original: i == 0, // First file (oldest) is original
            })
            .collect();

        if files.len() > 1 {
            total_duplicate_count += (files.len() - 1) as u64;
        }

        groups.push(crate::scanner::DuplicateGroup {
            id: db_group.id as u64,
            hash: db_group.hash,
            file_size: db_group.file_size as u64,
            files,
            wasted_space: db_group.wasted_space as u64,
        });
    }

    Ok(Some(DetectionResult {
        groups,
        duplicate_count: total_duplicate_count,
        total_wasted_space: session.wasted_space as u64,
        unique_files: session.total_files as u64 - total_duplicate_count,
        stats: crate::scanner::DetectionStats::default(),
    }))
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
