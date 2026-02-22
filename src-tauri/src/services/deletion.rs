//! File deletion service with verification and trash support

use crate::scanner::hasher::HashError;
use crate::scanner::FileHasher;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
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

/// Phase of a batch deletion operation
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeletionPhase {
    Verifying,
    Trashing,
}

/// Progress event emitted during batch deletion
#[derive(Debug, Clone, Serialize)]
pub struct DeletionProgressEvent {
    pub phase: DeletionPhase,
    pub completed: usize,
    pub total: usize,
    pub current_path: Option<String>,
}

/// Result of the verification phase
pub struct VerificationResult {
    /// Files that passed verification (exist + hash matches)
    pub verified: Vec<DeletionRequest>,
    /// Files that failed verification
    pub failed: Vec<DeletionResult>,
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

    /// Verify a batch of files without deleting them.
    /// Returns verified files and failed files separately.
    /// Calls `on_progress(completed, total)` after each file.
    pub fn verify_batch(
        &mut self,
        requests: &[DeletionRequest],
        mut on_progress: impl FnMut(usize, usize),
    ) -> VerificationResult {
        let mut verified = Vec::new();
        let mut failed = Vec::new();
        let total = requests.len();

        for (i, request) in requests.iter().enumerate() {
            let path = Path::new(&request.path);

            match self.verify_file(path, &request.expected_hash) {
                Ok(true) => verified.push(request.clone()),
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

        VerificationResult { verified, failed }
    }

    /// Trash all given files in a single OS operation via `trash::delete_all()`.
    /// Uses the default Finder method on macOS — produces one trash sound
    /// for the entire batch and preserves "Put Back" for all files.
    pub fn trash_verified(verified: &[DeletionRequest]) -> BatchDeletionResult {
        if verified.is_empty() {
            return BatchDeletionResult {
                successful: Vec::new(),
                failed: Vec::new(),
                total_freed: 0,
            };
        }

        let paths: Vec<PathBuf> = verified.iter().map(|r| PathBuf::from(&r.path)).collect();

        match trash::delete_all(&paths) {
            Ok(()) => {
                let mut total_freed = 0u64;
                let successful = verified
                    .iter()
                    .map(|r| {
                        total_freed += r.size;
                        DeletionResult {
                            path: r.path.clone(),
                            success: true,
                            error: None,
                            size: r.size,
                        }
                    })
                    .collect();
                BatchDeletionResult {
                    successful,
                    failed: Vec::new(),
                    total_freed,
                }
            }
            Err(e) => {
                // delete_all is fail-fast — check which files were actually removed
                let mut successful = Vec::new();
                let mut failed = Vec::new();
                let mut total_freed = 0u64;

                for request in verified {
                    let path = Path::new(&request.path);
                    if path.exists() {
                        // File still exists — it failed or wasn't reached
                        failed.push(DeletionResult {
                            path: request.path.clone(),
                            success: false,
                            error: Some(e.to_string()),
                            size: request.size,
                        });
                    } else {
                        // File was successfully trashed
                        total_freed += request.size;
                        successful.push(DeletionResult {
                            path: request.path.clone(),
                            success: true,
                            error: None,
                            size: request.size,
                        });
                    }
                }

                BatchDeletionResult {
                    successful,
                    failed,
                    total_freed,
                }
            }
        }
    }

    /// Delete a single file to trash (used by tests)
    #[cfg(test)]
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

    /// Delete multiple files to trash (used by tests; production uses verify_batch + trash_verified)
    #[cfg(test)]
    pub fn delete_batch(&mut self, requests: &[DeletionRequest]) -> BatchDeletionResult {
        let mut successful = Vec::new();
        let mut failed = Vec::new();
        let mut total_freed: u64 = 0;

        for request in requests {
            let result = self.delete_to_trash(request);
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
        let batch_result = service.delete_batch(&requests);

        assert_eq!(batch_result.successful.len(), 1);
        assert_eq!(batch_result.failed.len(), 1);
        assert_eq!(batch_result.total_freed, 12);
    }

    #[test]
    fn test_verify_batch_all_valid() {
        let dir = tempdir().unwrap();
        let mut hasher = FileHasher::new();

        let path1 = dir.path().join("file1.txt");
        let path2 = dir.path().join("file2.txt");
        File::create(&path1)
            .unwrap()
            .write_all(b"content1")
            .unwrap();
        File::create(&path2)
            .unwrap()
            .write_all(b"content2")
            .unwrap();

        let hash1 = hasher.full_hash(&path1).unwrap();
        let hash2 = hasher.full_hash(&path2).unwrap();

        let requests = vec![
            DeletionRequest {
                path: path1.display().to_string(),
                expected_hash: hash1,
                size: 8,
            },
            DeletionRequest {
                path: path2.display().to_string(),
                expected_hash: hash2,
                size: 8,
            },
        ];

        let mut service = DeletionService::new();
        let result = service.verify_batch(&requests, |_, _| {});

        assert_eq!(result.verified.len(), 2);
        assert!(result.failed.is_empty());
    }

    #[test]
    fn test_verify_batch_mixed() {
        let dir = tempdir().unwrap();
        let mut hasher = FileHasher::new();

        let good_path = dir.path().join("good.txt");
        File::create(&good_path)
            .unwrap()
            .write_all(b"good")
            .unwrap();
        let good_hash = hasher.full_hash(&good_path).unwrap();

        let changed_path = dir.path().join("changed.txt");
        File::create(&changed_path)
            .unwrap()
            .write_all(b"changed")
            .unwrap();

        let missing_path = dir.path().join("missing.txt");

        let requests = vec![
            DeletionRequest {
                path: good_path.display().to_string(),
                expected_hash: good_hash,
                size: 4,
            },
            DeletionRequest {
                path: changed_path.display().to_string(),
                expected_hash: "wrong_hash".to_string(),
                size: 7,
            },
            DeletionRequest {
                path: missing_path.display().to_string(),
                expected_hash: "any".to_string(),
                size: 10,
            },
        ];

        let mut service = DeletionService::new();
        let result = service.verify_batch(&requests, |_, _| {});

        assert_eq!(result.verified.len(), 1);
        assert_eq!(result.failed.len(), 2);
        assert_eq!(result.verified[0].path, good_path.display().to_string());
    }

    #[test]
    fn test_verify_batch_progress_callback() {
        let dir = tempdir().unwrap();
        let mut hasher = FileHasher::new();

        let path = dir.path().join("file.txt");
        File::create(&path)
            .unwrap()
            .write_all(b"data")
            .unwrap();
        let hash = hasher.full_hash(&path).unwrap();

        let requests = vec![
            DeletionRequest {
                path: path.display().to_string(),
                expected_hash: hash,
                size: 4,
            },
            DeletionRequest {
                path: dir.path().join("missing.txt").display().to_string(),
                expected_hash: "any".to_string(),
                size: 10,
            },
        ];

        let mut progress_calls = Vec::new();
        let mut service = DeletionService::new();
        service.verify_batch(&requests, |completed, total| {
            progress_calls.push((completed, total));
        });

        assert_eq!(progress_calls, vec![(1, 2), (2, 2)]);
    }

    #[test]
    fn test_trash_verified_empty() {
        let result = DeletionService::trash_verified(&[]);

        assert!(result.successful.is_empty());
        assert!(result.failed.is_empty());
        assert_eq!(result.total_freed, 0);
    }
}
