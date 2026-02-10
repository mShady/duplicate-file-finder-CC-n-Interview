//! Application state management

use std::sync::atomic::{AtomicBool, Ordering};

/// Global application state
/// Uses atomic types for lock-free thread-safe access
pub struct AppState {
    /// Flag indicating if a scan is currently running
    is_scanning: AtomicBool,
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            is_scanning: AtomicBool::new(false),
        }
    }

    /// Check if a scan is currently running
    /// (Will be used by scan commands in future phases)
    #[allow(dead_code)]
    pub fn is_scanning(&self) -> bool {
        // Acquire ordering ensures we see all writes that happened before the flag was set.
        // This is important if is_scanning ever guards access to other shared state.
        self.is_scanning.load(Ordering::Acquire)
    }

    /// Set the scanning state
    /// (Will be used by scan commands in future phases)
    #[allow(dead_code)]
    pub fn set_scanning(&self, value: bool) {
        // Release ordering ensures all prior writes are visible to threads that subsequently
        // observe this flag change via Acquire load.
        self.is_scanning.store(value, Ordering::Release);
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
        assert!(!state.is_scanning());
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.is_scanning());
    }

    #[test]
    fn test_app_state_set_scanning() {
        let state = AppState::new();
        assert!(!state.is_scanning());
        state.set_scanning(true);
        assert!(state.is_scanning());
        state.set_scanning(false);
        assert!(!state.is_scanning());
    }
}
