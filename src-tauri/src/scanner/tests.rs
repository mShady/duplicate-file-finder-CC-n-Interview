//! Integration tests for the scanner module

#[cfg(test)]
mod integration_tests {
    use crate::scanner::{DirectoryWalker, FileInfo, ParallelismMode, ScanConfig};
    use std::fs::{self, File};
    use std::io::Write;
    use tempfile::tempdir;

    fn create_test_structure() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        let root = dir.path();

        // Create directory structure
        // root/
        //   file1.txt (10 bytes)
        //   file2.txt (20 bytes)
        //   subdir1/
        //     file3.txt (15 bytes)
        //     file4.txt (25 bytes)
        //   subdir2/
        //     deep/
        //       file5.txt (30 bytes)
        //   empty_dir/

        // Root files
        let mut f1 = File::create(root.join("file1.txt")).unwrap();
        f1.write_all(&[0u8; 10]).unwrap();

        let mut f2 = File::create(root.join("file2.txt")).unwrap();
        f2.write_all(&[0u8; 20]).unwrap();

        // subdir1
        fs::create_dir(root.join("subdir1")).unwrap();
        let mut f3 = File::create(root.join("subdir1/file3.txt")).unwrap();
        f3.write_all(&[0u8; 15]).unwrap();

        let mut f4 = File::create(root.join("subdir1/file4.txt")).unwrap();
        f4.write_all(&[0u8; 25]).unwrap();

        // subdir2/deep
        fs::create_dir_all(root.join("subdir2/deep")).unwrap();
        let mut f5 = File::create(root.join("subdir2/deep/file5.txt")).unwrap();
        f5.write_all(&[0u8; 30]).unwrap();

        // empty_dir
        fs::create_dir(root.join("empty_dir")).unwrap();

        dir
    }

    #[test]
    fn test_walk_all_files() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        assert_eq!(result.files.len(), 5, "Should find 5 files");
        assert_eq!(result.stats.total_files, 5);
        assert_eq!(result.stats.total_bytes, 100); // 10 + 20 + 15 + 25 + 30
    }

    #[test]
    fn test_walk_with_depth_limit() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: Some(2), // Only root and immediate subdirs
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        // Should find root files and subdir1 files, but not subdir2/deep/file5.txt
        assert_eq!(
            result.files.len(),
            4,
            "Should find 4 files with depth limit 2"
        );
    }

    #[test]
    fn test_walk_multiple_paths() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![
                test_dir.path().join("subdir1"),
                test_dir.path().join("subdir2"),
            ],
            follow_symlinks: false,
            max_depth: None,
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        assert_eq!(
            result.files.len(),
            3,
            "Should find 3 files in subdir1 and subdir2"
        );
    }

    #[test]
    fn test_file_sizes_correct() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: Some(1),
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        // Find file1.txt
        let file1 = result
            .files
            .iter()
            .find(|f| f.path.file_name().unwrap() == "file1.txt")
            .unwrap();

        assert_eq!(file1.size, 10);
    }

    #[test]
    fn test_progress_tracking() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);

        // Check initial progress
        let progress = walker.progress();
        assert_eq!(progress.total_files, 0);

        // Walk and check final progress matches result
        let result = walker.walk().unwrap();
        assert_eq!(result.stats.total_files, 5);
    }

    #[test]
    #[cfg(unix)]
    fn test_symlink_skipping() {
        use std::os::unix::fs::symlink;
        let test_dir = create_test_structure();
        let link_path = test_dir.path().join("link_to_file1");
        symlink(test_dir.path().join("file1.txt"), &link_path).unwrap();

        let config = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: Some(1),
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        // Should not include the symlink
        let has_symlink = result
            .files
            .iter()
            .any(|f| f.path.file_name().unwrap() == "link_to_file1");
        assert!(!has_symlink, "Symlink should be skipped");

        // Stats should track skipped symlinks
        assert_eq!(result.stats.symlinks_skipped, 1);
    }

    #[test]
    fn test_empty_directory() {
        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().join("empty_dir")],
            follow_symlinks: false,
            max_depth: None,
            parallelism: ParallelismMode::Light,
        };

        let walker = DirectoryWalker::new(config);
        let result = walker.walk().unwrap();

        assert_eq!(
            result.files.len(),
            0,
            "Empty directory should have no files"
        );
    }

    #[test]
    fn test_parallelism_modes() {
        // Verify the thread counts are reasonable
        let light = ParallelismMode::Light.thread_count();
        let normal = ParallelismMode::Normal.thread_count();
        let aggressive = ParallelismMode::Aggressive.thread_count();

        assert!(light <= 2);
        assert!(normal >= light);
        assert!(aggressive >= normal);
    }

    #[test]
    fn test_file_info_metadata() {
        let test_dir = create_test_structure();
        let file_path = test_dir.path().join("file1.txt");

        let info = FileInfo::from_path(file_path).unwrap();

        assert_eq!(info.size, 10);
        assert!(!info.is_symlink);
        assert!(info.created_at > 0 || info.modified_at > 0);
    }

    #[test]
    fn test_walk_with_callback_error_handling() {
        use crate::scanner::ScanError;

        let test_dir = create_test_structure();

        let config = ScanConfig {
            paths: vec![test_dir.path().to_path_buf()],
            follow_symlinks: false,
            max_depth: None,
            parallelism: ParallelismMode::Normal,
        };

        let walker = DirectoryWalker::new(config);
        let mut call_count = 0u32;

        // Callback that fails on every file
        let result = walker.walk_with_callback(|_file_info| {
            call_count += 1;
            Err(ScanError::Path("intentional test error".to_string()))
        });

        // Walk should succeed even though all callbacks failed
        let _stats = result.unwrap();

        // All files were visited (callback was called) but all errored
        assert!(call_count > 0, "callback should have been called at least once");

        // The walker counts callback errors as skipped files via progress tracker.
        // stats.errors comes from WalkDir entry errors, not callback errors.
        // This documents the current behavior for the safety net.
    }
}
