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

    /// Get the "original" file (marked during detection)
    #[allow(dead_code)]
    pub fn original(&self) -> Option<&DuplicateFile> {
        self.files.iter().find(|f| f.is_original)
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

    /// Get a cancellation handle (used in tests)
    #[cfg(test)]
    pub fn cancel_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.cancelled)
    }

    /// Set an external cancellation flag (e.g., from scan state)
    pub fn set_cancel_flag(&mut self, flag: Arc<AtomicBool>) {
        self.cancelled = flag;
    }

    /// Detect duplicates from a list of files
    pub fn detect(&mut self, files: Vec<FileInfo>) -> Result<DetectionResult, ScanError> {
        let mut stats = DetectionStats::default();

        // Track total non-symlink files for unique_files calculation
        let total_files = files.iter().filter(|f| !f.is_symlink).count() as u64;

        // Stage 1: Group by size
        let start = std::time::Instant::now();
        let size_groups = Self::group_by_size(files);
        #[allow(clippy::cast_possible_truncation)]
        {
            stats.size_grouping_ms = start.elapsed().as_millis() as u64;
        }
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
        #[allow(clippy::cast_possible_truncation)]
        {
            stats.partial_hashing_ms = start.elapsed().as_millis() as u64;
        }

        if self.cancelled.load(Ordering::Relaxed) {
            return Err(ScanError::Cancelled);
        }

        // Stage 3: Verify with full hash
        let start = std::time::Instant::now();
        let duplicate_groups = self.verify_with_full_hash(partial_groups, &mut stats)?;
        #[allow(clippy::cast_possible_truncation)]
        {
            stats.full_hashing_ms = start.elapsed().as_millis() as u64;
        }

        // Calculate totals
        let duplicate_count: u64 = duplicate_groups
            .iter()
            .map(|g| (g.files.len() - 1) as u64)
            .sum();
        let total_wasted_space: u64 = duplicate_groups.iter().map(|g| g.wasted_space).sum();
        let unique_files = total_files.saturating_sub(duplicate_count);

        Ok(DetectionResult {
            groups: duplicate_groups,
            duplicate_count,
            total_wasted_space,
            unique_files,
            stats,
        })
    }

    /// Stage 1: Group files by size
    fn group_by_size(files: Vec<FileInfo>) -> HashMap<u64, Vec<FileInfo>> {
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
                    let mut dup_files: Vec<DuplicateFile> =
                        files.into_iter().map(DuplicateFile::from).collect();

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
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().cast_signed())
            .unwrap_or(0);
        let modified = metadata
            .modified()
            .map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs().cast_signed())
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

        let file1 = create_test_file(dir.path(), "newer.txt", content);
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

        // Should be sorted by wasted space descending (1000 > 200)
        assert!(result.groups[0].wasted_space > result.groups[1].wasted_space);
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

    #[test]
    fn test_large_file_detection() {
        let dir = tempdir().unwrap();

        // Create larger files (16KB) to test partial hash path
        let large_content: Vec<u8> = (0..16384).map(|i| (i % 256) as u8).collect();

        let files = vec![
            create_test_file(dir.path(), "large1.bin", &large_content),
            create_test_file(dir.path(), "large2.bin", &large_content),
        ];

        let mut detector = DuplicateDetector::new();
        let result = detector.detect(files).unwrap();

        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].files.len(), 2);
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
            create_test_file(dir.path(), "false_pos1.bin", &content1),
            create_test_file(dir.path(), "false_pos2.bin", &content2),
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
            .map(|i| create_test_file(dir.path(), &format!("file{i}.txt"), content))
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
        assert!(result.stats.partial_hashes > 0);
        assert!(result.stats.full_hashes > 0);
    }
}
