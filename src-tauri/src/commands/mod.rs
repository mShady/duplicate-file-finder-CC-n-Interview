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

/// Trims whitespace and rejects empty strings with a descriptive error.
pub(crate) fn validate_non_empty(input: &str, label: &str) -> Result<String, String> {
    let trimmed = input.trim().to_string();
    if trimmed.is_empty() {
        return Err(format!("{label} cannot be empty"));
    }
    Ok(trimmed)
}

/// Maximum number of records per page for paginated queries.
pub(crate) const MAX_PAGE_SIZE: i32 = 1000;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_non_empty_rejects_whitespace_only() {
        assert!(validate_non_empty("   ", "Key").is_err());
    }

    #[test]
    fn validate_non_empty_rejects_empty_string() {
        assert!(validate_non_empty("", "Path").is_err());
    }

    #[test]
    fn validate_non_empty_trims_and_returns() {
        assert_eq!(validate_non_empty("  theme  ", "Key").unwrap(), "theme");
    }

    #[test]
    fn validate_non_empty_error_includes_label() {
        let err = validate_non_empty("", "Setting key").unwrap_err();
        assert_eq!(err, "Setting key cannot be empty");
    }

    #[test]
    fn max_page_size_clamps_correctly() {
        assert_eq!(5000_i32.clamp(0, MAX_PAGE_SIZE), MAX_PAGE_SIZE);
        assert_eq!((-5_i32).clamp(0, MAX_PAGE_SIZE), 0);
        assert_eq!(50_i32.clamp(0, MAX_PAGE_SIZE), 50);
    }
}