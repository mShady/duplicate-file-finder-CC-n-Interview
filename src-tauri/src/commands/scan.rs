//! Scan-related Tauri commands

use crate::db::queries;
use crate::scanner::{
    DetectionResult, ParallelismMode, ScanConfig, ScanProgress,
};
use crate::services::scan::{ScanComplete, ScanEventSink, ScanService, ScanState};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Mutex;
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

/// Bridges [`ScanEventSink`] to Tauri frontend events via [`AppHandle`].
struct TauriEventSink {
    handle: AppHandle,
}

impl ScanEventSink for TauriEventSink {
    fn on_progress(&self, progress: &ScanProgress) {
        let _ = self.handle.emit("scan-progress", progress);
    }

    fn on_phase(&self, phase: &str, message: &str) {
        let _ = self.handle.emit(
            "scan-phase",
            serde_json::json!({
                "phase": phase,
                "message": message,
            }),
        );
    }

    fn on_error(&self, session_id: i64, error: &str) {
        let _ = self.handle.emit(
            "scan-error",
            serde_json::json!({
                "session_id": session_id,
                "error": error,
            }),
        );
    }

    fn on_complete(&self, completion: &ScanComplete) {
        let _ = self.handle.emit("scan-complete", completion);
    }

    fn on_results(&self, results: &DetectionResult) {
        let _ = self.handle.emit("scan-results", results);
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
    // Check if a scan is already running and mark as scanning atomically
    let db_arc = {
        let mut state = state.lock().map_err(|e| e.to_string())?;
        if state.is_scanning {
            return Err("A scan is already in progress".to_string());
        }
        state.is_scanning = true;
        state.database().ok_or_else(|| {
            state.is_scanning = false;
            "Database not initialized".to_string()
        })?
    };

    // Parse paths
    let paths: Vec<PathBuf> = request.paths.into_iter().map(PathBuf::from).collect();

    if paths.is_empty() {
        let mut state = state.lock().map_err(|e| e.to_string())?;
        state.is_scanning = false;
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
        let db = db_arc.lock().await;

        let path_strings: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();

        match queries::scan_sessions::create(db.pool(), &path_strings).await {
            Ok(id) => id,
            Err(e) => {
                let mut state = state.lock().map_err(|e| e.to_string())?;
                state.is_scanning = false;
                return Err(e.to_string());
            }
        }
    };

    // Store session ID
    {
        let mut state = state.lock().map_err(|e| e.to_string())?;
        state.current_scan_id = Some(session_id);
    }

    // Build scan configuration
    let config = ScanConfig {
        paths,
        follow_symlinks: false,
        max_depth: None,
        parallelism,
    };

    // Store cancel flag so cancel_scan can reach it
    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    {
        let mut scan_state = scan_state.lock().map_err(|e| e.to_string())?;
        scan_state.cancel_flag = Some(std::sync::Arc::clone(&cancel_flag));
    }

    // Create event sink and spawn the scan task
    let handle = app_handle.clone();
    let sink = TauriEventSink {
        handle: app_handle.clone(),
    };

    tauri::async_runtime::spawn(async move {
        ScanService::run(config, session_id, cancel_flag, db_arc, sink).await;

        // Cleanup state after the service completes (success, failure, or cancellation)
        let app_state = handle.state::<Mutex<AppState>>();
        if let Ok(mut state) = app_state.lock() {
            state.is_scanning = false;
            state.current_scan_id = None;
        }

        let scan_state_ref = handle.state::<Mutex<ScanState>>();
        if let Ok(mut ss) = scan_state_ref.lock() {
            ss.cancel_flag = None;
        };
    });

    Ok(ScanResponse {
        session_id,
        message: "Scan started".to_string(),
    })
}

/// Cancel the current scan
#[tauri::command]
pub async fn cancel_scan(
    _state: State<'_, Mutex<AppState>>,
    scan_state: State<'_, Mutex<ScanState>>,
) -> Result<(), String> {
    // Only set the cancel flag — ScanService::run() handles the DB status
    // update when it detects cancellation. This avoids a race condition where
    // both cancel_scan and ScanService write conflicting statuses.
    let scan_state = scan_state.lock().map_err(|e| e.to_string())?;
    if let Some(cancel_flag) = &scan_state.cancel_flag {
        cancel_flag.store(true, Ordering::Relaxed);
    } else {
        return Err("No scan in progress".to_string());
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
                size: f.size.try_into().unwrap_or(0u64),
                created_at: f.created_at,
                modified_at: f.modified_at,
                is_original: i == 0, // First file (oldest) is original
            })
            .collect();

        if files.len() > 1 {
            total_duplicate_count += (files.len() - 1) as u64;
        }

        groups.push(crate::scanner::DuplicateGroup {
            id: db_group.id.try_into().unwrap_or(0u64),
            hash: db_group.hash,
            file_size: db_group.file_size.try_into().unwrap_or(0u64),
            files,
            wasted_space: db_group.wasted_space.try_into().unwrap_or(0u64),
        });
    }

    Ok(Some(DetectionResult {
        groups,
        duplicate_count: total_duplicate_count,
        total_wasted_space: session.wasted_space.try_into().unwrap_or(0u64),
        unique_files: (session.total_files.try_into().unwrap_or(0u64)).saturating_sub(total_duplicate_count),
        stats: crate::scanner::DetectionStats::default(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_request_deserializes_with_paths_and_parallelism() {
        let json = r#"{"paths": ["/home/user/docs"], "parallelism": "light"}"#;
        let req: ScanRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.paths, vec!["/home/user/docs"]);
        assert_eq!(req.parallelism.as_deref(), Some("light"));
    }

    #[test]
    fn scan_request_deserializes_without_parallelism() {
        let json = r#"{"paths": ["/tmp"]}"#;
        let req: ScanRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.paths, vec!["/tmp"]);
        assert!(req.parallelism.is_none());
    }

    #[test]
    fn scan_response_serializes_correctly() {
        let resp = ScanResponse {
            session_id: 42,
            message: "Scan started".to_string(),
        };
        let json: serde_json::Value = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["session_id"], 42);
        assert_eq!(json["message"], "Scan started");
    }

    /// Mirrors the parallelism parsing logic from `start_scan`.
    fn parse_parallelism(input: Option<&str>) -> ParallelismMode {
        match input {
            Some("light") => ParallelismMode::Light,
            Some("aggressive") => ParallelismMode::Aggressive,
            _ => ParallelismMode::Normal,
        }
    }

    #[test]
    fn parallelism_parsing_matches_expected_modes() {
        assert!(matches!(
            parse_parallelism(Some("light")),
            ParallelismMode::Light
        ));
        assert!(matches!(
            parse_parallelism(Some("aggressive")),
            ParallelismMode::Aggressive
        ));
        assert!(matches!(parse_parallelism(None), ParallelismMode::Normal));
        assert!(matches!(
            parse_parallelism(Some("unknown")),
            ParallelismMode::Normal
        ));
    }
}
