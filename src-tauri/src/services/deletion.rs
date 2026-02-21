//! File deletion service with verification and trash support

use crate::scanner::hasher::HashError;
use crate::scanner::FileHasher;
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DeletionError {
    #[error("File not found: {0}")]
    NotFound(String),

    #[error("Hash error: {0}")]
    Hash(#[from] HashError),

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

        Ok(self.hasher.verify_hash(path, expected_hash)?)
    }

    /// Delete a single file to trash
    #[allow(dead_code)]
    pub fn delete_to_trash(&mut self, request: &DeletionRequest) -> DeletionResult {
        let path = Path::new(&request.path);

        // Verify file exists and hash matches (verify_file handles both checks)
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

    /// Delete multiple files to trash in a single batch operation.
    ///
    /// Calls `on_progress(verified_so_far, total)` after verifying each file's hash,
    /// then moves all verified files to the system trash with a single `trash::delete_all()`
    /// call so the OS trash notification/sound plays exactly once.
    pub fn delete_batch<F>(&mut self, requests: &[DeletionRequest], mut on_progress: F) -> BatchDeletionResult
    where
        F: FnMut(usize, usize),
    {
        let total = requests.len();
        let mut verified: Vec<(&DeletionRequest, std::path::PathBuf)> = Vec::new();
        let mut failed: Vec<DeletionResult> = Vec::new();

        // Phase 1: verify all file hashes without modifying anything
        for (i, request) in requests.iter().enumerate() {
            let path = Path::new(&request.path);
            match self.verify_file(path, &request.expected_hash) {
                Ok(true) => verified.push((request, path.to_path_buf())),
                Ok(false) => failed.push(DeletionResult {
                    path: request.path.clone(),
                    success: false,
                    error: Some("File changed since scan".to_string()),
                    size: request.size,
                }),
                Err(e) => failed.push(DeletionResult {
                    path: request.path.clone(),
                    success: false,
                    error: Some(e.to_string()),
                    size: request.size,
                }),
            }
            on_progress(i + 1, total);
        }

        // Phase 2: single batch trash operation (triggers one OS notification)
        let mut successful: Vec<DeletionResult> = Vec::new();
        let mut total_freed: u64 = 0;

        if !verified.is_empty() {
            let paths: Vec<&std::path::PathBuf> = verified.iter().map(|(_, p)| p).collect();
            match trash::delete_all(&paths) {
                Ok(()) => {
                    for (req, _) in &verified {
                        total_freed += req.size;
                        successful.push(DeletionResult {
                            path: req.path.clone(),
                            success: true,
                            error: None,
                            size: req.size,
                        });
                    }
                }
                Err(_) => {
                    // Fallback: delete individually to identify which files failed
                    for (req, path) in &verified {
                        match trash::delete(path) {
                            Ok(()) => {
                                total_freed += req.size;
                                successful.push(DeletionResult {
                                    path: req.path.clone(),
                                    success: true,
                                    error: None,
                                    size: req.size,
                                });
                            }
                            Err(e) => failed.push(DeletionResult {
                                path: req.path.clone(),
                                success: false,
                                error: Some(e.to_string()),
                                size: req.size,
                            }),
                        }
                    }
                }
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
    fn test_verify_file_matching_hash() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"test content").unwrap();

        let mut service = DeletionService::new();
        let mut hasher = FileHasher::new();
        let hash = hasher.full_hash(&path).unwrap();

        assert!(service.verify_file(&path, &hash).unwrap());
    }

    #[test]
    fn test_verify_file_wrong_hash() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"test content").unwrap();

        let mut service = DeletionService::new();

        assert!(!service.verify_file(&path, "wrong_hash").unwrap());
    }

    #[test]
    fn test_verify_file_not_found() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nonexistent.txt");

        let mut service = DeletionService::new();
        let result = service.verify_file(&path, "any_hash");

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DeletionError::NotFound(_)));
    }

    #[test]
    fn test_delete_to_trash_missing_file() {
        let dir = tempdir().unwrap();
        let request = DeletionRequest {
            path: dir.path().join("nonexistent.txt").display().to_string(),
            expected_hash: "any_hash".to_string(),
            size: 100,
        };

        let mut service = DeletionService::new();
        let result = service.delete_to_trash(&request);

        assert!(!result.success);
        assert!(result
            .error
            .as_deref()
            .unwrap()
            .contains("File not found"));
    }

    #[test]
    fn test_delete_to_trash_hash_mismatch() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"test content").unwrap();

        let request = DeletionRequest {
            path: path.display().to_string(),
            expected_hash: "wrong_hash".to_string(),
            size: 12,
        };

        let mut service = DeletionService::new();
        let result = service.delete_to_trash(&request);

        assert!(!result.success);
        assert_eq!(result.error.as_deref(), Some("File changed since scan"));
        // File should still exist after failed deletion
        assert!(path.exists());
    }

    #[test]
    fn test_delete_to_trash_success() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("to_delete.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"delete me").unwrap();

        let mut hasher = FileHasher::new();
        let hash = hasher.full_hash(&path).unwrap();

        let request = DeletionRequest {
            path: path.display().to_string(),
            expected_hash: hash,
            size: 9,
        };

        let mut service = DeletionService::new();
        let result = service.delete_to_trash(&request);

        assert!(result.success);
        assert!(result.error.is_none());
        assert!(!path.exists());
    }

    #[test]
    fn test_delete_batch_mixed_results() {
        let dir = tempdir().unwrap();

        // Create a file that will succeed
        let good_path = dir.path().join("good.txt");
        let mut file = File::create(&good_path).unwrap();
        file.write_all(b"good content").unwrap();

        let mut hasher = FileHasher::new();
        let good_hash = hasher.full_hash(&good_path).unwrap();

        let requests = vec![
            DeletionRequest {
                path: good_path.display().to_string(),
                expected_hash: good_hash,
                size: 12,
            },
            DeletionRequest {
                path: dir.path().join("missing.txt").display().to_string(),
                expected_hash: "any".to_string(),
                size: 50,
            },
        ];

        let mut service = DeletionService::new();
        let batch_result = service.delete_batch(&requests, |_, _| {});

        assert_eq!(batch_result.successful.len(), 1);
        assert_eq!(batch_result.failed.len(), 1);
        assert_eq!(batch_result.total_freed, 12);
    }
}
