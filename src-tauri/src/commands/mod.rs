//! Tauri command handlers

pub mod deletion;
pub mod protected;
pub mod scan;
pub mod settings;

// Re-export command functions for convenience
pub use deletion::*;
pub use protected::*;
pub use scan::*;
pub use settings::*;