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
}

/// Result of hashing a file
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
        #[allow(clippy::cast_possible_truncation)]
        let chunk_size = PARTIAL_HASH_CHUNK_SIZE as usize;
        let mut first_chunk = vec![0u8; chunk_size];
        file.read_exact(&mut first_chunk)?;
        hasher.update(&first_chunk);

        // Seek to last chunk
        #[allow(clippy::cast_possible_wrap)]
        let neg_chunk = -(PARTIAL_HASH_CHUNK_SIZE as i64);
        file.seek(SeekFrom::End(neg_chunk))?;

        // Read last chunk
        let mut last_chunk = vec![0u8; chunk_size];
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
    #[allow(dead_code, clippy::unused_self)]
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
    #[allow(dead_code)]
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
    pub fn verify_hash<P: AsRef<Path>>(
        &mut self,
        path: P,
        expected_hash: &str,
    ) -> Result<bool, HashError> {
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
#[allow(dead_code)]
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
    #[allow(clippy::cast_possible_truncation)]
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

    #[test]
    fn test_full_hash_parallel_matches_sequential() {
        let dir = tempdir().unwrap();

        // Create a file >1MB to exercise the parallel hashing path
        let content = vec![0xABu8; 2 * 1024 * 1024]; // 2MB
        let path = create_test_file(dir.path(), "large.bin", &content);

        let mut hasher = FileHasher::new();
        let sequential = hasher.full_hash(&path).unwrap();
        let parallel = hasher.full_hash_parallel(&path).unwrap();

        assert_eq!(sequential, parallel);
    }

    #[test]
    fn test_partial_hash_deterministic_across_calls() {
        let dir = tempdir().unwrap();

        // Create a file large enough for partial hashing (>8KB)
        let content = vec![0xCDu8; 16384];
        let path = create_test_file(dir.path(), "repeat.bin", &content);

        let mut hasher = FileHasher::new();
        let hash1 = hasher.partial_hash(&path).unwrap();
        let hash2 = hasher.partial_hash(&path).unwrap();
        let hash3 = hasher.partial_hash(&path).unwrap();

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn test_partial_hash_different_files_interleaved() {
        let dir = tempdir().unwrap();

        // Two distinct files, both large enough for partial hashing
        let content_a = vec![0xAAu8; 16384];
        let content_b = vec![0xBBu8; 16384];
        let path_a = create_test_file(dir.path(), "file_a.bin", &content_a);
        let path_b = create_test_file(dir.path(), "file_b.bin", &content_b);

        let mut hasher = FileHasher::new();

        // Interleave: A, B, A — verify no buffer contamination
        let hash_a1 = hasher.partial_hash(&path_a).unwrap();
        let hash_b = hasher.partial_hash(&path_b).unwrap();
        let hash_a2 = hasher.partial_hash(&path_a).unwrap();

        assert_eq!(hash_a1, hash_a2, "file_a hash should be stable across interleaved calls");
        assert_ne!(hash_a1, hash_b, "different files should produce different hashes");
    }
}
