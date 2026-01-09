//! Application state management

/// Global application state
pub struct AppState {
    /// Flag indicating if a scan is currently running
    pub is_scanning: bool,
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            is_scanning: false,
        }
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
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.is_scanning);
    }
}
