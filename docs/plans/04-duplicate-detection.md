# File 04: Duplicate Detection

## Overview

This file covers the multi-stage duplicate detection algorithm using BLAKE3 hashing. The algorithm uses a three-stage approach for efficiency:

1. Group files by size (files must have identical sizes to be duplicates)
2. Partial hash comparison (first/last 4KB) for quick elimination
3. Full content hash only when partial hashes match

By the end of this file, you'll have a working duplicate detection system that efficiently identifies identical files.

## Prerequisites

- Completed File 01 (Project Foundation)
- Completed File 02 (Database Foundation)
- Completed File 03 (File Scanning Core)

---

## Phase 4.1: Add BLAKE3 Dependency

### Overview

Add the BLAKE3 crate for high-performance, secure hashing.

### Changes Required

#### 4.1.1 Update Cargo.toml

**File**: `src-tauri/Cargo.toml`

Add BLAKE3 to the dependencies:

```toml
[dependencies]
# ... existing dependencies ...

# Hashing
blake3 = { version = "1.8", features = ["rayon"] }
```

### Success Criteria

#### Automated Verification

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes

#### Manual Verification

- [ ] BLAKE3 version is 1.8.x or later

### Commit

Execute `/cl:commit` to commit changes with meaningful message.

### Code Review

## Run code-review-fix-loop agent on `src-tauri/Cargo.toml`.

## Phase 4.2: Create Hasher Module

### Overview

Create the hasher module that handles partial and full file hashing using BLAKE3.

### Changes Required

#### 4.2.1 Create Hasher Module

**File**: `src-tauri/src/scanner/hasher.rs`

```rust
//! File hashing module using BLAKE3

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;
use thiserror::Error;

/// Size of partial hash chunks (4KB from start and end)
const PARTIAL_HASH_CHUNK_SIZE: u64 = 4096;

/// Minimum file size for partial hashing to be meaningful
/// Files smaller than this will use full hash directly
const MIN_SIZE_FOR_PARTIAL: u64 = PARTIAL_HASH_CHUNK_SIZE * 2;

/// Hash-related errors
#[derive(Error, Debug)]
pub enum HashError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("File not found: {0}")]
    NotFound(String),

    #[error("File changed during hashing")]
    FileChanged,
}

/// Result of hashing a file
#[derive(Debug, Clone)]
pub struct HashResult {
    /// Partial hash (first 4KB + last 4KB)
    pub partial_hash: String,
    /// Full content hash (only computed if needed)
    pub full_hash: Option<String>,
    /// File size at time of hashing
    pub size: u64,
}

/// File hasher using BLAKE3
pub struct FileHasher {
    /// Buffer for reading file chunks
    buffer: Vec<u8>,
}

impl FileHasher {
    /// Create a new file hasher
    pub fn new() -> Self {
        Self {
            buffer: vec![0u8; 65536], // 64KB buffer for full hashing
        }
    }

    /// Compute partial hash of a file (first 4KB + last 4KB)
    ///
    /// For files smaller than 8KB, this returns the full content hash.
    pub fn partial_hash<P: AsRef<Path>>(&mut self, path: P) -> Result<String, HashError> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();

        // For empty files, return a special hash
        if size == 0 {
            return Ok(blake3::hash(b"").to_hex().to_string());
        }

        // For small files, just hash the whole thing
        if size < MIN_SIZE_FOR_PARTIAL {
            return self.full_hash(path);
        }

        let mut hasher = blake3::Hasher::new();

        // Read first chunk
        let mut first_chunk = vec![0u8; PARTIAL_HASH_CHUNK_SIZE as usize];
        file.read_exact(&mut first_chunk)?;
        hasher.update(&first_chunk);

        // Seek to last chunk
        file.seek(SeekFrom::End(-(PARTIAL_HASH_CHUNK_SIZE as i64)))?;

        // Read last chunk
        let mut last_chunk = vec![0u8; PARTIAL_HASH_CHUNK_SIZE as usize];
        file.read_exact(&mut last_chunk)?;
        hasher.update(&last_chunk);

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Compute full content hash of a file
    pub fn full_hash<P: AsRef<Path>>(&mut self, path: P) -> Result<String, HashError> {
        let path = path.as_ref();
        let mut file = File::open(path)?;
        let metadata = file.metadata()?;

        // For empty files, return a consistent hash
        if metadata.len() == 0 {
            return Ok(blake3::hash(b"").to_hex().to_string());
        }

        let mut hasher = blake3::Hasher::new();

        loop {
            let bytes_read = file.read(&mut self.buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&self.buffer[..bytes_read]);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Compute full hash with parallel processing for large files
    ///
    /// Uses BLAKE3's rayon feature for files larger than 1MB
    pub fn full_hash_parallel<P: AsRef<Path>>(&self, path: P) -> Result<String, HashError> {
        let path = path.as_ref();
        let data = std::fs::read(path)?;

        // For empty files
        if data.is_empty() {
            return Ok(blake3::hash(b"").to_hex().to_string());
        }

        // Use parallel hashing for large files
        let mut hasher = blake3::Hasher::new();
        hasher.update_rayon(&data);

        Ok(hasher.finalize().to_hex().to_string())
    }

    /// Compute both partial and full hash in one pass for small files,
    /// or partial only for large files
    pub fn compute_hashes<P: AsRef<Path>>(
        &mut self,
        path: P,
        compute_full: bool,
    ) -> Result<HashResult, HashError> {
        let path = path.as_ref();
        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();

        let partial_hash = self.partial_hash(path)?;

        let full_hash = if compute_full || size < MIN_SIZE_FOR_PARTIAL {
            // For small files, partial_hash already returns full hash
            if size < MIN_SIZE_FOR_PARTIAL {
                Some(partial_hash.clone())
            } else {
                Some(self.full_hash(path)?)
            }
        } else {
            None
        };

        Ok(HashResult {
            partial_hash,
            full_hash,
            size,
        })
    }

    /// Verify a file still has the expected hash
    pub fn verify_hash<P: AsRef<Path>>(&mut self, path: P, expected_hash: &str) -> Result<bool, HashError> {
        let current_hash = self.full_hash(path)?;
        Ok(current_hash == expected_hash)
    }
}

impl Default for FileHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute hash of raw data (utility function)
pub fn hash_data(data: &[u8]) -> String {
    blake3::hash(data).to_hex().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_file(dir: &Path, name: &str, content: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();
        path
    }

    #[test]
    fn test_empty_file_hash() {
        let dir = tempdir().unwrap();
        let path = create_test_file(dir.path(), "empty.txt", b"");

        let mut hasher = FileHasher::new();
        let hash = hasher.full_hash(&path).unwrap();

        // Empty file should have consistent hash
        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64); // BLAKE3 produces 256-bit (64 hex chars) hash
    }

    #[test]
    fn test_small_file_hash() {
        let dir = tempdir().unwrap();
        let content = b"Hello, World!";
        let path = create_test_file(dir.path(), "small.txt", content);

        let mut hasher = FileHasher::new();
        let partial = hasher.partial_hash(&path).unwrap();
        let full = hasher.full_hash(&path).unwrap();

        // For small files, partial and full should be the same
        assert_eq!(partial, full);
    }

    #[test]
    fn test_large_file_partial_hash() {
        let dir = tempdir().unwrap();

        // Create a file larger than 8KB
        let mut content = vec![0u8; 16384]; // 16KB
        content[0] = 1; // Modify first byte
        content[16383] = 2; // Modify last byte

        let path = create_test_file(dir.path(), "large.bin", &content);

        let mut hasher = FileHasher::new();
        let partial = hasher.partial_hash(&path).unwrap();
        let full = hasher.full_hash(&path).unwrap();

        // For large files, partial and full should be different
        assert_ne!(partial, full);
    }

    #[test]
    fn test_identical_files_same_hash() {
        let dir = tempdir().unwrap();
        let content = b"Identical content for testing";

        let path1 = create_test_file(dir.path(), "file1.txt", content);
        let path2 = create_test_file(dir.path(), "file2.txt", content);

        let mut hasher = FileHasher::new();
        let hash1 = hasher.full_hash(&path1).unwrap();
        let hash2 = hasher.full_hash(&path2).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_different_files_different_hash() {
        let dir = tempdir().unwrap();

        let path1 = create_test_file(dir.path(), "file1.txt", b"Content A");
        let path2 = create_test_file(dir.path(), "file2.txt", b"Content B");

        let mut hasher = FileHasher::new();
        let hash1 = hasher.full_hash(&path1).unwrap();
        let hash2 = hasher.full_hash(&path2).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_verify_hash() {
        let dir = tempdir().unwrap();
        let content = b"Test content for verification";
        let path = create_test_file(dir.path(), "verify.txt", content);

        let mut hasher = FileHasher::new();
        let hash = hasher.full_hash(&path).unwrap();

        // Verify with correct hash
        assert!(hasher.verify_hash(&path, &hash).unwrap());

        // Verify with wrong hash
        assert!(!hasher.verify_hash(&path, "wrong_hash").unwrap());
    }

    #[test]
    fn test_compute_hashes() {
        let dir = tempdir().unwrap();
        let content = b"Content for compute_hashes test";
        let path = create_test_file(dir.path(), "compute.txt", content);

        let mut hasher = FileHasher::new();

        // Without full hash
        let result = hasher.compute_hashes(&path, false).unwrap();
        assert!(!result.partial_hash.is_empty());
        // For small files, full hash is included anyway
        assert!(result.full_hash.is_some());

        // With full hash
        let result = hasher.compute_hashes(&path, true).unwrap();
        assert!(result.full_hash.is_some());
    }

    #[test]
    fn test_hash_data() {
        let data = b"Test data";
        let hash = hash_data(data);

        assert!(!hash.is_empty());
        assert_eq!(hash.len(), 64);

        // Same data should produce same hash
        assert_eq!(hash, hash_data(data));
    }

    #[test]
    fn test_partial_hash_different_middle() {
        let dir = tempdir().unwrap();

        // Create two files with same start/end but different middle
        let mut content1 = vec![0u8; 16384];
        let mut content2 = vec![0u8; 16384];

        // Same first 4KB
        for i in 0..4096 {
            content1[i] = (i % 256) as u8;
            content2[i] = (i % 256) as u8;
        }

        // Different middle
        content1[8000] = 1;
        content2[8000] = 2;

        // Same last 4KB
        for i in 12288..16384 {
            content1[i] = ((i - 12288) % 256) as u8;
            content2[i] = ((i - 12288) % 256) as u8;
        }

        let path1 = create_test_file(dir.path(), "middle1.bin", &content1);
        let path2 = create_test_file(dir.path(), "middle2.bin", &content2);

        let mut hasher = FileHasher::new();
        let partial1 = hasher.partial_hash(&path1).unwrap();
        let partial2 = hasher.partial_hash(&path2).unwrap();
        let full1 = hasher.full_hash(&path1).unwrap();
        let full2 = hasher.full_hash(&path2).unwrap();

        // Partial hashes should be the same (same start/end)
        assert_eq!(partial1, partial2);

        // Full hashes should be different
        assert_ne!(full1, full2);
    }
}
```

#### 4.2.2 Update Scanner Module

**File**: `src-tauri/src/scanner/mod.rs`

```rust
//! File scanning module

pub mod hasher;
pub mod types;
pub mod walker;

#[cfg(test)]
mod tests;

pub use hasher::{FileHasher, HashError, HashResult};
pub use types::*;
pub use walker::DirectoryWalker;
```

### Success Criteria

#### Automated Verification

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml hasher` passes all hasher tests

#### Manual Verification

- [ ] Hasher correctly produces BLAKE3 hashes
- [ ] Partial hash covers first/last 4KB

### Commit

Execute `/cl:commit` to commit changes with meaningful message.

### Code Review

## Run code-review-fix-loop agent on `src-tauri/src/scanner/hasher.rs`.

## Phase 4.2.3: Empty File Handling Specification

### Overview

Zero-byte files require special handling since they all have identical content (nothing). This section documents the explicit empty file grouping behavior.

### Empty File Behavior

#### Design Decision

All empty files (size = 0 bytes) are **grouped as duplicates** because:

1. They have identical content (empty)
2. They produce identical hashes (`blake3::hash(b"")`)
3. They waste disk space through inode overhead

#### Implementation Details

The hasher already handles this correctly in Phase 4.2:

```rust
// In FileHasher::partial_hash() and FileHasher::full_hash()
// For empty files, return a special hash
if size == 0 {
    return Ok(blake3::hash(b"").to_hex().to_string());
}
```

**Empty file hash value** (BLAKE3 of empty byte array):

```
af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262
```

All empty files will:

1. Be grouped together in Stage 1 (same size: 0)
2. Have identical partial hash in Stage 2
3. Have identical full hash in Stage 3
4. Form a single duplicate group

#### Wasted Space Calculation

For empty files, wasted space is technically 0 bytes, but the UI should still display the group to help users clean up unnecessary empty files. The duplicate count reflects actual redundant files.

```rust
// In DuplicateGroup::new()
let wasted_space = if files.len() > 1 {
    file_size * (files.len() as u64 - 1)  // 0 * N = 0 for empty files
} else {
    0
};
```

#### Test Coverage

Add the following test to `detector.rs` (in Phase 4.7):

```rust
#[test]
fn test_empty_files_are_duplicates() {
    let dir = tempdir().unwrap();

    let files = vec![
        create_test_file(dir.path(), "empty1.txt", b""),
        create_test_file(dir.path(), "empty2.txt", b""),
        create_test_file(dir.path(), "empty3.txt", b""),
    ];

    let mut detector = DuplicateDetector::new();
    let result = detector.detect(files).unwrap();

    // All empty files should be in one group
    assert_eq!(result.groups.len(), 1);
    assert_eq!(result.groups[0].files.len(), 3);
    assert_eq!(result.groups[0].file_size, 0);
    assert_eq!(result.total_wasted_space, 0); // 0 bytes per file
}
```

---

## Phase 4.3: Create Duplicate Detector

### Overview

Create the duplicate detector that groups files by size, then by partial hash, then verifies with full hash.

### Changes Required

#### 4.3.1 Create Detector Module

**File**: `src-tauri/src/scanner/detector.rs`

```rust
//! Duplicate file detection module

use super::hasher::FileHasher;
use super::types::{FileInfo, ScanError};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// A group of duplicate files
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    /// Unique identifier for this group
    pub id: u64,
    /// The full content hash shared by all files in this group
    pub hash: String,
    /// Size of each file in this group
    pub file_size: u64,
    /// Files in this group
    pub files: Vec<DuplicateFile>,
    /// Total wasted space (size * (count - 1))
    pub wasted_space: u64,
}

impl DuplicateGroup {
    /// Create a new duplicate group
    pub fn new(id: u64, hash: String, file_size: u64, files: Vec<DuplicateFile>) -> Self {
        let wasted_space = if files.len() > 1 {
            file_size * (files.len() as u64 - 1)
        } else {
            0
        };

        Self {
            id,
            hash,
            file_size,
            files,
            wasted_space,
        }
    }

    /// Get the number of files in this group
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get the "original" file (oldest by creation date)
    pub fn original(&self) -> Option<&DuplicateFile> {
        self.files.iter().min_by_key(|f| f.created_at)
    }
}

/// A file within a duplicate group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateFile {
    /// File path
    pub path: PathBuf,
    /// File size
    pub size: u64,
    /// Creation timestamp
    pub created_at: i64,
    /// Modification timestamp
    pub modified_at: i64,
    /// Whether this is the "original" (oldest)
    pub is_original: bool,
}

impl From<FileInfo> for DuplicateFile {
    fn from(info: FileInfo) -> Self {
        Self {
            path: info.path,
            size: info.size,
            created_at: info.created_at,
            modified_at: info.modified_at,
            is_original: false, // Set later when group is formed
        }
    }
}

/// Duplicate detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Groups of duplicate files
    pub groups: Vec<DuplicateGroup>,
    /// Total number of duplicate files (excluding originals)
    pub duplicate_count: u64,
    /// Total wasted space
    pub total_wasted_space: u64,
    /// Number of unique files (no duplicates)
    pub unique_files: u64,
    /// Detection statistics
    pub stats: DetectionStats,
}

/// Detection statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectionStats {
    /// Files grouped by size
    pub size_groups: u64,
    /// Files that passed size grouping
    pub size_candidates: u64,
    /// Partial hashes computed
    pub partial_hashes: u64,
    /// Full hashes computed
    pub full_hashes: u64,
    /// Time spent in each stage (ms)
    pub size_grouping_ms: u64,
    pub partial_hashing_ms: u64,
    pub full_hashing_ms: u64,
}

/// Duplicate file detector
pub struct DuplicateDetector {
    /// File hasher
    hasher: FileHasher,
    /// Cancellation flag
    cancelled: Arc<AtomicBool>,
    /// Next group ID
    next_group_id: u64,
}

impl DuplicateDetector {
    /// Create a new duplicate detector
    pub fn new() -> Self {
        Self {
            hasher: FileHasher::new(),
            cancelled: Arc::new(AtomicBool::new(false)),
            next_group_id: 1,
        }
    }

    /// Get a cancellation handle
    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    /// Detect duplicates from a list of files
    pub fn detect(&mut self, files: Vec<FileInfo>) -> Result<DetectionResult, ScanError> {
        let mut stats = DetectionStats::default();

        // Stage 1: Group by size
        let start = std::time::Instant::now();
        let size_groups = self.group_by_size(files);
        stats.size_grouping_ms = start.elapsed().as_millis() as u64;
        stats.size_groups = size_groups.len() as u64;

        // Filter to only groups with multiple files
        let size_candidates: Vec<_> = size_groups
            .into_iter()
            .filter(|(_, files)| files.len() > 1)
            .collect();
        stats.size_candidates = size_candidates.iter().map(|(_, f)| f.len() as u64).sum();

        if self.cancelled.load(Ordering::Relaxed) {
            return Err(ScanError::Cancelled);
        }

        // Stage 2: Group by partial hash within size groups
        let start = std::time::Instant::now();
        let partial_groups = self.group_by_partial_hash(size_candidates, &mut stats)?;
        stats.partial_hashing_ms = start.elapsed().as_millis() as u64;

        if self.cancelled.load(Ordering::Relaxed) {
            return Err(ScanError::Cancelled);
        }

        // Stage 3: Verify with full hash
        let start = std::time::Instant::now();
        let duplicate_groups = self.verify_with_full_hash(partial_groups, &mut stats)?;
        stats.full_hashing_ms = start.elapsed().as_millis() as u64;

        // Calculate totals
        let duplicate_count: u64 = duplicate_groups
            .iter()
            .map(|g| (g.files.len() - 1) as u64)
            .sum();
        let total_wasted_space: u64 = duplicate_groups.iter().map(|g| g.wasted_space).sum();
        let unique_files = stats.size_candidates - duplicate_count;

        Ok(DetectionResult {
            groups: duplicate_groups,
            duplicate_count,
            total_wasted_space,
            unique_files,
            stats,
        })
    }

    /// Stage 1: Group files by size
    fn group_by_size(&self, files: Vec<FileInfo>) -> HashMap<u64, Vec<FileInfo>> {
        let mut groups: HashMap<u64, Vec<FileInfo>> = HashMap::new();

        for file in files {
            // Skip symlinks
            if file.is_symlink {
                continue;
            }

            groups.entry(file.size).or_default().push(file);
        }

        groups
    }

    /// Stage 2: Group by partial hash within size groups
    fn group_by_partial_hash(
        &mut self,
        size_groups: Vec<(u64, Vec<FileInfo>)>,
        stats: &mut DetectionStats,
    ) -> Result<Vec<(String, u64, Vec<FileInfo>)>, ScanError> {
        let mut partial_groups: HashMap<(String, u64), Vec<FileInfo>> = HashMap::new();

        for (_size, files) in size_groups {
            for file in files {
                if self.cancelled.load(Ordering::Relaxed) {
                    return Err(ScanError::Cancelled);
                }

                match self.hasher.partial_hash(&file.path) {
                    Ok(hash) => {
                        stats.partial_hashes += 1;
                        let key = (hash, file.size);
                        partial_groups.entry(key).or_default().push(file);
                    }
                    Err(e) => {
                        log::debug!("Failed to hash {}: {}", file.path.display(), e);
                    }
                }
            }
        }

        // Filter to only groups with multiple files and return as vec
        Ok(partial_groups
            .into_iter()
            .filter(|(_, files)| files.len() > 1)
            .map(|((hash, size), files)| (hash, size, files))
            .collect())
    }

    /// Stage 3: Verify duplicates with full hash
    fn verify_with_full_hash(
        &mut self,
        partial_groups: Vec<(String, u64, Vec<FileInfo>)>,
        stats: &mut DetectionStats,
    ) -> Result<Vec<DuplicateGroup>, ScanError> {
        let mut final_groups: Vec<DuplicateGroup> = Vec::new();

        for (_partial_hash, size, files) in partial_groups {
            if self.cancelled.load(Ordering::Relaxed) {
                return Err(ScanError::Cancelled);
            }

            // Group by full hash
            let mut full_hash_groups: HashMap<String, Vec<FileInfo>> = HashMap::new();

            for file in files {
                match self.hasher.full_hash(&file.path) {
                    Ok(hash) => {
                        stats.full_hashes += 1;
                        full_hash_groups.entry(hash).or_default().push(file);
                    }
                    Err(e) => {
                        log::debug!("Failed to full hash {}: {}", file.path.display(), e);
                    }
                }
            }

            // Create duplicate groups for files with matching full hashes
            for (hash, files) in full_hash_groups {
                if files.len() > 1 {
                    let mut dup_files: Vec<DuplicateFile> = files
                        .into_iter()
                        .map(DuplicateFile::from)
                        .collect();

                    // Mark the oldest file as original
                    if let Some(oldest_idx) = dup_files
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, f)| f.created_at)
                        .map(|(i, _)| i)
                    {
                        dup_files[oldest_idx].is_original = true;
                    }

                    let group = DuplicateGroup::new(
                        self.next_group_id,
                        hash,
                        size,
                        dup_files,
                    );
                    self.next_group_id += 1;
                    final_groups.push(group);
                }
            }
        }

        // Sort by wasted space (descending)
        final_groups.sort_by(|a, b| b.wasted_space.cmp(&a.wasted_space));

        Ok(final_groups)
    }

    /// Detect duplicates with streaming (processes files as they come in)
    pub fn detect_streaming<I>(&mut self, files: I) -> Result<DetectionResult, ScanError>
    where
        I: Iterator<Item = FileInfo>,
    {
        let files: Vec<_> = files.collect();
        self.detect(files)
    }
}

impl Default for DuplicateDetector {
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

    fn create_test_file(dir: &std::path::Path, name: &str, content: &[u8]) -> FileInfo {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();

        // Get file metadata
        let metadata = std::fs::metadata(&path).unwrap();
        let created = metadata
            .created()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)
            .unwrap_or(0);
        let modified = metadata
            .modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)
            .unwrap_or(0);

        FileInfo {
            path,
            size: content.len() as u64,
            created_at: created,
            modified_at: modified,
            is_symlink: false,
        }
    }

    #[test]
    fn test_no_duplicates() {
        let dir = tempdir().unwrap();
        let files = vec![
            create_test_file(dir.path(), "file1.txt", b"Content A"),
            create_test_file(dir.path(), "file2.txt", b"Content B"),
            create_test_file(dir.path(), "file3.txt", b"Content C"),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        assert_eq!(result.groups.len(), 0);
        assert_eq!(result.duplicate_count, 0);
        assert_eq!(result.total_wasted_space, 0);
    }

    #[test]
    fn test_simple_duplicates() {
        let dir = tempdir().unwrap();
        let content = b"Identical content";

        let files = vec![
            create_test_file(dir.path(), "file1.txt", content),
            create_test_file(dir.path(), "file2.txt", content),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
        assert_eq!(result.duplicate_count, 1);
        assert_eq!(result.total_wasted_space, content.len() as u64);
    }

    #[test]
    fn test_multiple_duplicate_groups() {
        let dir = tempdir().unwrap();

        let files = vec![
            // Group 1: 2 files
            create_test_file(dir.path(), "a1.txt", b"Group A content"),
            create_test_file(dir.path(), "a2.txt", b"Group A content"),
            // Group 2: 3 files
            create_test_file(dir.path(), "b1.txt", b"Group B"),
            create_test_file(dir.path(), "b2.txt", b"Group B"),
            create_test_file(dir.path(), "b3.txt", b"Group B"),
            // Unique file
            create_test_file(dir.path(), "unique.txt", b"Unique"),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        assert_eq!(result.groups.len(), 2);
        assert_eq!(result.duplicate_count, 3); // 1 + 2 duplicates
    }

    #[test]
    fn test_different_sizes_not_duplicates() {
        let dir = tempdir().unwrap();

        // Same prefix but different sizes
        let files = vec![
            create_test_file(dir.path(), "file1.txt", b"Short"),
            create_test_file(dir.path(), "file2.txt", b"Shorter content"),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        // Different sizes means no duplicates
        assert_eq!(result.groups.len(), 0);
    }

    #[test]
    fn test_empty_files_are_duplicates() {
        let dir = tempdir().unwrap();

        let files = vec![
            create_test_file(dir.path(), "empty1.txt", b""),
            create_test_file(dir.path(), "empty2.txt", b""),
            create_test_file(dir.path(), "empty3.txt", b""),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        // All empty files should be in one group
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 3);
        assert_eq!(result.total_wasted_space, 0); // 0 bytes per file
    }

    #[test]
    fn test_original_detection() {
        let dir = tempdir().unwrap();
        let content = b"Test content";

        // Create files with small delays to ensure different timestamps
        let file1 = create_test_file(dir.path(), "newer.txt", content);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let mut file2 = create_test_file(dir.path(), "older.txt", content);

        // Manually set file2 as older
        file2.created_at = file1.created_at - 1000;

        let files = vec![file1, file2];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        assert_eq!(result.groups.len(), 1);

        let original = result.groups[0].original().unwrap();
        assert!(original.path.to_string_lossy().contains("older.txt"));
        assert!(original.is_original);
    }

    #[test]
    fn test_wasted_space_calculation() {
        let dir = tempdir().unwrap();
        let content = vec![0u8; 1000]; // 1000 bytes

        let files = vec![
            create_test_file(dir.path(), "file1.bin", &content),
            create_test_file(dir.path(), "file2.bin", &content),
            create_test_file(dir.path(), "file3.bin", &content),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        // 3 files of 1000 bytes, 2 are duplicates
        assert_eq!(result.total_wasted_space, 2000);
    }

    #[test]
    fn test_sorted_by_wasted_space() {
        let dir = tempdir().unwrap();

        let small_content = vec![0u8; 100];
        let large_content = vec![0u8; 1000];

        let files = vec![
            // Small duplicate group (200 bytes wasted)
            create_test_file(dir.path(), "small1.bin", &small_content),
            create_test_file(dir.path(), "small2.bin", &small_content),
            create_test_file(dir.path(), "small3.bin", &small_content),
            // Large duplicate group (1000 bytes wasted)
            create_test_file(dir.path(), "large1.bin", &large_content),
            create_test_file(dir.path(), "large2.bin", &large_content),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        // Should be sorted by wasted space descending
        assert!(result.groups[0].wasted_space >= result.groups[1].wasted_space);
    }

    #[test]
    fn test_cancel_detection() {
        let dir = tempdir().unwrap();
        let content = b"Test";

        let files = vec![
            create_test_file(dir.path(), "file1.txt", content),
            create_test_file(dir.path(), "file2.txt", content),
        ];

        let mut detector = DuplicateDetector::new();
        let cancel = detector.cancel_handle();

        // Cancel immediately
        cancel.store(true, Ordering::Relaxed);

        let result = detector.detect(files);
        assert!(matches!(result, Err(ScanError::Cancelled)));
    }
}
```

#### 4.3.2 Update Scanner Module

**File**: `src-tauri/src/scanner/mod.rs`

```rust
//! File scanning module

pub mod detector;
pub mod hasher;
pub mod types;
pub mod walker;

#[cfg(test)]
mod tests;

pub use detector::{DetectionResult, DuplicateDetector, DuplicateFile, DuplicateGroup};
pub use hasher::{FileHasher, HashError, HashResult};
pub use types::*;
pub use walker::DirectoryWalker;
```

### Success Criteria

#### Automated Verification

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml detector` passes all detector tests
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml` shows no warnings

#### Manual Verification

- [ ] Detector correctly identifies duplicate files
- [ ] Three-stage algorithm works as expected

### Commit

Execute `/cl:commit` to commit changes with meaningful message.

### Code Review

## Run code-review-fix-loop agent on `src-tauri/src/scanner/detector.rs`.

## Phase 4.3.3: Live Streaming Event Emission

### Overview

This section specifies how and when duplicate discovery events are emitted to the frontend during detection, enabling live UI updates.

### Event Emission Strategy

#### Event Types and Timing

| Event Name        | Emission Point                     | Frequency            | Purpose                      |
| ----------------- | ---------------------------------- | -------------------- | ---------------------------- |
| `scan-progress`   | During file collection             | Every 100 files      | Show file discovery progress |
| `scan-phase`      | Phase transitions                  | Once per phase       | Indicate detection stage     |
| `duplicate-found` | After full hash confirms duplicate | Per group discovered | Live duplicate streaming     |
| `scan-results`    | Detection complete                 | Once                 | Final complete results       |
| `scan-complete`   | End of scan                        | Once                 | Summary statistics           |

#### New Event: `duplicate-found` (Live Streaming)

Add this event to stream duplicate groups as they are discovered:

**Backend emission** (in `detector.rs` `verify_with_full_hash`):

```rust
/// Stage 3: Verify duplicates with full hash (with live streaming)
fn verify_with_full_hash(
    &mut self,
    partial_groups: Vec<(String, u64, Vec<FileInfo>)>,
    stats: &mut DetectionStats,
    app_handle: Option<&AppHandle>,  // Optional for streaming
) -> Result<Vec<DuplicateGroup>, ScanError> {
    let mut final_groups: Vec<DuplicateGroup> = Vec::new();

    for (_partial_hash, size, files) in partial_groups {
        // ... existing grouping logic ...

        // Create duplicate groups for files with matching full hashes
        for (hash, files) in full_hash_groups {
            if files.len() > 1 {
                // ... create dup_files ...

                let group = DuplicateGroup::new(
                    self.next_group_id,
                    hash.clone(),
                    size,
                    dup_files,
                );
                self.next_group_id += 1;

                // LIVE STREAMING: Emit duplicate-found event immediately
                if let Some(handle) = app_handle {
                    let _ = handle.emit("duplicate-found", &group);
                }

                final_groups.push(group);
            }
        }
    }

    // ... rest of method ...
}
```

**Event payload structure**:

```typescript
interface DuplicateFoundEvent {
  id: number; // Group ID
  hash: string; // Full content hash
  file_size: number; // Size of each file
  files: DuplicateFile[];
  wasted_space: number;
}
```

#### Updated `scan-phase` Events

Emit phase transitions at each stage:

```rust
// In start_scan() command

// Phase 1: File collection
let _ = app_handle.emit("scan-phase", serde_json::json!({
    "phase": "collecting",
    "message": "Discovering files..."
}));

// Phase 2: Size grouping (implicit, very fast)

// Phase 3: Partial hashing
let _ = app_handle.emit("scan-phase", serde_json::json!({
    "phase": "partial_hashing",
    "message": "Quick-filtering candidates..."
}));

// Phase 4: Full hashing and verification
let _ = app_handle.emit("scan-phase", serde_json::json!({
    "phase": "full_hashing",
    "message": "Verifying duplicates..."
}));

// Phase 5: Storing results
let _ = app_handle.emit("scan-phase", serde_json::json!({
    "phase": "storing",
    "message": "Saving results..."
}));
```

#### Detection Progress Event

Add a new event for hashing progress:

```rust
// Emit during partial/full hashing
if stats.partial_hashes % 50 == 0 {
    let _ = app_handle.emit("detection-progress", serde_json::json!({
        "partial_hashes": stats.partial_hashes,
        "full_hashes": stats.full_hashes,
        "groups_found": final_groups.len()
    }));
}
```

### Frontend Subscription Pattern

See **Phase 5.1.5** in `05-results-ui.md` for the complete frontend event subscription pattern.

### Throttling Guidelines

To prevent UI flooding:

- `scan-progress`: Every 100 files
- `duplicate-found`: Immediately (groups are infrequent relative to files)
- `detection-progress`: Every 50 hashes
- `scan-phase`: Only on actual phase changes

---

## Phase 4.4: Add Database Queries for Duplicates

### Overview

Add database queries for storing and retrieving duplicate groups and files.

### Changes Required

#### 4.4.1 Update Database Queries

**File**: `src-tauri/src/db/queries.rs`

Add the following module to the existing queries.rs file:

```rust
// Add to the existing queries.rs file

/// Duplicate groups queries
pub mod duplicate_groups {
    use super::*;

    /// Create a new duplicate group
    pub async fn create(
        pool: &SqlitePool,
        hash: &str,
        file_size: i64,
        file_count: i32,
        wasted_space: i64,
        scan_session_id: Option<i64>,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO duplicate_groups (hash, file_size, file_count, wasted_space, scan_session_id)
             VALUES (?, ?, ?, ?, ?) RETURNING id"
        )
        .bind(hash)
        .bind(file_size)
        .bind(file_count)
        .bind(wasted_space)
        .bind(scan_session_id)
        .fetch_one(pool)
        .await?;

        Ok(sqlx::Row::get(&result, 0))
    }

    /// Get all duplicate groups for a scan session
    pub async fn get_by_session(
        pool: &SqlitePool,
        session_id: i64,
    ) -> Result<Vec<crate::db::models::DuplicateGroup>, sqlx::Error> {
        let results: Vec<(i64, String, i64, i32, i64, i64)> = sqlx::query_as(
            "SELECT id, hash, file_size, file_count, wasted_space, created_at
             FROM duplicate_groups
             WHERE scan_session_id = ?
             ORDER BY wasted_space DESC"
        )
        .bind(session_id)
        .fetch_all(pool)
        .await?;

        Ok(results.into_iter().map(|(id, hash, file_size, file_count, wasted_space, created_at)| {
            crate::db::models::DuplicateGroup {
                id,
                hash,
                file_size,
                file_count,
                wasted_space,
                created_at,
            }
        }).collect())
    }

    /// Get total wasted space for a session
    pub async fn get_total_wasted_space(
        pool: &SqlitePool,
        session_id: i64,
    ) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COALESCE(SUM(wasted_space), 0) FROM duplicate_groups WHERE scan_session_id = ?"
        )
        .bind(session_id)
        .fetch_one(pool)
        .await?;

        Ok(result.0)
    }

    /// Delete all groups for a session
    pub async fn delete_by_session(
        pool: &SqlitePool,
        session_id: i64,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM duplicate_groups WHERE scan_session_id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }
}

/// Scanned files queries
pub mod scanned_files {
    use super::*;

    /// Insert a scanned file
    pub async fn insert(
        pool: &SqlitePool,
        path: &str,
        size: i64,
        partial_hash: Option<&str>,
        full_hash: Option<&str>,
        created_at: i64,
        modified_at: i64,
        group_id: Option<i64>,
        scan_session_id: Option<i64>,
    ) -> Result<i64, sqlx::Error> {
        let result = sqlx::query(
            "INSERT INTO scanned_files (path, size, partial_hash, full_hash, created_at, modified_at, group_id, scan_session_id)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                size = excluded.size,
                partial_hash = excluded.partial_hash,
                full_hash = excluded.full_hash,
                created_at = excluded.created_at,
                modified_at = excluded.modified_at,
                group_id = excluded.group_id,
                scan_session_id = excluded.scan_session_id,
                scanned_at = strftime('%s', 'now')
             RETURNING id"
        )
        .bind(path)
        .bind(size)
        .bind(partial_hash)
        .bind(full_hash)
        .bind(created_at)
        .bind(modified_at)
        .bind(group_id)
        .bind(scan_session_id)
        .fetch_one(pool)
        .await?;

        Ok(sqlx::Row::get(&result, 0))
    }

    /// Get files by group ID
    pub async fn get_by_group(
        pool: &SqlitePool,
        group_id: i64,
    ) -> Result<Vec<crate::db::models::ScannedFile>, sqlx::Error> {
        let results: Vec<(i64, String, i64, Option<String>, Option<String>, i64, i64, i64, Option<i64>)> =
            sqlx::query_as(
                "SELECT id, path, size, partial_hash, full_hash, created_at, modified_at, scanned_at, group_id
                 FROM scanned_files
                 WHERE group_id = ?
                 ORDER BY created_at ASC"
            )
            .bind(group_id)
            .fetch_all(pool)
            .await?;

        Ok(results.into_iter().map(|(id, path, size, partial_hash, full_hash, created_at, modified_at, scanned_at, group_id)| {
            crate::db::models::ScannedFile {
                id,
                path,
                size,
                partial_hash,
                full_hash,
                created_at,
                modified_at,
                scanned_at,
                group_id,
            }
        }).collect())
    }

    /// Get file by path
    pub async fn get_by_path(
        pool: &SqlitePool,
        path: &str,
    ) -> Result<Option<crate::db::models::ScannedFile>, sqlx::Error> {
        let result: Option<(i64, String, i64, Option<String>, Option<String>, i64, i64, i64, Option<i64>)> =
            sqlx::query_as(
                "SELECT id, path, size, partial_hash, full_hash, created_at, modified_at, scanned_at, group_id
                 FROM scanned_files
                 WHERE path = ?"
            )
            .bind(path)
            .fetch_optional(pool)
            .await?;

        Ok(result.map(|(id, path, size, partial_hash, full_hash, created_at, modified_at, scanned_at, group_id)| {
            crate::db::models::ScannedFile {
                id,
                path,
                size,
                partial_hash,
                full_hash,
                created_at,
                modified_at,
                scanned_at,
                group_id,
            }
        }))
    }

    /// Update file's group assignment
    pub async fn update_group(
        pool: &SqlitePool,
        file_id: i64,
        group_id: Option<i64>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE scanned_files SET group_id = ? WHERE id = ?")
            .bind(group_id)
            .bind(file_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    /// Delete files by session
    pub async fn delete_by_session(
        pool: &SqlitePool,
        session_id: i64,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query("DELETE FROM scanned_files WHERE scan_session_id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;

        Ok(result.rows_affected())
    }

    /// Count files in a session
    pub async fn count_by_session(
        pool: &SqlitePool,
        session_id: i64,
    ) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM scanned_files WHERE scan_session_id = ?"
        )
        .bind(session_id)
        .fetch_one(pool)
        .await?;

        Ok(result.0)
    }
}

/// File cache queries (for incremental scanning)
pub mod file_cache {
    use super::*;

    /// Get cached file info
    pub async fn get(
        pool: &SqlitePool,
        path: &str,
        size: i64,
        modified_at: i64,
    ) -> Result<Option<crate::db::models::FileCache>, sqlx::Error> {
        let result: Option<(i64, String, i64, i64, String, Option<String>, i64)> = sqlx::query_as(
            "SELECT id, path, size, modified_at, partial_hash, full_hash, cached_at
             FROM file_cache
             WHERE path = ? AND size = ? AND modified_at = ?"
        )
        .bind(path)
        .bind(size)
        .bind(modified_at)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|(id, path, size, modified_at, partial_hash, full_hash, cached_at)| {
            crate::db::models::FileCache {
                id,
                path,
                size,
                modified_at,
                partial_hash,
                full_hash,
                cached_at,
            }
        }))
    }

    /// Upsert file cache entry
    pub async fn upsert(
        pool: &SqlitePool,
        path: &str,
        size: i64,
        modified_at: i64,
        partial_hash: &str,
        full_hash: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO file_cache (path, size, modified_at, partial_hash, full_hash)
             VALUES (?, ?, ?, ?, ?)
             ON CONFLICT(path) DO UPDATE SET
                size = excluded.size,
                modified_at = excluded.modified_at,
                partial_hash = excluded.partial_hash,
                full_hash = COALESCE(excluded.full_hash, file_cache.full_hash),
                cached_at = strftime('%s', 'now')"
        )
        .bind(path)
        .bind(size)
        .bind(modified_at)
        .bind(partial_hash)
        .bind(full_hash)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Clear old cache entries
    pub async fn clear_old(
        pool: &SqlitePool,
        days: i32,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            "DELETE FROM file_cache WHERE cached_at < strftime('%s', 'now') - (? * 86400)"
        )
        .bind(days)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Remove cache entry for a specific path (for explicit invalidation)
    pub async fn invalidate(
        pool: &SqlitePool,
        path: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM file_cache WHERE path = ?")
            .bind(path)
            .execute(pool)
            .await?;
        Ok(())
    }

    /// Remove cache entries for files that no longer exist
    /// Call this after a scan to clean up stale entries
    pub async fn cleanup_missing_files(
        pool: &SqlitePool,
        valid_paths: &[String],
    ) -> Result<u64, sqlx::Error> {
        // For large sets, this should be done in batches
        // Here we use a simple approach for moderate sizes
        if valid_paths.is_empty() {
            return Ok(0);
        }

        // Build placeholders for the IN clause
        let placeholders: Vec<&str> = valid_paths.iter().map(|_| "?").collect();
        let query = format!(
            "DELETE FROM file_cache WHERE path NOT IN ({})",
            placeholders.join(", ")
        );

        let mut query_builder = sqlx::query(&query);
        for path in valid_paths {
            query_builder = query_builder.bind(path);
        }

        let result = query_builder.execute(pool).await?;
        Ok(result.rows_affected())
    }
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml queries` passes

#### Manual Verification

- [ ] Database queries are comprehensive

### Commit

Execute `/cl:commit` to commit changes with meaningful message.

### Code Review

## Run code-review-fix-loop agent on the updated `src-tauri/src/db/queries.rs`.

## Phase 4.4.1: Cache Invalidation Strategy

### Overview

The file cache enables incremental scanning by storing computed hashes. This section documents the cache invalidation strategy to ensure cache correctness when files are modified, renamed, moved, or deleted.

### Cache Invalidation Approach

#### 1. Implicit Invalidation via Composite Key

The cache uses a **composite key** of `(path, size, modified_at)` for lookups:

```rust
pub async fn get(
    pool: &SqlitePool,
    path: &str,
    size: i64,
    modified_at: i64,
) -> Result<Option<FileCache>, sqlx::Error>
```

**How it works:**

- When a file is **modified**, its `modified_at` timestamp changes
- The cache lookup with the new timestamp returns `None` (cache miss)
- The file is rehashed and the cache is updated via `upsert()`
- The old entry is automatically replaced due to `ON CONFLICT(path) DO UPDATE`

**Covered scenarios:**

- File content modified (timestamp changes → cache miss → rehash)
- File truncated/appended (size and/or timestamp changes → cache miss)

#### 2. Explicit Invalidation for Path Changes

When files are renamed or moved, the path changes but the content remains the same. The `invalidate()` function handles this:

```rust
pub async fn invalidate(pool: &SqlitePool, path: &str) -> Result<(), sqlx::Error>
```

**When to call:**

- After detecting a file rename operation
- After detecting a file move operation
- When a user manually requests cache refresh

#### 3. Cleanup of Deleted Files

After a scan completes, files that no longer exist should be removed from the cache:

```rust
pub async fn cleanup_missing_files(
    pool: &SqlitePool,
    valid_paths: &[String],
) -> Result<u64, sqlx::Error>
```

**When to call:**

- At the end of each full scan, pass all discovered file paths
- The function removes cache entries for paths not in the valid set

#### 4. Time-Based Cache Expiration

Stale cache entries are cleaned up periodically:

```rust
pub async fn clear_old(pool: &SqlitePool, days: i32) -> Result<u64, sqlx::Error>
```

**Recommended usage:**

- Call on app startup with `days = 30` (configurable)
- Removes entries not accessed in the specified period
- Prevents unbounded cache growth

### Cache Invalidation Flow

```
┌─────────────────────────────────────────────────────────────────┐
│                     File Cache Lookup                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   File discovered during scan                                    │
│           │                                                      │
│           ▼                                                      │
│   ┌───────────────────────────────────┐                         │
│   │ Query cache with (path, size, mtime) │                      │
│   └───────────────────────────────────┘                         │
│           │                                                      │
│     ┌─────┴─────┐                                               │
│     │           │                                                │
│   Cache Hit   Cache Miss                                         │
│     │           │                                                │
│     ▼           ▼                                                │
│  Use cached   Compute hash                                       │
│   hashes         │                                               │
│     │           ▼                                                │
│     │      Upsert to cache                                       │
│     │      (replaces old entry                                   │
│     │       if path exists)                                      │
│     │           │                                                │
│     └─────┬─────┘                                                │
│           ▼                                                      │
│    Continue scan                                                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                   Post-Scan Cleanup                              │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   Scan completes with list of valid paths                        │
│           │                                                      │
│           ▼                                                      │
│   cleanup_missing_files(valid_paths)                             │
│           │                                                      │
│           ▼                                                      │
│   Removes cache entries for:                                     │
│   - Deleted files                                                │
│   - Files outside scan scope                                     │
│   - Renamed/moved files (old paths)                              │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Edge Cases Handled

| Scenario                  | Detection Method                | Invalidation Action         |
| ------------------------- | ------------------------------- | --------------------------- |
| File modified             | `modified_at` changes           | Implicit (cache miss)       |
| File renamed              | Old path not in scan results    | `cleanup_missing_files()`   |
| File moved                | Old path not in scan results    | `cleanup_missing_files()`   |
| File deleted              | Path not in scan results        | `cleanup_missing_files()`   |
| File replaced (same name) | `size` or `modified_at` changes | Implicit (cache miss)       |
| Cache corruption          | N/A                             | `clear_old(0)` to purge all |

### Implementation Notes

1. **Batch cleanup**: For large scans (>100k files), `cleanup_missing_files()` should be called in batches to avoid SQLite query size limits.

2. **Incremental scan mode**: When doing a "quick scan" that only checks previously scanned directories, skip `cleanup_missing_files()` to preserve cache entries for unscanned areas.

3. **Watch mode** (future): For real-time file watching, call `invalidate()` on file change events from the OS file watcher.

---

## Phase 4.5: Integrate Detection with Scan

### Overview

Integrate the duplicate detector with the scan workflow.

### Changes Required

#### 4.5.1 Update Scan Commands

**File**: `src-tauri/src/commands/scan.rs`

Update the `start_scan` function to include duplicate detection:

```rust
//! Scan-related Tauri commands

use crate::db::models::ScanStatus;
use crate::db::queries;
use crate::scanner::{
    DetectionResult, DirectoryWalker, DuplicateDetector, FileInfo,
    ParallelismMode, ScanConfig, ScanProgress,
};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager, State};

/// Scan request from frontend
#[derive(Debug, Clone, Deserialize)]
pub struct ScanRequest {
    pub paths: Vec<String>,
    pub parallelism: Option<String>,
    pub quick_scan: Option<bool>,
}

/// Scan response for frontend
#[derive(Debug, Clone, Serialize)]
pub struct ScanResponse {
    pub session_id: i64,
    pub message: String,
}

/// Scan completion data
#[derive(Debug, Clone, Serialize)]
pub struct ScanComplete {
    pub session_id: i64,
    pub total_files: u64,
    pub total_bytes: u64,
    pub duplicate_groups: usize,
    pub duplicate_files: u64,
    pub wasted_space: u64,
    pub duration_ms: u64,
}

/// Global scan state (separate from AppState for cancellation)
pub struct ScanState {
    pub cancel_flag: Option<Arc<AtomicBool>>,
}

impl ScanState {
    pub fn new() -> Self {
        Self { cancel_flag: None }
    }
}

impl Default for ScanState {
    fn default() -> Self {
        Self::new()
    }
}

/// Start a new scan
#[tauri::command]
pub async fn start_scan(
    request: ScanRequest,
    app_handle: AppHandle,
    state: State<'_, Mutex<AppState>>,
    scan_state: State<'_, Mutex<ScanState>>,
) -> Result<ScanResponse, String> {
    // Check if a scan is already running
    {
        let state = state.lock().map_err(|e| e.to_string())?;
        if state.is_scanning {
            return Err("A scan is already in progress".to_string());
        }
    }

    // Parse paths
    let paths: Vec<PathBuf> = request
        .paths
        .into_iter()
        .map(PathBuf::from)
        .collect();

    if paths.is_empty() {
        return Err("No paths provided for scanning".to_string());
    }

    // Parse parallelism mode
    let parallelism = match request.parallelism.as_deref() {
        Some("light") => ParallelismMode::Light,
        Some("aggressive") => ParallelismMode::Aggressive,
        _ => ParallelismMode::Normal,
    };

    // Create scan session in database
    let session_id = {
        let state = state.lock().map_err(|e| e.to_string())?;
        let db = state.database().ok_or("Database not initialized")?;
        let db = tauri::async_runtime::block_on(db.lock());

        let path_strings: Vec<String> = paths.iter().map(|p| p.display().to_string()).collect();

        tauri::async_runtime::block_on(async {
            queries::scan_sessions::create(db.pool(), &path_strings).await
        })
        .map_err(|e| e.to_string())?
    };

    // Update state to indicate scanning
    {
        let mut state = state.lock().map_err(|e| e.to_string())?;
        state.is_scanning = true;
        state.current_scan_id = Some(session_id);
    }

    // Create scan configuration
    let config = ScanConfig {
        paths,
        follow_symlinks: false,
        max_depth: None,
        parallelism,
    };

    // Create walker and get cancel handle
    let walker = DirectoryWalker::new(config);
    let cancel_handle = walker.cancel_handle();

    // Store cancel handle
    {
        let mut scan_state = scan_state.lock().map_err(|e| e.to_string())?;
        scan_state.cancel_flag = Some(cancel_handle);
    }

    // Spawn scan task
    let state_clone = state.inner().clone();
    let scan_state_clone = scan_state.inner().clone();

    tauri::async_runtime::spawn(async move {
        let start_time = std::time::Instant::now();

        // Phase 1: Collect all files
        log::info!("Starting file collection...");
        let (receiver, walker_handle) = walker.walk_channel();

        let mut files: Vec<FileInfo> = Vec::new();
        let mut file_count: u64 = 0;
        let mut total_size: u64 = 0;

        for result in receiver {
            match result {
                Ok(file_info) => {
                    file_count += 1;
                    total_size += file_info.size;

                    // Emit progress event every 100 files
                    if file_count % 100 == 0 {
                        let progress = ScanProgress {
                            total_files: file_count,
                            processed_files: file_count,
                            total_bytes: total_size,
                            current_path: Some(file_info.path.display().to_string()),
                            skipped_files: 0,
                            estimated_total: None,
                        };
                        let _ = app_handle.emit("scan-progress", progress);
                    }

                    files.push(file_info);
                }
                Err((path, error)) => {
                    log::debug!("Skipped file {}: {}", path.display(), error);
                }
            }
        }

        let scan_stats = walker_handle.join().unwrap_or_default();
        log::info!(
            "File collection complete: {} files, {} bytes",
            file_count,
            total_size
        );

        // Phase 2: Detect duplicates
        log::info!("Starting duplicate detection...");
        let _ = app_handle.emit(
            "scan-phase",
            serde_json::json!({
                "phase": "detecting",
                "message": "Analyzing files for duplicates..."
            }),
        );

        let mut detector = DuplicateDetector::new();
        let detection_result = match detector.detect(files) {
            Ok(result) => result,
            Err(e) => {
                log::error!("Detection failed: {}", e);

                // Update status to failed
                if let Ok(state) = state_clone.lock() {
                    if let Some(db) = state.database() {
                        let db = db.blocking_lock();
                        let _ = tauri::async_runtime::block_on(async {
                            queries::scan_sessions::update_status(
                                db.pool(),
                                session_id,
                                ScanStatus::Failed,
                            )
                            .await
                        });
                    }
                }

                // Clear scanning state
                if let Ok(mut state) = state_clone.lock() {
                    state.is_scanning = false;
                    state.current_scan_id = None;
                }

                let _ = app_handle.emit(
                    "scan-error",
                    serde_json::json!({
                        "session_id": session_id,
                        "error": e.to_string()
                    }),
                );
                return;
            }
        };

        log::info!(
            "Detection complete: {} groups, {} duplicates, {} bytes wasted",
            detection_result.groups.len(),
            detection_result.duplicate_count,
            detection_result.total_wasted_space
        );

        // Phase 3: Store results in database
        if let Ok(state) = state_clone.lock() {
            if let Some(db) = state.database() {
                let db = db.blocking_lock();

                // Store duplicate groups and files
                for group in &detection_result.groups {
                    let group_id = tauri::async_runtime::block_on(async {
                        queries::duplicate_groups::create(
                            db.pool(),
                            &group.hash,
                            group.file_size as i64,
                            group.files.len() as i32,
                            group.wasted_space as i64,
                            Some(session_id),
                        )
                        .await
                    });

                    if let Ok(group_id) = group_id {
                        for file in &group.files {
                            let _ = tauri::async_runtime::block_on(async {
                                queries::scanned_files::insert(
                                    db.pool(),
                                    &file.path.display().to_string(),
                                    file.size as i64,
                                    None, // partial_hash stored separately
                                    Some(&group.hash),
                                    file.created_at,
                                    file.modified_at,
                                    Some(group_id),
                                    Some(session_id),
                                )
                                .await
                            });
                        }
                    }
                }

                // Update session stats
                let _ = tauri::async_runtime::block_on(async {
                    queries::scan_sessions::update_stats(
                        db.pool(),
                        session_id,
                        scan_stats.total_files as i64,
                        scan_stats.total_bytes as i64,
                        detection_result.groups.len() as i32,
                        detection_result.total_wasted_space as i64,
                    )
                    .await
                });

                let _ = tauri::async_runtime::block_on(async {
                    queries::scan_sessions::update_status(
                        db.pool(),
                        session_id,
                        ScanStatus::Completed,
                    )
                    .await
                });
            }
        }

        // Clear scanning state
        if let Ok(mut state) = state_clone.lock() {
            state.is_scanning = false;
            state.current_scan_id = None;
        }

        // Clear cancel flag
        if let Ok(mut scan_state) = scan_state_clone.lock() {
            scan_state.cancel_flag = None;
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Emit completion event
        let _ = app_handle.emit(
            "scan-complete",
            ScanComplete {
                session_id,
                total_files: scan_stats.total_files,
                total_bytes: scan_stats.total_bytes,
                duplicate_groups: detection_result.groups.len(),
                duplicate_files: detection_result.duplicate_count,
                wasted_space: detection_result.total_wasted_space,
                duration_ms,
            },
        );

        // Also emit the full detection result for the UI
        let _ = app_handle.emit("scan-results", &detection_result);
    });

    Ok(ScanResponse {
        session_id,
        message: "Scan started".to_string(),
    })
}

/// Cancel the current scan
#[tauri::command]
pub async fn cancel_scan(
    state: State<'_, Mutex<AppState>>,
    scan_state: State<'_, Mutex<ScanState>>,
) -> Result<(), String> {
    // Set cancel flag
    {
        let scan_state = scan_state.lock().map_err(|e| e.to_string())?;
        if let Some(cancel_flag) = &scan_state.cancel_flag {
            cancel_flag.store(true, Ordering::Relaxed);
        } else {
            return Err("No scan in progress".to_string());
        }
    }

    // Update database status
    {
        let state = state.lock().map_err(|e| e.to_string())?;
        if let (Some(db), Some(session_id)) = (state.database(), state.current_scan_id) {
            let db = db.blocking_lock();
            tauri::async_runtime::block_on(async {
                queries::scan_sessions::update_status(db.pool(), session_id, ScanStatus::Cancelled)
                    .await
            })
            .map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/// Get current scan progress
#[tauri::command]
pub async fn get_scan_progress(
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<ScanProgress>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;

    if !state.is_scanning {
        return Ok(None);
    }

    Ok(Some(ScanProgress::default()))
}

/// Check if a scan is currently running
#[tauri::command]
pub async fn is_scanning(state: State<'_, Mutex<AppState>>) -> Result<bool, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    Ok(state.is_scanning)
}

/// Get the latest scan results
#[tauri::command]
pub async fn get_scan_results(
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<DetectionResult>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let db = state.database().ok_or("Database not initialized")?;
    let db = tauri::async_runtime::block_on(db.lock());

    // Get latest completed session
    let session = tauri::async_runtime::block_on(async {
        queries::scan_sessions::get_latest(db.pool()).await
    })
    .map_err(|e| e.to_string())?;

    let session = match session {
        Some(s) if s.status == "completed" => s,
        _ => return Ok(None),
    };

    // Get duplicate groups
    let db_groups = tauri::async_runtime::block_on(async {
        queries::duplicate_groups::get_by_session(db.pool(), session.id).await
    })
    .map_err(|e| e.to_string())?;

    // Build detection result from database
    let mut groups = Vec::new();
    let mut total_duplicate_count: u64 = 0;

    for db_group in db_groups {
        let db_files = tauri::async_runtime::block_on(async {
            queries::scanned_files::get_by_group(db.pool(), db_group.id).await
        })
        .map_err(|e| e.to_string())?;

        let files: Vec<crate::scanner::DuplicateFile> = db_files
            .into_iter()
            .enumerate()
            .map(|(i, f)| crate::scanner::DuplicateFile {
                path: PathBuf::from(&f.path),
                size: f.size as u64,
                created_at: f.created_at,
                modified_at: f.modified_at,
                is_original: i == 0, // First file (oldest) is original
            })
            .collect();

        if files.len() > 1 {
            total_duplicate_count += (files.len() - 1) as u64;
        }

        groups.push(crate::scanner::DuplicateGroup {
            id: db_group.id as u64,
            hash: db_group.hash,
            file_size: db_group.file_size as u64,
            files,
            wasted_space: db_group.wasted_space as u64,
        });
    }

    Ok(Some(DetectionResult {
        groups,
        duplicate_count: total_duplicate_count,
        total_wasted_space: session.wasted_space as u64,
        unique_files: session.total_files as u64 - total_duplicate_count,
        stats: crate::scanner::DetectionStats::default(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_state_new() {
        let state = ScanState::new();
        assert!(state.cancel_flag.is_none());
    }

    #[test]
    fn test_scan_state_default() {
        let state = ScanState::default();
        assert!(state.cancel_flag.is_none());
    }
}
```

#### 4.5.2 Update lib.rs to Register New Command

**File**: `src-tauri/src/lib.rs`

Add the `get_scan_results` command:

```rust
.invoke_handler(tauri::generate_handler![
    commands::greet,
    // Settings commands
    commands::get_setting,
    commands::set_setting,
    commands::get_all_settings,
    // Protected folders commands
    commands::add_protected_folder,
    commands::remove_protected_folder,
    commands::get_protected_folders,
    commands::is_path_protected,
    // Scan commands
    commands::start_scan,
    commands::cancel_scan,
    commands::get_scan_progress,
    commands::is_scanning,
    commands::get_scan_results,
])
```

### Success Criteria

#### Automated Verification

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes

#### Manual Verification

- [ ] Scan correctly detects duplicates
- [ ] Results are stored in the database
- [ ] Events are emitted to the frontend

### Commit

Execute `/cl:commit` to commit changes with meaningful message.

### Code Review

## Run code-review-fix-loop agent on `src-tauri/src/commands/scan.rs`.

## Phase 4.6: Update Detection Stats Type

### Overview

Add the DetectionStats type to the scanner types module for complete export.

### Changes Required

#### 4.6.1 Update Scanner Types

**File**: `src-tauri/src/scanner/types.rs`

Add at the end of the file before the tests:

```rust
// Re-export detection types
pub use super::detector::{DetectionResult, DetectionStats, DuplicateFile, DuplicateGroup};
```

Then update the mod.rs to export properly:

**File**: `src-tauri/src/scanner/mod.rs`

```rust
//! File scanning module

pub mod detector;
pub mod hasher;
pub mod types;
pub mod walker;

#[cfg(test)]
mod tests;

pub use detector::{DetectionResult, DetectionStats, DuplicateDetector, DuplicateFile, DuplicateGroup};
pub use hasher::{FileHasher, HashError, HashResult};
pub use types::*;
pub use walker::DirectoryWalker;
```

### Success Criteria

#### Automated Verification

- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` passes

### Commit

Execute `/cl:commit` to commit changes with meaningful message.

### Code Review

## Run code-review-fix-loop agent on scanner module files.

## Phase 4.7: Add Detection Unit Tests

### Overview

Add comprehensive tests for the duplicate detection system.

### Changes Required

#### 4.7.1 Add More Detector Tests

**File**: `src-tauri/src/scanner/detector.rs`

Add additional tests to the existing test module:

```rust
// Add to the existing tests module in detector.rs

    #[test]
    fn test_large_file_detection() {
        let dir = tempdir().unwrap();

        // Create larger files (16KB) to test partial hash path
        let large_content: Vec<u8> = (0..16384).map(|i| (i % 256) as u8).collect();

        let files = vec![
            create_large_test_file(dir.path(), "large1.bin", &large_content),
            create_large_test_file(dir.path(), "large2.bin", &large_content),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
    }

    fn create_large_test_file(dir: &std::path::Path, name: &str, content: &[u8]) -> FileInfo {
        let path = dir.join(name);
        let mut file = File::create(&path).unwrap();
        file.write_all(content).unwrap();

        let metadata = std::fs::metadata(&path).unwrap();
        let created = metadata
            .created()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)
            .unwrap_or(0);
        let modified = metadata
            .modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64)
            .unwrap_or(0);

        FileInfo {
            path,
            size: content.len() as u64,
            created_at: created,
            modified_at: modified,
            is_symlink: false,
        }
    }

    #[test]
    fn test_partial_hash_false_positive() {
        let dir = tempdir().unwrap();

        // Create files with same start/end but different middle
        // This tests that full hash is used to eliminate false positives
        let mut content1: Vec<u8> = vec![0u8; 16384];
        let mut content2: Vec<u8> = vec![0u8; 16384];

        // Same first 4KB
        for i in 0..4096 {
            content1[i] = 0xAA;
            content2[i] = 0xAA;
        }

        // Different middle
        content1[8000] = 0xFF;
        content2[8000] = 0x00;

        // Same last 4KB
        for i in 12288..16384 {
            content1[i] = 0xBB;
            content2[i] = 0xBB;
        }

        let files = vec![
            create_large_test_file(dir.path(), "false_pos1.bin", &content1),
            create_large_test_file(dir.path(), "false_pos2.bin", &content2),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        // Should NOT be detected as duplicates because full hash differs
        assert_eq!(result.groups.len(), 0);
    }

    #[test]
    fn test_many_duplicates_performance() {
        let dir = tempdir().unwrap();
        let content = b"Duplicate content for performance test";

        // Create 100 duplicate files
        let files: Vec<FileInfo> = (0..100)
            .map(|i| create_test_file(dir.path(), &format!("file{}.txt", i), content))
            .collect();

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 100);
        assert_eq!(result.duplicate_count, 99);
    }

    #[test]
    fn test_mixed_sizes_and_duplicates() {
        let dir = tempdir().unwrap();

        let files = vec![
            // Size 10
            create_test_file(dir.path(), "a1.txt", b"0123456789"),
            create_test_file(dir.path(), "a2.txt", b"0123456789"),
            // Size 10 but different content
            create_test_file(dir.path(), "b1.txt", b"9876543210"),
            // Size 5
            create_test_file(dir.path(), "c1.txt", b"12345"),
            create_test_file(dir.path(), "c2.txt", b"12345"),
            create_test_file(dir.path(), "c3.txt", b"12345"),
            // Unique
            create_test_file(dir.path(), "unique.txt", b"unique content here"),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        // Should have 2 groups: size 10 dups and size 5 dups
        assert_eq!(result.groups.len(), 2);

        // Total duplicates: 1 (from size 10 group) + 2 (from size 5 group)
        assert_eq!(result.duplicate_count, 3);
    }

    #[test]
    fn test_stats_tracking() {
        let dir = tempdir().unwrap();
        let content = b"Test content";

        let files = vec![
            create_test_file(dir.path(), "file1.txt", content),
            create_test_file(dir.path(), "file2.txt", content),
            create_test_file(dir.path(), "file3.txt", b"Different"),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        // Check that stats were tracked
        assert!(result.stats.size_grouping_ms >= 0);
        assert!(result.stats.partial_hashes > 0);
        assert!(result.stats.full_hashes > 0);
    }
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test --manifest-path src-tauri/Cargo.toml detector` passes all tests
- [ ] `cargo clippy --manifest-path src-tauri/Cargo.toml` shows no warnings

#### Manual Verification

- [ ] Tests cover edge cases and performance scenarios

### Commit

Execute `/cl:commit` to commit changes with meaningful message.

### Code Review

## Run code-review-fix-loop agent on the test additions.

## Phase 4.8: Update Frontend to Display Detection Results

### Overview

Update the ScanButton component to display duplicate detection results.

### Changes Required

#### 4.8.1 Update ScanButton Component

**File**: `src/lib/components/ScanButton.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';

  interface ScanProgress {
    total_files: number;
    processed_files: number;
    total_bytes: number;
    current_path: string | null;
    skipped_files: number;
  }

  interface ScanComplete {
    session_id: number;
    total_files: number;
    total_bytes: number;
    duplicate_groups: number;
    duplicate_files: number;
    wasted_space: number;
    duration_ms: number;
  }

  interface DuplicateFile {
    path: string;
    size: number;
    created_at: number;
    modified_at: number;
    is_original: boolean;
  }

  interface DuplicateGroup {
    id: number;
    hash: string;
    file_size: number;
    files: DuplicateFile[];
    wasted_space: number;
  }

  interface DetectionResult {
    groups: DuplicateGroup[];
    duplicate_count: number;
    total_wasted_space: number;
    unique_files: number;
  }

  let isScanning = $state(false);
  let progress = $state<ScanProgress | null>(null);
  let scanResult = $state<ScanComplete | null>(null);
  let detectionResult = $state<DetectionResult | null>(null);
  let error = $state<string | null>(null);
  let phase = $state<string>('idle');

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;
  let unlistenResults: UnlistenFn | null = null;
  let unlistenPhase: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  onMount(async () => {
    unlistenProgress = await listen<ScanProgress>('scan-progress', (event) => {
      progress = event.payload;
    });

    unlistenComplete = await listen<ScanComplete>('scan-complete', (event) => {
      scanResult = event.payload;
      isScanning = false;
      progress = null;
      phase = 'complete';
    });

    unlistenResults = await listen<DetectionResult>('scan-results', (event) => {
      detectionResult = event.payload;
    });

    unlistenPhase = await listen<{ phase: string; message: string }>('scan-phase', (event) => {
      phase = event.payload.phase;
    });

    unlistenError = await listen<{ session_id: number; error: string }>('scan-error', (event) => {
      error = event.payload.error;
      isScanning = false;
      phase = 'error';
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenComplete?.();
    unlistenResults?.();
    unlistenPhase?.();
    unlistenError?.();
  });

  async function startScan() {
    error = null;
    scanResult = null;
    detectionResult = null;
    phase = 'scanning';

    try {
      isScanning = true;
      // Scan current user's home directory for testing
      // In production, this would be selected by the user
      await invoke('start_scan', {
        request: {
          paths: ['/Users'],
          parallelism: 'normal',
        },
      });
    } catch (e) {
      error = String(e);
      isScanning = false;
      phase = 'error';
    }
  }

  async function cancelScan() {
    try {
      await invoke('cancel_scan');
      isScanning = false;
      progress = null;
      phase = 'cancelled';
    } catch (e) {
      error = String(e);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const seconds = Math.floor(ms / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  }

  function truncatePath(path: string, maxLen: number = 50): string {
    if (path.length <= maxLen) return path;
    const start = path.substring(0, 20);
    const end = path.substring(path.length - 25);
    return `${start}...${end}`;
  }
</script>

<div class="scan-container">
  <div class="scan-controls">
    {#if isScanning}
      <button class="cancel-button" onclick={cancelScan}> Cancel Scan </button>
    {:else}
      <button class="scan-button" onclick={startScan}> Start Scan </button>
    {/if}
  </div>

  {#if error}
    <div class="error-message">
      Error: {error}
    </div>
  {/if}

  {#if isScanning}
    <div class="progress-container">
      <div class="progress-header">
        {#if phase === 'scanning'}
          Scanning files...
        {:else if phase === 'detecting'}
          Analyzing for duplicates...
        {:else}
          Processing...
        {/if}
      </div>
      {#if progress}
        <div class="progress-stats">
          <div class="stat">
            <span class="label">Files:</span>
            <span class="value">{progress.total_files.toLocaleString()}</span>
          </div>
          <div class="stat">
            <span class="label">Size:</span>
            <span class="value">{formatBytes(progress.total_bytes)}</span>
          </div>
        </div>
        {#if progress.current_path}
          <div class="current-path" title={progress.current_path}>
            {truncatePath(progress.current_path)}
          </div>
        {/if}
      {/if}
    </div>
  {/if}

  {#if scanResult && !isScanning}
    <div class="result-container">
      <div class="result-header">Scan Complete</div>
      <div class="result-stats">
        <div class="stat">
          <span class="label">Total Files:</span>
          <span class="value">{scanResult.total_files.toLocaleString()}</span>
        </div>
        <div class="stat">
          <span class="label">Total Size:</span>
          <span class="value">{formatBytes(scanResult.total_bytes)}</span>
        </div>
        <div class="stat">
          <span class="label">Duration:</span>
          <span class="value">{formatDuration(scanResult.duration_ms)}</span>
        </div>
        <div class="stat highlight">
          <span class="label">Duplicate Groups:</span>
          <span class="value">{scanResult.duplicate_groups}</span>
        </div>
        <div class="stat highlight">
          <span class="label">Duplicate Files:</span>
          <span class="value">{scanResult.duplicate_files}</span>
        </div>
        <div class="stat highlight warning">
          <span class="label">Wasted Space:</span>
          <span class="value">{formatBytes(scanResult.wasted_space)}</span>
        </div>
      </div>
    </div>

    {#if detectionResult && detectionResult.groups.length > 0}
      <div class="groups-container">
        <div class="groups-header">
          Duplicate Groups ({detectionResult.groups.length})
        </div>
        <div class="groups-list">
          {#each detectionResult.groups.slice(0, 10) as group}
            <div class="group-card">
              <div class="group-info">
                <span class="group-size">{formatBytes(group.file_size)} each</span>
                <span class="group-count">{group.files.length} files</span>
                <span class="group-wasted">Wasted: {formatBytes(group.wasted_space)}</span>
              </div>
              <div class="group-files">
                {#each group.files as file}
                  <div class="file-item" class:original={file.is_original}>
                    {#if file.is_original}
                      <span class="original-badge">Original</span>
                    {/if}
                    <span class="file-path" title={file.path}>{truncatePath(file.path, 60)}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
          {#if detectionResult.groups.length > 10}
            <div class="more-groups">
              +{detectionResult.groups.length - 10} more groups...
            </div>
          {/if}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .scan-container {
    width: 100%;
    max-width: 700px;
  }

  .scan-controls {
    margin-bottom: 1rem;
  }

  .scan-button,
  .cancel-button {
    width: 100%;
    padding: 0.75rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .scan-button {
    background: var(--primary);
    color: white;
  }

  .cancel-button {
    background: var(--error);
    color: white;
  }

  .scan-button:hover,
  .cancel-button:hover {
    opacity: 0.9;
  }

  .error-message {
    padding: 0.75rem;
    background: var(--error-bg);
    color: var(--error);
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .progress-container,
  .result-container {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 1rem;
  }

  .progress-header,
  .result-header,
  .groups-header {
    font-weight: 600;
    margin-bottom: 0.75rem;
  }

  .progress-stats,
  .result-stats {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.5rem;
  }

  .stat {
    display: flex;
    justify-content: space-between;
    padding: 0.25rem 0;
  }

  .stat.highlight {
    background: var(--background);
    padding: 0.5rem;
    border-radius: 4px;
  }

  .stat.warning .value {
    color: var(--warning);
    font-weight: 600;
  }

  .label {
    color: var(--text-secondary);
  }

  .value {
    font-weight: 500;
  }

  .current-path {
    margin-top: 0.75rem;
    padding: 0.5rem;
    background: var(--background);
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.75rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }

  .result-container {
    background: var(--success-bg);
  }

  .result-header {
    color: var(--success);
  }

  .groups-container {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
  }

  .groups-header {
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
  }

  .groups-list {
    max-height: 400px;
    overflow-y: auto;
  }

  .group-card {
    border: 1px solid var(--border);
    border-radius: 6px;
    margin-top: 0.75rem;
    overflow: hidden;
  }

  .group-info {
    display: flex;
    gap: 1rem;
    padding: 0.75rem;
    background: var(--background);
    font-size: 0.875rem;
  }

  .group-size {
    font-weight: 500;
  }

  .group-count {
    color: var(--text-secondary);
  }

  .group-wasted {
    color: var(--warning);
    margin-left: auto;
  }

  .group-files {
    padding: 0.5rem;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    font-size: 0.8rem;
    font-family: var(--font-mono);
  }

  .file-item.original {
    background: var(--success-bg);
    border-radius: 4px;
  }

  .original-badge {
    font-size: 0.7rem;
    padding: 0.1rem 0.3rem;
    background: var(--success);
    color: white;
    border-radius: 3px;
    font-family: var(--font-sans);
  }

  .file-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .more-groups {
    text-align: center;
    padding: 1rem;
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
```

### Success Criteria

#### Automated Verification

- [ ] `npm run check` passes

#### Manual Verification

- [ ] `npm run tauri dev` starts without errors
- [ ] Scanning works and shows progress
- [ ] Detection results are displayed
- [ ] Duplicate groups are listed with file paths
- [ ] "Original" badge is shown on oldest file

### Commit

Execute `/cl:commit` to commit changes with meaningful message.

### Code Review

## Run code-review-fix-loop agent on `src/lib/components/ScanButton.svelte`.

## End of File 04

After completing all phases in this file, you should have:

1. BLAKE3 hashing integration
2. File hasher with partial and full hash support
3. Duplicate detector with three-stage algorithm
4. Database queries for duplicate groups and files
5. Integrated detection with scan workflow
6. Frontend display of detection results
7. Comprehensive unit tests

**Next**: Proceed to [05-results-ui.md](./05-results-ui.md) to build the full results UI with master-detail layout.
