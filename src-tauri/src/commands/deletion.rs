//! Deletion Tauri commands

use crate::db::queries;
use crate::services::deletion::{BatchDeletionResult, DeletionRequest, DeletionService};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct DeleteFilesRequest {
    pub files: Vec<DeletionRequest>,
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
            let _ = queries::deletion_history::record(
                db.pool(),
                &deleted.path,
                #[allow(clippy::cast_possible_wrap)]
                (deleted.size as i64),
                "", // Hash already verified during deletion
                None,
                None,
                None,
            )
            .await;
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

/// Get deletion history
#[tauri::command]
pub async fn get_deletion_history(
    limit: i32,
    offset: i32,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<crate::db::models::DeletionRecord>, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::deletion_history::get_history(db.pool(), limit, offset)
        .await
        .map_err(|e| e.to_string())
}
