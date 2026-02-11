//! File deletion service with verification and trash support

use crate::scanner::FileHasher;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeletionError {
    #[error("File not found: {0}")]
    NotFound(String),

    #[error("File changed since scan: {0}")]
    FileChanged(String),

    #[error("Protected path: {0}")]
    ProtectedPath(String),

    #[error("Trash error: {0}")]
    Trash(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionRequest {
    pub path: String,
    pub expected_hash: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeletionResult {
    pub path: String,
    pub success: bool,
    pub error: Option<String>,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchDeletionResult {
    pub successful: Vec<DeletionResult>,
    pub failed: Vec<DeletionResult>,
    pub total_freed: u64,
}

pub struct DeletionService {
    hasher: FileHasher,
}

impl DeletionService {
    pub fn new() -> Self {
        Self {
            hasher: FileHasher::new(),
        }
    }

    /// Verify file hash hasn't changed before deletion
    pub fn verify_file(&mut self, path: &Path, expected_hash: &str) -> Result<bool, DeletionError> {
        if !path.exists() {
            return Err(DeletionError::NotFound(path.display().to_string()));
        }

        match self.hasher.full_hash(path) {
            Ok(current_hash) => Ok(current_hash == expected_hash),
            Err(e) => Err(DeletionError::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))),
        }
    }

    /// Delete a single file to trash
    pub fn delete_to_trash(&mut self, request: &DeletionRequest) -> DeletionResult {
        let path = Path::new(&request.path);

        // Verify file exists
        if !path.exists() {
            return DeletionResult {
                path: request.path.clone(),
                success: false,
                error: Some("File not found".to_string()),
                size: request.size,
            };
        }

        // Verify hash matches
        match self.verify_file(path, &request.expected_hash) {
            Ok(true) => {}
            Ok(false) => {
                return DeletionResult {
                    path: request.path.clone(),
                    success: false,
                    error: Some("File changed since scan".to_string()),
                    size: request.size,
                };
            }
            Err(e) => {
                return DeletionResult {
                    path: request.path.clone(),
                    success: false,
                    error: Some(e.to_string()),
                    size: request.size,
                };
            }
        }

        // Move to trash
        match trash::delete(path) {
            Ok(()) => DeletionResult {
                path: request.path.clone(),
                success: true,
                error: None,
                size: request.size,
            },
            Err(e) => DeletionResult {
                path: request.path.clone(),
                success: false,
                error: Some(e.to_string()),
                size: request.size,
            },
        }
    }

    /// Delete multiple files to trash
    pub fn delete_batch(&mut self, requests: Vec<DeletionRequest>) -> BatchDeletionResult {
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let mut total_freed: u64 = 0;

        for request in requests {
            let result = self.delete_to_trash(&request);
            if result.success {
                total_freed += result.size;
                successful.push(result);
            } else {
                failed.push(result);
            }
        }

        BatchDeletionResult {
            successful,
            failed,
            total_freed,
        }
    }
}

impl Default for DeletionService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_verify_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"test content").unwrap();

        let mut service = DeletionService::new();
        let mut hasher = FileHasher::new();
        let hash = hasher.full_hash(&path).unwrap();

        assert!(service.verify_file(&path, &hash).unwrap());
        assert!(!service.verify_file(&path, "wrong_hash").unwrap());
    }
}
