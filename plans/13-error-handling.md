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

    /// File was moved to a different location since scan
    #[error("File moved: The file has been moved to a different location since the scan: {path}")]
    FileMoved { path: String, hint: String },

    /// File was deleted by user or another application
    #[error("File deleted: The file was deleted outside of DupliFind: {path}")]
    FileDeleted { path: String },

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
    /// Get a user-friendly message explaining what happened and what to do
    pub fn user_message(&self) -> String {
        match self {
            AppError::FileNotFound { path } => {
                format!("The file '{}' no longer exists. It may have been deleted or moved.", path)
            }
            AppError::FileMoved { path, hint } => {
                format!(
                    "The file '{}' has been moved since the last scan. {}. Run a new scan to update file locations.",
                    path, hint
                )
            }
            AppError::FileDeleted { path } => {
                format!(
                    "The file '{}' was deleted outside of DupliFind. It will be removed from the results. No action needed.",
                    path
                )
            }
            AppError::PermissionDenied { path } => {
                format!("Access denied to '{}'. Check file permissions or run DupliFind with appropriate access.", path)
            }
            AppError::FileCorrupted { path } => {
                format!("The file '{}' appears to be corrupted or cannot be read. It will be skipped.", path)
            }
            AppError::FileLocked { path } => {
                format!("The file '{}' is currently in use by another application. Close the other application and try again.", path)
            }
            AppError::FileChanged { path } => {
                format!("The file '{}' has been modified since the scan. Run a new scan to get updated results.", path)
            }
            AppError::DiskFull { message } => {
                format!("Disk space is critically low: {}. Free up disk space before continuing.", message)
            }
            AppError::NetworkDisconnected { path } => {
                format!("The network drive containing '{}' is disconnected. Reconnect the drive and try again.", path)
            }
            AppError::Database { message } => {
                format!("Database error: {}. Try restarting the application.", message)
            }
            AppError::Cancelled => "The operation was cancelled.".to_string(),
            AppError::Unknown { message } => {
                format!("An unexpected error occurred: {}. Please try again.", message)
            }
        }
    }

    /// Get a short action suggestion for the user
    pub fn suggested_action(&self) -> &'static str {
        match self {
            AppError::FileNotFound { .. } | AppError::FileDeleted { .. } => "Remove from results",
            AppError::FileMoved { .. } | AppError::FileChanged { .. } => "Run new scan",
            AppError::PermissionDenied { .. } => "Check permissions",
            AppError::FileCorrupted { .. } => "Skip file",
            AppError::FileLocked { .. } => "Retry later",
            AppError::DiskFull { .. } => "Free disk space",
            AppError::NetworkDisconnected { .. } => "Reconnect drive",
            AppError::Database { .. } => "Restart app",
            AppError::Cancelled => "No action needed",
            AppError::Unknown { .. } => "Retry",
        }
    }

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
                | AppError::FileDeleted { .. }
                | AppError::FileMoved { .. }
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

### Code Review

Run code-review-fix-loop agent.

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

### Code Review

Run code-review-fix-loop agent.

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
          <button class="retry-btn" onclick={() => onRetry([file.path])}> Retry </button>
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

### Code Review

Run code-review-fix-loop agent.

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

  // Import shared format utilities (see plans/issues/2026-02-12-issue-6-12-consolidate-format-tests.md)
  import { formatBytes } from '$lib/utils/format';

  let { availableSpace, onResume, onCancel }: Props = $props();
</script>

<div class="alert-overlay">
  <div class="alert-dialog">
    <div class="alert-icon">⚠️</div>
    <h2>Disk Space Low</h2>

    <p class="message">The scan has been paused because disk space is critically low.</p>

    <div class="space-info">
      <span class="label">Available space:</span>
      <span class="value">{formatBytes(availableSpace)}</span>
    </div>

    <p class="suggestion">
      Please free up some disk space before continuing. The scan needs space to store temporary data
      and the database.
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

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent on disk monitoring code.

---

## Phase 13.5: Add Disk I/O Throttling

### Overview

Implement smart throttling that monitors disk queue depth, adapts to system load, and prevents disk saturation during scans.

### Key Throttling Strategies

1. **Time-based throttling**: Minimum interval between file reads
2. **Queue depth monitoring**: Track pending I/O operations and pause when queue gets too deep
3. **Adaptive throttling**: Adjust delay based on measured read latency
4. **Parallelism mode respect**: Different throttling profiles for light/normal/aggressive modes

### Changes Required

**File**: `src-tauri/src/scanner/throttle.rs`

```rust
//! Disk I/O throttling to prevent system overload during scans

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Configuration for different throttling profiles
#[derive(Debug, Clone, Copy)]
pub struct ThrottleProfile {
    /// Minimum milliseconds between file read operations
    pub min_interval_ms: u64,
    /// Maximum concurrent pending I/O operations before throttling kicks in
    pub max_queue_depth: usize,
    /// Target read latency in milliseconds - throttle if reads are slower than this
    pub target_latency_ms: u64,
    /// How many recent latency samples to track for adaptive throttling
    pub latency_sample_count: usize,
}

impl ThrottleProfile {
    /// Light profile: Minimal system impact
    pub fn light() -> Self {
        Self {
            min_interval_ms: 5,
            max_queue_depth: 4,
            target_latency_ms: 50,
            latency_sample_count: 20,
        }
    }

    /// Normal profile: Balanced performance
    pub fn normal() -> Self {
        Self {
            min_interval_ms: 1,
            max_queue_depth: 16,
            target_latency_ms: 100,
            latency_sample_count: 50,
        }
    }

    /// Aggressive profile: Maximum speed, higher system impact
    pub fn aggressive() -> Self {
        Self {
            min_interval_ms: 0,
            max_queue_depth: 64,
            target_latency_ms: 200,
            latency_sample_count: 100,
        }
    }
}

/// Smart I/O throttler with adaptive behavior
pub struct IoThrottler {
    profile: Mutex<ThrottleProfile>,
    enabled: AtomicBool,
    last_read_time: AtomicU64,
    pending_operations: AtomicUsize,
    // Recent read latencies for adaptive throttling
    latency_samples: Mutex<VecDeque<u64>>,
    // Current adaptive delay (adjusted based on measured latency)
    adaptive_delay_ms: AtomicU64,
}

impl IoThrottler {
    pub fn new() -> Self {
        Self::with_profile(ThrottleProfile::normal())
    }

    pub fn with_profile(profile: ThrottleProfile) -> Self {
        Self {
            profile: Mutex::new(profile),
            enabled: AtomicBool::new(true),
            last_read_time: AtomicU64::new(0),
            pending_operations: AtomicUsize::new(0),
            latency_samples: Mutex::new(VecDeque::with_capacity(100)),
            adaptive_delay_ms: AtomicU64::new(0),
        }
    }

    /// Set the throttling profile
    pub fn set_profile(&self, profile: ThrottleProfile) {
        if let Ok(mut p) = self.profile.lock() {
            *p = profile;
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    /// Call before starting a file read operation
    pub fn begin_read(&self) {
        self.pending_operations.fetch_add(1, Ordering::Relaxed);
    }

    /// Call after completing a file read operation with the duration
    pub fn end_read(&self, latency_ms: u64) {
        self.pending_operations.fetch_sub(1, Ordering::Relaxed);
        self.record_latency(latency_ms);
    }

    /// Record a latency sample and adjust adaptive delay
    fn record_latency(&self, latency_ms: u64) {
        let profile = self.profile.lock().ok();
        let sample_count = profile.as_ref().map(|p| p.latency_sample_count).unwrap_or(50);
        let target_latency = profile.as_ref().map(|p| p.target_latency_ms).unwrap_or(100);

        if let Ok(mut samples) = self.latency_samples.lock() {
            samples.push_back(latency_ms);
            while samples.len() > sample_count {
                samples.pop_front();
            }

            // Calculate average latency
            if samples.len() >= 10 {
                let avg_latency: u64 = samples.iter().sum::<u64>() / samples.len() as u64;

                // If average latency exceeds target, increase adaptive delay
                if avg_latency > target_latency {
                    let new_delay = (avg_latency - target_latency).min(50);
                    self.adaptive_delay_ms.store(new_delay, Ordering::Relaxed);
                } else {
                    // Gradually reduce delay if we're under target
                    let current = self.adaptive_delay_ms.load(Ordering::Relaxed);
                    if current > 0 {
                        self.adaptive_delay_ms.store(current.saturating_sub(1), Ordering::Relaxed);
                    }
                }
            }
        }
    }

    /// Apply throttling - call before each file operation
    pub fn throttle(&self) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }

        let profile = match self.profile.lock() {
            Ok(p) => *p,
            Err(_) => return,
        };

        // 1. Check queue depth - if too many pending ops, wait for some to complete
        let pending = self.pending_operations.load(Ordering::Relaxed);
        if pending >= profile.max_queue_depth {
            // Exponential backoff based on how far over the limit we are
            let overflow = pending - profile.max_queue_depth;
            let backoff_ms = (2u64.pow(overflow.min(5) as u32)).min(100);
            std::thread::sleep(Duration::from_millis(backoff_ms));
        }

        // 2. Apply minimum interval
        if profile.min_interval_ms > 0 {
            let now_ns = Instant::now().elapsed().as_nanos() as u64;
            let last = self.last_read_time.load(Ordering::Relaxed);
            let min_interval_ns = profile.min_interval_ms * 1_000_000;

            if now_ns.saturating_sub(last) < min_interval_ns {
                std::thread::sleep(Duration::from_millis(profile.min_interval_ms));
            }

            self.last_read_time.store(
                Instant::now().elapsed().as_nanos() as u64,
                Ordering::Relaxed,
            );
        }

        // 3. Apply adaptive delay based on measured latency
        let adaptive_delay = self.adaptive_delay_ms.load(Ordering::Relaxed);
        if adaptive_delay > 0 {
            std::thread::sleep(Duration::from_millis(adaptive_delay));
        }
    }

    /// Get current throttling statistics
    pub fn stats(&self) -> ThrottleStats {
        let avg_latency = self.latency_samples.lock()
            .map(|s| {
                if s.is_empty() { 0 } else { s.iter().sum::<u64>() / s.len() as u64 }
            })
            .unwrap_or(0);

        ThrottleStats {
            pending_operations: self.pending_operations.load(Ordering::Relaxed),
            adaptive_delay_ms: self.adaptive_delay_ms.load(Ordering::Relaxed),
            avg_latency_ms: avg_latency,
        }
    }
}

impl Default for IoThrottler {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about current throttling state
#[derive(Debug, Clone)]
pub struct ThrottleStats {
    pub pending_operations: usize,
    pub adaptive_delay_ms: u64,
    pub avg_latency_ms: u64,
}
```

### Integration with Scanner

Update `walker.rs` to use the throttler:

```rust
// In DirectoryWalker::walk_with_callback
let throttler = IoThrottler::with_profile(match self.config.parallelism {
    ParallelismMode::Light => ThrottleProfile::light(),
    ParallelismMode::Normal => ThrottleProfile::normal(),
    ParallelismMode::Aggressive => ThrottleProfile::aggressive(),
});

for entry in walker {
    // Apply throttling before reading file
    throttler.throttle();
    throttler.begin_read();

    let start = Instant::now();
    // ... read file metadata / hash ...
    let latency = start.elapsed().as_millis() as u64;

    throttler.end_read(latency);
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo check` passes
- [ ] `cargo test throttle` passes

#### Manual Verification

- [ ] Scan respects parallelism mode settings
- [ ] System remains responsive during "Light" mode scan
- [ ] Throttling adapts when disk becomes saturated
- [ ] Queue depth is respected (pending operations don't exceed max)

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

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

### Code Review

Run code-review-fix-loop agent.

---

## Phase 13.7: Tests

Add tests for:

- Error handling
- Skipped files management
- Throttling
- Incremental scanning

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

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
