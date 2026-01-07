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

    #[error("Disk full")]
    DiskFull,

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

## Phase 13.4: Add Disk I/O Throttling

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

## Phase 13.5: Implement Incremental Scanning

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

## Phase 13.6: Tests

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
- Disk I/O throttling
- Incremental scanning with hash cache
- Quick scan vs full rescan options

**Next**: Proceed to [14-platform-polish.md](./14-platform-polish.md)
