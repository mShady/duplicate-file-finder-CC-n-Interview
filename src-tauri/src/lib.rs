// DupliFind - Main library entry point
#![warn(clippy::all, clippy::pedantic)]

mod commands;
mod db;
mod scanner;
mod services;
mod state;

use commands::scan::ScanState;
use state::AppState;
use std::sync::Mutex;
use tauri::Manager;

/// Run the Tauri application.
///
/// # Panics
///
/// Panics if the Tauri application fails to initialize or run,
/// or if the database cannot be initialized.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Initialize logging
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Get app data directory
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            log::info!("App data directory: {}", app_data_dir.display());

            // Initialize application state with database
            // We use block_on here since setup is synchronous
            let state = tauri::async_runtime::block_on(async {
                let mut state = AppState::new();
                state
                    .init_database(app_data_dir)
                    .await
                    .expect("Failed to initialize database");
                state
            });

            app.manage(Mutex::new(state));

            // Initialize scan state
            app.manage(Mutex::new(ScanState::new()));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            // Settings commands
            commands::get_setting,
            commands::set_setting,
            commands::get_all_settings,
            // Protected folders commands
            commands::add_protected_folder,
            commands::remove_protected_folder,
            commands::get_protected_folders,
            commands::is_path_protected,
            // Scan commands
            commands::start_scan,
            commands::cancel_scan,
            commands::get_scan_progress,
            commands::is_scanning,
            commands::get_scan_results,
            // Deletion commands
            commands::delete_files,
            commands::get_deletion_history,
            commands::get_deletion_history_summary,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
