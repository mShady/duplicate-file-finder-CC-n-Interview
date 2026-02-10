//! Database models representing table structures

use serde::{Deserialize, Serialize};

/// Represents a scanned file in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedFile {
    pub id: i64,
    pub path: String,
    pub size: i64,
    pub partial_hash: Option<String>,
    pub full_hash: Option<String>,
    pub created_at: i64,
    pub modified_at: i64,
    pub scanned_at: i64,
    pub group_id: Option<i64>,
}

/// Represents a group of duplicate files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub id: i64,
    pub hash: String,
    pub file_size: i64,
    pub file_count: i32,
    pub wasted_space: i64,
    pub created_at: i64,
}

/// Represents a scan session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSession {
    pub id: i64,
    pub started_at: i64,
    pub completed_at: Option<i64>,
    pub status: String,
    pub scanned_paths: String,
    pub total_files: i64,
    pub total_size: i64,
    pub duplicate_groups: i32,
    pub wasted_space: i64,
}

/// Scan session status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Running,
    Paused,
    Completed,
    Cancelled,
    Failed,
}

impl ScanStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "running" => Some(ScanStatus::Running),
            "paused" => Some(ScanStatus::Paused),
            "completed" => Some(ScanStatus::Completed),
            "cancelled" => Some(ScanStatus::Cancelled),
            "failed" => Some(ScanStatus::Failed),
            _ => None,
        }
    }
}

/// Represents a user setting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

/// Represents a protected folder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtectedFolder {
    pub id: i64,
    pub path: String,
    pub added_at: i64,
}

/// Represents a deletion record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionRecord {
    pub id: i64,
    pub file_path: String,
    pub file_size: i64,
    pub file_hash: String,
    pub deleted_at: i64,
    pub group_id: i64,
}

/// Represents a file in the hash cache (for incremental scanning)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCache {
    pub id: i64,
    pub path: String,
    pub size: i64,
    pub modified_at: i64,
    pub partial_hash: String,
    pub full_hash: Option<String>,
    pub cached_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_status_conversion() {
        assert_eq!(ScanStatus::Running.as_str(), "running");
        assert_eq!(ScanStatus::from_str("running"), Some(ScanStatus::Running));
        assert_eq!(ScanStatus::from_str("invalid"), None);
    }
}
