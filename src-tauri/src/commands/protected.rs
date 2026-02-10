//! Protected folders Tauri commands

use crate::db::models::ProtectedFolder;
use crate::db::queries;
use crate::state::AppState;
use std::sync::Mutex;
use tauri::State;

/// Add a protected folder
#[tauri::command]
pub async fn add_protected_folder(
    path: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<i64, String> {
    let path = path.trim().to_string();
    if path.is_empty() {
        return Err("Protected folder path cannot be empty".to_string());
    }

    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::protected_folders::add(db.pool(), &path)
        .await
        .map_err(|e| e.to_string())
}

/// Remove a protected folder
#[tauri::command]
pub async fn remove_protected_folder(
    id: i64,
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::protected_folders::remove(db.pool(), id)
        .await
        .map_err(|e| e.to_string())
}

/// Get all protected folders
#[tauri::command]
pub async fn get_protected_folders(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<ProtectedFolder>, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::protected_folders::get_all(db.pool())
        .await
        .map_err(|e| e.to_string())
}

/// Check if a path is protected
#[tauri::command]
pub async fn is_path_protected(
    path: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<bool, String> {
    let path = path.trim().to_string();
    if path.is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::protected_folders::is_protected(db.pool(), &path)
        .await
        .map_err(|e| e.to_string())
}
