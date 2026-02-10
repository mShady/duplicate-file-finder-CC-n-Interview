//! Scan-related Tauri commands

use crate::db::models::ScanStatus;
use crate::db::queries;
use crate::scanner::{DirectoryWalker, ParallelismMode, ScanConfig, ScanProgress};
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
                    if file_count.is_multiple_of(100) {
                        let progress = ScanProgress {
                            total_files: file_count,
                            processed_files: file_count,
                            total_bytes: total_size,
                            current_path: Some(file_info.path.display().to_string()),
                            skipped_files: 0,
                            estimated_total: None,
                        };
                        let _ = handle.emit("scan-progress", progress);
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
        let walk_stats = walker_handle.join().unwrap_or_default();

        // Update database with final stats - retrieve state from AppHandle
        // Extract the db Arc before awaiting to avoid holding MutexGuard across await
        let app_state = handle.state::<Mutex<AppState>>();
        let db_arc = app_state.lock().ok().and_then(|s| s.database());

        #[allow(clippy::cast_possible_wrap)]
        if let Some(db_arc) = db_arc {
            let db = db_arc.lock().await;
            let _ = queries::scan_sessions::update_stats(
                db.pool(),
                session_id,
                walk_stats.total_files as i64,
                walk_stats.total_bytes as i64,
                0, // duplicate_groups - set later
                0, // wasted_space - set later
            )
            .await;

            let _ =
                queries::scan_sessions::update_status(db.pool(), session_id, ScanStatus::Completed)
                    .await;
        }

        // Clear scanning state
        if let Ok(mut state) = app_state.lock() {
            state.is_scanning = false;
            state.current_scan_id = None;
        }

        // Clear cancel flag
        let scan_state = handle.state::<Mutex<ScanState>>();
        if let Ok(mut scan_state) = scan_state.lock() {
            scan_state.cancel_flag = None;
        }

        // Emit completion event
        let _ = handle.emit(
            "scan-complete",
            serde_json::json!({
                "session_id": session_id,
                "stats": walk_stats,
            }),
        );
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
