//! Settings-related Tauri commands

use crate::db::queries;
use crate::state::AppState;
use std::sync::Mutex;
use tauri::State;

/// Get a setting value
#[tauri::command]
pub async fn get_setting(
    key: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<String>, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::settings::get(db.pool(), &key)
        .await
        .map_err(|e| e.to_string())
}

/// Set a setting value
#[tauri::command]
pub async fn set_setting(
    key: String,
    value: String,
    state: State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::settings::set(db.pool(), &key, &value)
        .await
        .map_err(|e| e.to_string())
}

/// Get all settings
#[tauri::command]
pub async fn get_all_settings(
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<crate::db::models::Setting>, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::settings::get_all(db.pool())
        .await
        .map_err(|e| e.to_string())
}
