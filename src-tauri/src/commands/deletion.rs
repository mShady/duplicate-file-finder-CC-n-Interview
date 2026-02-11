//! Deletion Tauri commands

use crate::db::queries;
use crate::services::deletion::{BatchDeletionResult, DeletionRequest, DeletionService};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct DeleteFilesRequest {
    pub files: Vec<DeletionRequest>,
    /// Maps deleted file path -> path of the retained duplicate copy (if any)
    #[serde(default)]
    pub kept_paths: HashMap<String, String>,
    /// Maps deleted file path -> duplicate_groups.id for deletion history
    #[serde(default)]
    pub group_ids: HashMap<String, i64>,
}

#[derive(Debug, Serialize)]
pub struct DeleteFilesResponse {
    pub result: BatchDeletionResult,
    pub message: String,
}

/// Delete files to trash
#[tauri::command]
pub async fn delete_files(
    request: DeleteFilesRequest,
    state: State<'_, Mutex<AppState>>,
) -> Result<DeleteFilesResponse, String> {
    if request.files.is_empty() {
        return Err("No files to delete".to_string());
    }

    // Get database handle
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    // Check for protected paths
    {
        let db = db.lock().await;
        for file in &request.files {
            let is_protected = queries::protected_folders::is_protected(db.pool(), &file.path)
                .await
                .map_err(|e| e.to_string())?;

            if is_protected {
                return Err(format!("Cannot delete protected file: {}", file.path));
            }
        }
    }

    // Build a path-to-hash lookup so we can record the verified hash in history
    // after spawn_blocking consumes the files vector.
    let hash_lookup: HashMap<String, String> = request
        .files
        .iter()
        .map(|f| (f.path.clone(), f.expected_hash.clone()))
        .collect();

    let kept_paths = request.kept_paths;
    let group_ids = request.group_ids;

    // Perform deletion (blocking I/O, run on blocking thread)
    let files = request.files;
    let result = tokio::task::spawn_blocking(move || {
        let mut service = DeletionService::new();
        service.delete_batch(&files)
    })
    .await
    .map_err(|e| e.to_string())?;

    // Record deletions in history
    {
        let db = db.lock().await;
        for deleted in &result.successful {
            let hash = hash_lookup.get(&deleted.path).map_or("", String::as_str);
            let kept = kept_paths.get(&deleted.path).map(String::as_str);
            let group_id = group_ids.get(&deleted.path).copied();
            let size_i64 = i64::try_from(deleted.size).unwrap_or(i64::MAX);
            if let Err(e) = queries::deletion_history::record(
                db.pool(),
                &deleted.path,
                size_i64,
                hash,
                group_id,
                None,
                None,
                kept,
            )
            .await
            {
                log::warn!("Failed to record deletion history for {}: {}", deleted.path, e);
            }
        }
    }

    let message = format!(
        "Deleted {} files, freed {} bytes. {} failed.",
        result.successful.len(),
        result.total_freed,
        result.failed.len()
    );

    Ok(DeleteFilesResponse { result, message })
}

/// Get deletion history summary (total count and total freed space)
#[tauri::command]
pub async fn get_deletion_history_summary(
    state: State<'_, Mutex<AppState>>,
) -> Result<(i64, i64), String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::deletion_history::get_summary(db.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Get deletion history
#[tauri::command]
pub async fn get_deletion_history(
    limit: i32,
    offset: i32,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<crate::db::models::DeletionRecord>, String> {
    // Validate pagination parameters
    let limit = limit.clamp(0, 1000);
    let offset = offset.max(0);

    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::deletion_history::get_history(db.pool(), limit, offset)
        .await
        .map_err(|e| e.to_string())
}
