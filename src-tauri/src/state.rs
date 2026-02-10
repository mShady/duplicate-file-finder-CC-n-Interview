//! Application state management

use crate::db::Database;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex as AsyncMutex;

/// Global application state
///
/// This struct is wrapped in `std::sync::Mutex` at the Tauri management level.
/// The database is wrapped in `Arc<AsyncMutex>` so it can be cloned out of the
/// sync mutex and used across async boundaries in command handlers.
pub struct AppState {
    /// Database connection
    pub db: Option<Arc<AsyncMutex<Database>>>,
    /// Flag indicating if a scan is currently running
    pub is_scanning: bool,
    /// Current scan session ID
    pub current_scan_id: Option<i64>,
}

impl AppState {
    /// Create a new application state (without database)
    pub fn new() -> Self {
        Self {
            db: None,
            is_scanning: false,
            current_scan_id: None,
        }
    }

    /// Initialize the database connection
    pub async fn init_database(&mut self, app_data_dir: PathBuf) -> Result<(), crate::db::DbError> {
        let db_path = app_data_dir.join("duplifind.db");
        log::info!("Initializing database at: {}", db_path.display());

        let db = Database::new(db_path).await?;
        self.db = Some(Arc::new(AsyncMutex::new(db)));

        Ok(())
    }

    /// Get a reference to the database
    pub fn database(&self) -> Option<Arc<AsyncMutex<Database>>> {
        self.db.clone()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert!(!state.is_scanning);
        assert!(state.db.is_none());
        assert!(state.current_scan_id.is_none());
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.is_scanning);
    }

    #[tokio::test]
    async fn test_init_database() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut state = AppState::new();

        let result = state.init_database(temp_dir.path().to_path_buf()).await;
        assert!(result.is_ok());
        assert!(state.db.is_some());
    }
}
