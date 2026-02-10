//! Tauri command handlers

pub mod protected;
pub mod settings;

/// Simple greet command for testing
#[tauri::command]
pub fn greet(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "Hello! Welcome to DupliFind.".to_string();
    }
    format!("Hello, {trimmed}! Welcome to DupliFind.")
}

// Re-export command functions for convenience
pub use protected::*;
pub use settings::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let result = greet("World");
        assert_eq!(result, "Hello, World! Welcome to DupliFind.");
    }

    #[test]
    fn test_greet_empty() {
        let result = greet("");
        assert_eq!(result, "Hello! Welcome to DupliFind.");
    }

    #[test]
    fn test_greet_whitespace_only() {
        let result = greet("   ");
        assert_eq!(result, "Hello! Welcome to DupliFind.");
    }

    #[test]
    fn test_greet_with_leading_trailing_spaces() {
        let result = greet("  Alice  ");
        assert_eq!(result, "Hello, Alice! Welcome to DupliFind.");
    }
}
