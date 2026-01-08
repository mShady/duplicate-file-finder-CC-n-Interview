# File 13: Error Handling & Performance

## Overview

This file covers implementing comprehensive error handling, skip/retry for unreadable files, disk I/O throttling, and incremental scanning optimizations.

## Prerequisites

- Completed Files 01-12

---

## Phase 13.1: Create Error Types and Handling

### Overview
Create comprehensive error types and standardized error handling.

### Changes Required

**File**: `src-tauri/src/error.rs`

```rust
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Serialize)]
#[serde(tag = "type", content = "details")]
pub enum AppError {
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    #[error("Permission denied: {path}")]
    PermissionDenied { path: String },

    #[error("File corrupted or unreadable: {path}")]
    FileCorrupted { path: String },

    #[error("File locked by another application: {path}")]
    FileLocked { path: String },

    #[error("File changed since scan: {path}")]
    FileChanged { path: String },

    #[error("Disk full: {message}")]
    DiskFull { message: String },

    #[error("Network drive disconnected: {path}")]
    NetworkDisconnected { path: String },

    #[error("Database error: {message}")]
    Database { message: String },

    #[error("Scan cancelled")]
    Cancelled,

    #[error("Unknown error: {message}")]
    Unknown { message: String },
}

impl AppError {
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AppError::FileLocked { .. } | AppError::NetworkDisconnected { .. }
        )
    }

    pub fn is_skippable(&self) -> bool {
        matches!(
            self,
            AppError::PermissionDenied { .. }
                | AppError::FileCorrupted { .. }
                | AppError::FileLocked { .. }
        )
    }

    /// Returns true if this error should pause the scan
    pub fn should_pause_scan(&self) -> bool {
        matches!(self, AppError::DiskFull { .. })
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::NotFound => AppError::FileNotFound {
                path: "unknown".to_string(),
            },
            std::io::ErrorKind::PermissionDenied => AppError::PermissionDenied {
                path: "unknown".to_string(),
            },
            _ => AppError::Unknown {
                message: err.to_string(),
            },
        }
    }
}
```

### Commit
Execute `/cl:commit`

---

## Phase 13.2: Create Skipped Files Manager

### Overview
Track and manage skipped files during scanning.

### Changes Required

**File**: `src-tauri/src/scanner/skipped.rs`

```rust
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedFile {
    pub path: PathBuf,
    pub reason: String,
    pub error_type: String,
    pub retryable: bool,
    pub skipped_at: i64,
}

pub struct SkippedFilesManager {
    files: Arc<Mutex<HashMap<PathBuf, SkippedFile>>>,
}

impl SkippedFilesManager {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn add(&self, path: PathBuf, error: &AppError) {
        let skipped = SkippedFile {
            path: path.clone(),
            reason: error.to_string(),
            error_type: format!("{:?}", error),
            retryable: error.is_retryable(),
            skipped_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64,
        };

        if let Ok(mut files) = self.files.lock() {
            files.insert(path, skipped);
        }
    }

    pub fn get_all(&self) -> Vec<SkippedFile> {
        self.files
            .lock()
            .map(|f| f.values().cloned().collect())
            .unwrap_or_default()
    }

    pub fn get_retryable(&self) -> Vec<PathBuf> {
        self.files
            .lock()
            .map(|f| {
                f.values()
                    .filter(|s| s.retryable)
                    .map(|s| s.path.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn count(&self) -> usize {
        self.files.lock().map(|f| f.len()).unwrap_or(0)
    }

    pub fn clear(&self) {
        if let Ok(mut files) = self.files.lock() {
            files.clear();
        }
    }
}

impl Default for SkippedFilesManager {
    fn default() -> Self {
        Self::new()
    }
}
```

### Commit
Execute `/cl:commit`

---

## Phase 13.3: Create Skipped Files UI

### Overview
Create UI for viewing and retrying skipped files.

### Changes Required

**File**: `src/lib/components/SkippedFilesPanel.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface SkippedFile {
    path: string;
    reason: string;
    retryable: boolean;
  }

  interface Props {
    files: SkippedFile[];
    onRetry: (paths: string[]) => void;
    onClose: () => void;
  }

  let { files, onRetry, onClose }: Props = $props();

  let retryableCount = $derived(files.filter((f) => f.retryable).length);

  function retryAll() {
    const paths = files.filter((f) => f.retryable).map((f) => f.path);
    onRetry(paths);
  }
</script>

<div class="panel">
  <div class="header">
    <h2>Skipped Files ({files.length})</h2>
    <button class="close-btn" onclick={onClose}>Close</button>
  </div>

  <p class="description">
    These files were skipped during the scan due to errors.
    {#if retryableCount > 0}
      {retryableCount} file(s) can be retried.
    {/if}
  </p>

  {#if retryableCount > 0}
    <button class="retry-all-btn" onclick={retryAll}>
      Retry {retryableCount} File(s)
    </button>
  {/if}

  <div class="files-list">
    {#each files as file}
      <div class="file-item" class:retryable={file.retryable}>
        <div class="file-path">{file.path}</div>
        <div class="file-reason">{file.reason}</div>
        {#if file.retryable}
          <button class="retry-btn" onclick={() => onRetry([file.path])}>
            Retry
          </button>
        {/if}
      </div>
    {/each}
  </div>
</div>

<style>
  .panel {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
    max-height: 400px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .description {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
  }

  .retry-all-btn {
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    margin-bottom: 1rem;
  }

  .files-list {
    flex: 1;
    overflow-y: auto;
  }

  .file-item {
    padding: 0.75rem;
    background: var(--background);
    border-radius: 4px;
    margin-bottom: 0.5rem;
  }

  .file-item.retryable {
    border-left: 3px solid var(--warning);
  }

  .file-path {
    font-family: var(--font-mono);
    font-size: 0.8rem;
    word-break: break-all;
  }

  .file-reason {
    font-size: 0.75rem;
    color: var(--error);
    margin-top: 0.25rem;
  }

  .retry-btn {
    margin-top: 0.5rem;
    padding: 0.25rem 0.5rem;
    background: var(--warning);
    color: white;
    border: none;
    border-radius: 3px;
    cursor: pointer;
    font-size: 0.75rem;
  }
</style>
```

### Commit
Execute `/cl:commit`

---

## Phase 13.4: Add Disk Full Detection and Handling

### Overview
Detect disk full conditions during scan and automatically pause with user notification.

### Changes Required

#### 13.4.1 Create Disk Space Monitor

**File**: `src-tauri/src/scanner/disk_monitor.rs`

```rust
//! Disk space monitoring during scans

use std::path::Path;

#[cfg(target_os = "macos")]
use std::os::unix::fs::MetadataExt;

/// Minimum free space threshold (100 MB)
const MIN_FREE_SPACE_BYTES: u64 = 100 * 1024 * 1024;

/// Check available disk space for a given path
pub fn get_available_space(path: &Path) -> Result<u64, std::io::Error> {
    #[cfg(target_os = "macos")]
    {
        use std::ffi::CString;
        use std::mem::MaybeUninit;

        let path_str = path.to_string_lossy();
        let c_path = CString::new(path_str.as_bytes())?;

        unsafe {
            let mut stat: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
            if libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) == 0 {
                let stat = stat.assume_init();
                Ok(stat.f_bavail as u64 * stat.f_frsize as u64)
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::GetDiskFreeSpaceExW;
        use windows::core::PCWSTR;

        let path_wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();

        unsafe {
            let mut free_bytes: u64 = 0;
            let result = GetDiskFreeSpaceExW(
                PCWSTR::from_raw(path_wide.as_ptr()),
                Some(&mut free_bytes as *mut u64),
                None,
                None,
            );

            if result.is_ok() {
                Ok(free_bytes)
            } else {
                Err(std::io::Error::last_os_error())
            }
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // Fallback for other platforms
        Ok(u64::MAX)
    }
}

/// Check if disk space is critically low
pub fn is_disk_space_low(path: &Path) -> bool {
    match get_available_space(path) {
        Ok(available) => available < MIN_FREE_SPACE_BYTES,
        Err(_) => false, // If we can't check, assume it's fine
    }
}

/// Disk space status
#[derive(Debug, Clone)]
pub struct DiskSpaceStatus {
    pub available_bytes: u64,
    pub is_low: bool,
    pub path: String,
}

impl DiskSpaceStatus {
    pub fn check(path: &Path) -> Self {
        let available_bytes = get_available_space(path).unwrap_or(u64::MAX);
        Self {
            available_bytes,
            is_low: available_bytes < MIN_FREE_SPACE_BYTES,
            path: path.display().to_string(),
        }
    }
}
```

#### 13.4.2 Integrate Disk Monitoring in Scanner

Update the scanner to check disk space periodically and pause if low:

**File**: `src-tauri/src/scanner/walker.rs` (add to walk methods)

```rust
// Check disk space every 1000 files
if self.progress.total_files.load(Ordering::Relaxed) % 1000 == 0 {
    if let Some(first_path) = self.config.paths.first() {
        if disk_monitor::is_disk_space_low(first_path) {
            // Emit disk full event
            log::warn!("Disk space low, pausing scan");
            self.paused.store(true, Ordering::Relaxed);

            // The frontend will be notified via event
            return Err(ScanError::DiskFull("Disk space critically low".to_string()));
        }
    }
}
```

#### 13.4.3 Create Disk Full Alert Component

**File**: `src/lib/components/DiskFullAlert.svelte`

```svelte
<script lang="ts">
  interface Props {
    availableSpace: number;
    onResume: () => void;
    onCancel: () => void;
  }

  let { availableSpace, onResume, onCancel }: Props = $props();

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
</script>

<div class="alert-overlay">
  <div class="alert-dialog">
    <div class="alert-icon">⚠️</div>
    <h2>Disk Space Low</h2>

    <p class="message">
      The scan has been paused because disk space is critically low.
    </p>

    <div class="space-info">
      <span class="label">Available space:</span>
      <span class="value">{formatBytes(availableSpace)}</span>
    </div>

    <p class="suggestion">
      Please free up some disk space before continuing. The scan needs space
      to store temporary data and the database.
    </p>

    <div class="actions">
      <button class="cancel-btn" onclick={onCancel}>Cancel Scan</button>
      <button class="resume-btn" onclick={onResume}>Check & Resume</button>
    </div>
  </div>
</div>

<style>
  .alert-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 2000;
  }

  .alert-dialog {
    background: var(--surface);
    border-radius: 12px;
    padding: 2rem;
    max-width: 400px;
    width: 90%;
    text-align: center;
  }

  .alert-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  h2 {
    margin: 0 0 1rem;
    color: var(--warning);
  }

  .message {
    color: var(--text);
    margin-bottom: 1rem;
  }

  .space-info {
    background: var(--background);
    padding: 1rem;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .space-info .label {
    color: var(--text-secondary);
  }

  .space-info .value {
    font-weight: 600;
    color: var(--error);
    margin-left: 0.5rem;
  }

  .suggestion {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 1.5rem;
  }

  .actions {
    display: flex;
    gap: 1rem;
    justify-content: center;
  }

  button {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
  }

  .cancel-btn {
    background: var(--error);
    color: white;
  }

  .resume-btn {
    background: var(--primary);
    color: white;
  }
</style>
```

#### 13.4.4 Add Disk Full Event Handling

Update the scan commands to emit a disk-full event:

**File**: `src-tauri/src/commands/scan.rs` (add to error handling)

```rust
// When disk full error occurs
if let Err(ScanError::DiskFull(msg)) = &result {
    let _ = app_handle.emit("disk-full", serde_json::json!({
        "message": msg,
        "available_space": disk_monitor::get_available_space(
            config.paths.first().unwrap_or(&PathBuf::from("/"))
        ).unwrap_or(0),
    }));
}
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check` passes
- [ ] `cargo test disk_monitor` passes

#### Manual Verification
- [ ] Scan pauses when disk space is low
- [ ] Alert dialog appears with available space info
- [ ] "Check & Resume" verifies space before resuming
- [ ] Scan can be cancelled from the alert

### Code Review
Run background code-reviewer agent on disk monitoring code.

### Commit
Execute `/cl:commit`

---

## Phase 13.5: Add Disk I/O Throttling

### Overview
Implement smart throttling that monitors disk queue depth.

### Changes Required

**File**: `src-tauri/src/scanner/throttle.rs`

```rust
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct IoThrottler {
    last_read_time: AtomicU64,
    min_interval_ms: AtomicU64,
    enabled: AtomicBool,
}

impl IoThrottler {
    pub fn new() -> Self {
        Self {
            last_read_time: AtomicU64::new(0),
            min_interval_ms: AtomicU64::new(0),
            enabled: AtomicBool::new(true),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn set_min_interval(&self, ms: u64) {
        self.min_interval_ms.store(ms, Ordering::Relaxed);
    }

    pub fn throttle(&self) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let min_interval = self.min_interval_ms.load(Ordering::Relaxed);
        if min_interval == 0 {
            return;
        }

        let now = Instant::now();
        let last = self.last_read_time.load(Ordering::Relaxed);

        // Simple time-based throttling
        let elapsed = now.elapsed().as_millis() as u64;
        if elapsed < min_interval {
            std::thread::sleep(Duration::from_millis(min_interval - elapsed));
        }

        self.last_read_time
            .store(now.elapsed().as_nanos() as u64, Ordering::Relaxed);
    }
}

impl Default for IoThrottler {
    fn default() -> Self {
        Self::new()
    }
}
```

### Commit
Execute `/cl:commit`

---

## Phase 13.6: Implement Incremental Scanning

### Overview
Use cached hashes for quick scans, only rehash new/modified files.

### Changes Required

Update scanner to:
1. Check file cache for existing hashes
2. Compare path + size + mtime to determine if rehash needed
3. Store new hashes in cache
4. Provide "quick scan" vs "full rescan" options

### Commit
Execute `/cl:commit`

---

## Phase 13.7: Tests

Add tests for:
- Error handling
- Skipped files management
- Throttling
- Incremental scanning

### Commit
Execute `/cl:commit`

---

## End of File 13

After completing all phases:
- Comprehensive error types
- Skip/retry for unreadable files
- Skipped files UI
- Disk full detection and auto-pause with notification
- Disk I/O throttling
- Incremental scanning with hash cache
- Quick scan vs full rescan options

**Next**: Proceed to [14-platform-polish.md](./14-platform-polish.md)
