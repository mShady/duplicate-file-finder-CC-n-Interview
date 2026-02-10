//! Database models representing table structures
//!
//! NOTE: Some models and methods in this module are marked as used in future phases:
//! - `ScannedFile`, `DuplicateGroup`: Phase 4 (Duplicate Detection)
//! - `ScanSession` queries: Phase 4 (Scan Results Display)
//! - `DeletionRecord`: Phase 5 (Deletion Tracking & History)
//! - `FileCache`: Phase 4 (Incremental Scanning)

#![allow(dead_code)] // Models/queries used in future phases

use serde::{Deserialize, Serialize};
use std::str::FromStr;

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
}

impl FromStr for ScanStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(ScanStatus::Running),
            "paused" => Ok(ScanStatus::Paused),
            "completed" => Ok(ScanStatus::Completed),
            "cancelled" => Ok(ScanStatus::Cancelled),
            "failed" => Ok(ScanStatus::Failed),
            other => Err(format!("Unknown scan status: {other}")),
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
    pub group_id: Option<i64>,
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
        assert_eq!("running".parse::<ScanStatus>(), Ok(ScanStatus::Running));
        assert!("invalid".parse::<ScanStatus>().is_err());
    }

    #[test]
    fn test_scan_status_roundtrip() {
        let statuses = [
            ScanStatus::Running,
            ScanStatus::Paused,
            ScanStatus::Completed,
            ScanStatus::Cancelled,
            ScanStatus::Failed,
        ];
        for status in statuses {
            let s = status.as_str();
            let parsed: ScanStatus = s.parse().unwrap();
            assert_eq!(parsed, status);
        }
    }
}
