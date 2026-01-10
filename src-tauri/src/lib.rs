// DupliFind - Main library entry point
#![warn(clippy::all, clippy::pedantic)]

mod commands;
mod state;

use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize application state
            let state = AppState::new();
            app.manage(Mutex::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
