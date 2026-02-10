# File 06: File Selection & Deletion

## Overview

This file covers implementing the complete file selection system and batch deletion workflow, including pre-delete verification, trash integration, and post-deletion summary.

## Prerequisites

- Completed Files 01-05

---

## Phase 6.1: Add Trash Dependencies

### Overview
Add the trash crate for cross-platform trash/recycle bin support.

### Changes Required

#### 6.1.1 Update Cargo.toml

**File**: `src-tauri/Cargo.toml`

```toml
[dependencies]
# ... existing dependencies ...

# Trash integration
trash = "5.2"
```

### Success Criteria
- [ ] `cargo check` passes

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 6.2: Create Deletion Service

### Overview
Create the service that handles file deletion with pre-verification.

### Changes Required

#### 6.2.1 Create Deletion Module

**File**: `src-tauri/src/services/mod.rs`

```rust
pub mod deletion;
```

**File**: `src-tauri/src/services/deletion.rs`

```rust
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
```

### Success Criteria
- [ ] `cargo check` passes
- [ ] `cargo test deletion` passes

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 6.3: Create Deletion Commands

### Overview
Create Tauri commands for file deletion.

### Changes Required

#### 6.3.1 Create Deletion Commands

**File**: `src-tauri/src/commands/deletion.rs`

```rust
//! Deletion Tauri commands

use crate::db::queries;
use crate::services::deletion::{BatchDeletionResult, DeletionRequest, DeletionService};
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Deserialize)]
pub struct DeleteFilesRequest {
    pub files: Vec<DeletionRequest>,
}

#[derive(Debug, Serialize)]
pub struct DeleteFilesResponse {
    pub result: BatchDeletionResult,
    pub message: String,
}

/// Delete files to trash
#[tauri::command]
pub async fn delete_files(
    request: DeleteFilesRequest,
    state: State<'_, Mutex<AppState>>,
) -> Result<DeleteFilesResponse, String> {
    if request.files.is_empty() {
        return Err("No files to delete".to_string());
    }

    // Check for protected paths
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    for file in &request.files {
        let is_protected = {
            let db = db.blocking_lock();
            tauri::async_runtime::block_on(async {
                queries::protected_folders::is_protected(db.pool(), &file.path).await
            })
            .map_err(|e| e.to_string())?
        };

        if is_protected {
            return Err(format!("Cannot delete protected file: {}", file.path));
        }
    }

    // Perform deletion
    let mut service = DeletionService::new();
    let result = service.delete_batch(request.files);

    // Record deletions in history
    {
        let db = db.blocking_lock();
        for deleted in &result.successful {
            let _ = tauri::async_runtime::block_on(async {
                queries::deletion_history::record(
                    db.pool(),
                    &deleted.path,
                    deleted.size as i64,
                    "", // Hash would need to be passed
                    None,
                    None,
                    None,
                )
                .await
            });
        }
    }

    let message = format!(
        "Deleted {} files, freed {} bytes. {} failed.",
        result.successful.len(),
        result.total_freed,
        result.failed.len()
    );

    Ok(DeleteFilesResponse { result, message })
}

/// Get deletion history
#[tauri::command]
pub async fn get_deletion_history(
    limit: i32,
    offset: i32,
    state: State<'_, Mutex<AppState>>,
) -> Result<Vec<crate::db::models::DeletionRecord>, String> {
    let db = {
        let state = state.lock().map_err(|e| e.to_string())?;
        state.database().ok_or("Database not initialized")?
    };

    let db = db.lock().await;
    queries::deletion_history::get_history(db.pool(), limit, offset)
        .await
        .map_err(|e| e.to_string())
}
```

#### 6.3.2 Update Commands Module

**File**: `src-tauri/src/commands/mod.rs`

Add:
```rust
pub mod deletion;
pub use deletion::*;
```

#### 6.3.3 Update lib.rs

Register the new commands in the invoke handler.

### Success Criteria
- [ ] `cargo check` passes

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 6.4: Create Deletion Confirmation Dialog

### Overview
Create the frontend deletion confirmation dialog.

### Changes Required

#### 6.4.1 Create Confirmation Dialog

**File**: `src/lib/components/DeleteConfirmDialog.svelte`

```svelte
<script lang="ts">
  interface Props {
    fileCount: number;
    totalSize: number;
    sampleFiles: string[];
    allInGroup: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { fileCount, totalSize, sampleFiles, allInGroup, onConfirm, onCancel }: Props = $props();

  // Extra confirmation required when deleting all copies
  let confirmAllCopies = $state(false);

  // Determine if confirm button should be enabled
  let canConfirm = $derived(!allInGroup || confirmAllCopies);

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
</script>

<div class="dialog-overlay" onclick={onCancel}>
  <div class="dialog" onclick={(e) => e.stopPropagation()}>
    <h2>Confirm Deletion</h2>

    {#if allInGroup}
      <div class="danger-banner">
        <div class="danger-icon">⚠️</div>
        <div class="danger-content">
          <strong>DANGER: You are deleting ALL copies!</strong>
          <p>This will permanently remove these files from your system. There will be NO remaining copies anywhere.</p>
        </div>
      </div>

      <div class="confirmation-checkbox">
        <label>
          <input type="checkbox" bind:checked={confirmAllCopies} />
          <span>I understand that ALL copies will be deleted and this action cannot be undone</span>
        </label>
      </div>
    {/if}

    <div class="summary">
      <p>
        <strong>{fileCount}</strong> files will be moved to Trash
        ({formatBytes(totalSize)})
      </p>
    </div>

    <div class="sample-files">
      <p>Files to delete:</p>
      <ul>
        {#each sampleFiles.slice(0, 5) as file}
          <li>{file}</li>
        {/each}
        {#if sampleFiles.length > 5}
          <li class="more">...and {sampleFiles.length - 5} more</li>
        {/if}
      </ul>
    </div>

    <div class="note">
      Files will be moved to the system Trash. You can restore them from there if needed.
    </div>

    <div class="actions">
      <button class="cancel-btn" onclick={onCancel}>Cancel</button>
      <button
        class="confirm-btn"
        onclick={onConfirm}
        disabled={!canConfirm}
        class:disabled={!canConfirm}
      >
        {allInGroup ? 'Delete ALL Copies' : 'Delete to Trash'}
      </button>
    </div>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--surface);
    border-radius: 12px;
    padding: 1.5rem;
    max-width: 500px;
    width: 90%;
  }

  h2 {
    margin: 0 0 1rem;
  }

  .warning-banner {
    background: var(--warning-bg);
    color: var(--warning);
    padding: 0.75rem;
    border-radius: 6px;
    margin-bottom: 1rem;
    font-weight: 500;
  }

  /* Stronger danger banner for deleting all copies */
  .danger-banner {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
    background: var(--error);
    color: white;
    padding: 1rem;
    border-radius: 8px;
    margin-bottom: 1rem;
  }

  .danger-icon {
    font-size: 2rem;
    line-height: 1;
  }

  .danger-content strong {
    display: block;
    font-size: 1.1rem;
    margin-bottom: 0.25rem;
  }

  .danger-content p {
    margin: 0;
    font-size: 0.9rem;
    opacity: 0.9;
  }

  .confirmation-checkbox {
    background: var(--error-bg);
    border: 2px solid var(--error);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .confirmation-checkbox label {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .confirmation-checkbox input[type="checkbox"] {
    width: 1.25rem;
    height: 1.25rem;
    margin-top: 0.125rem;
    flex-shrink: 0;
    cursor: pointer;
  }

  .confirm-btn.disabled,
  .confirm-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .summary {
    margin-bottom: 1rem;
  }

  .sample-files {
    background: var(--background);
    padding: 0.75rem;
    border-radius: 6px;
    margin-bottom: 1rem;
    max-height: 200px;
    overflow-y: auto;
  }

  .sample-files ul {
    margin: 0.5rem 0 0;
    padding-left: 1.5rem;
    font-size: 0.85rem;
    font-family: var(--font-mono);
  }

  .sample-files .more {
    color: var(--text-secondary);
    font-style: italic;
  }

  .note {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 1.5rem;
  }

  .actions {
    display: flex;
    gap: 0.75rem;
    justify-content: flex-end;
  }

  button {
    padding: 0.75rem 1.5rem;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
  }

  .cancel-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
  }

  .confirm-btn {
    background: var(--error);
    border: none;
    color: white;
  }
</style>
```

### Success Criteria
- [ ] `npm run check` passes

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 6.5: Create Deletion Summary Dialog

### Overview
Create the post-deletion summary dialog.

### Changes Required

#### 6.5.1 Create Summary Dialog

**File**: `src/lib/components/DeleteSummaryDialog.svelte`

```svelte
<script lang="ts">
  import type { BatchDeletionResult } from '$lib/types';

  interface Props {
    result: BatchDeletionResult;
    onClose: () => void;
  }

  let { result, onClose }: Props = $props();

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
</script>

<div class="dialog-overlay" onclick={onClose}>
  <div class="dialog" onclick={(e) => e.stopPropagation()}>
    <h2>Deletion Complete</h2>

    <div class="summary">
      <div class="stat success">
        <span class="value">{result.successful.length}</span>
        <span class="label">Files deleted</span>
      </div>
      <div class="stat">
        <span class="value">{formatBytes(result.total_freed)}</span>
        <span class="label">Space freed</span>
      </div>
      {#if result.failed.length > 0}
        <div class="stat error">
          <span class="value">{result.failed.length}</span>
          <span class="label">Failed</span>
        </div>
      {/if}
    </div>

    {#if result.failed.length > 0}
      <div class="failed-section">
        <h3>Failed Deletions</h3>
        <ul>
          {#each result.failed as item}
            <li>
              <span class="path">{item.path}</span>
              <span class="error">{item.error}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    <div class="note">
      Deleted files have been moved to your system Trash. You can restore them from there if needed.
    </div>

    <button class="close-btn" onclick={onClose}>Done</button>
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--surface);
    border-radius: 12px;
    padding: 1.5rem;
    max-width: 500px;
    width: 90%;
  }

  h2 {
    margin: 0 0 1rem;
  }

  .summary {
    display: flex;
    gap: 2rem;
    margin-bottom: 1.5rem;
  }

  .stat {
    text-align: center;
  }

  .stat .value {
    display: block;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .stat .label {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .stat.success .value {
    color: var(--success);
  }

  .stat.error .value {
    color: var(--error);
  }

  .failed-section {
    background: var(--error-bg);
    padding: 1rem;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .failed-section h3 {
    margin: 0 0 0.5rem;
    font-size: 0.9rem;
    color: var(--error);
  }

  .failed-section ul {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 0.85rem;
  }

  .failed-section li {
    margin-bottom: 0.5rem;
  }

  .failed-section .path {
    display: block;
    font-family: var(--font-mono);
  }

  .failed-section .error {
    color: var(--error);
    font-size: 0.8rem;
  }

  .note {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 1.5rem;
  }

  .close-btn {
    width: 100%;
    padding: 0.75rem;
    background: var(--primary);
    border: none;
    border-radius: 6px;
    color: white;
    font-weight: 500;
    cursor: pointer;
  }
</style>
```

### Success Criteria
- [ ] `npm run check` passes

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 6.6: Add Smart Selection Options

### Overview
Add selection helpers: select all except original, select by location, select by path depth, etc.

### Changes Required

#### 6.6.1 Create Selection Utilities

**File**: `src/lib/utils/selection.ts`

```typescript
import type { DuplicateFile, DuplicateGroup } from '$lib/types';

/**
 * Select all files except the oldest (original) in each group
 */
export function selectAllExceptOldest(groups: DuplicateGroup[]): Set<string> {
  const selected = new Set<string>();

  for (const group of groups) {
    // Sort by creation date, oldest first
    const sorted = [...group.files].sort((a, b) => a.created_at - b.created_at);

    // Select all except the first (oldest)
    for (let i = 1; i < sorted.length; i++) {
      selected.add(sorted[i].path);
    }
  }

  return selected;
}

/**
 * Select all files in a specific folder path
 */
export function selectByLocation(
  groups: DuplicateGroup[],
  folderPath: string,
  currentSelection: Set<string>
): Set<string> {
  const selected = new Set(currentSelection);

  for (const group of groups) {
    for (const file of group.files) {
      if (file.path.startsWith(folderPath)) {
        selected.add(file.path);
      }
    }
  }

  return selected;
}

/**
 * Select files by path depth (number of directory levels)
 * Useful for selecting files in deeper nested directories
 */
export function selectByPathDepth(
  groups: DuplicateGroup[],
  minDepth: number,
  maxDepth: number | null,
  currentSelection: Set<string>
): Set<string> {
  const selected = new Set(currentSelection);

  function getPathDepth(path: string): number {
    // Count directory separators
    const separator = path.includes('/') ? '/' : '\\';
    return path.split(separator).filter(Boolean).length;
  }

  for (const group of groups) {
    for (const file of group.files) {
      const depth = getPathDepth(file.path);
      if (depth >= minDepth && (maxDepth === null || depth <= maxDepth)) {
        selected.add(file.path);
      }
    }
  }

  return selected;
}

/**
 * Select deepest files in each group (files with longest path depth)
 */
export function selectDeepestInGroup(groups: DuplicateGroup[]): Set<string> {
  const selected = new Set<string>();

  function getPathDepth(path: string): number {
    const separator = path.includes('/') ? '/' : '\\';
    return path.split(separator).filter(Boolean).length;
  }

  for (const group of groups) {
    // Find max depth in this group
    let maxDepth = 0;
    for (const file of group.files) {
      const depth = getPathDepth(file.path);
      if (depth > maxDepth) maxDepth = depth;
    }

    // Select all files at max depth (except keep at least one)
    const deepestFiles = group.files.filter(f => getPathDepth(f.path) === maxDepth);
    const otherFiles = group.files.filter(f => getPathDepth(f.path) < maxDepth);

    // If all files are at the same depth, keep the oldest
    if (otherFiles.length === 0) {
      const sorted = [...deepestFiles].sort((a, b) => a.created_at - b.created_at);
      for (let i = 1; i < sorted.length; i++) {
        selected.add(sorted[i].path);
      }
    } else {
      // Select all deepest files
      for (const file of deepestFiles) {
        selected.add(file.path);
      }
    }
  }

  return selected;
}

/**
 * Clear all selections
 */
export function clearSelection(): Set<string> {
  return new Set();
}
```

#### 6.6.2 Create Smart Selection Panel

**File**: `src/lib/components/SmartSelectionPanel.svelte`

```svelte
<script lang="ts">
  import type { DuplicateGroup } from '$lib/types';
  import {
    selectAllExceptOldest,
    selectByLocation,
    selectByPathDepth,
    selectDeepestInGroup,
    clearSelection,
  } from '$lib/utils/selection';

  interface Props {
    groups: DuplicateGroup[];
    selectedFiles: Set<string>;
    onSelectionChange: (selected: Set<string>) => void;
  }

  let { groups, selectedFiles, onSelectionChange }: Props = $props();

  let pathDepthMin = $state(1);
  let pathDepthMax = $state<number | null>(null);
  let folderPath = $state('');

  function handleSelectAllExceptOldest() {
    onSelectionChange(selectAllExceptOldest(groups));
  }

  function handleSelectByLocation() {
    if (folderPath.trim()) {
      onSelectionChange(selectByLocation(groups, folderPath.trim(), selectedFiles));
    }
  }

  function handleSelectByPathDepth() {
    onSelectionChange(selectByPathDepth(groups, pathDepthMin, pathDepthMax, selectedFiles));
  }

  function handleSelectDeepest() {
    onSelectionChange(selectDeepestInGroup(groups));
  }

  function handleClearSelection() {
    onSelectionChange(clearSelection());
  }
</script>

<div class="smart-selection">
  <h3>Smart Selection</h3>

  <div class="selection-option">
    <button onclick={handleSelectAllExceptOldest}>
      Select All Except Oldest
    </button>
    <p class="hint">Keep the original (oldest) file in each group</p>
  </div>

  <div class="selection-option">
    <button onclick={handleSelectDeepest}>
      Select Deepest Files
    </button>
    <p class="hint">Select files in the deepest directory levels</p>
  </div>

  <div class="selection-option">
    <div class="input-group">
      <label>
        Path depth range:
        <input type="number" bind:value={pathDepthMin} min="1" placeholder="Min" />
        to
        <input type="number" bind:value={pathDepthMax} min="1" placeholder="Max (optional)" />
      </label>
      <button onclick={handleSelectByPathDepth}>Select by Depth</button>
    </div>
    <p class="hint">Select files at specific directory depth levels</p>
  </div>

  <div class="selection-option">
    <div class="input-group">
      <input
        type="text"
        bind:value={folderPath}
        placeholder="Enter folder path..."
      />
      <button onclick={handleSelectByLocation} disabled={!folderPath.trim()}>
        Select by Location
      </button>
    </div>
    <p class="hint">Select all duplicates in a specific folder</p>
  </div>

  <div class="selection-option">
    <button class="clear-btn" onclick={handleClearSelection}>
      Clear Selection
    </button>
  </div>
</div>

<style>
  .smart-selection {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
  }

  h3 {
    margin: 0 0 1rem;
    font-size: 1rem;
  }

  .selection-option {
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }

  .selection-option:last-child {
    margin-bottom: 0;
    padding-bottom: 0;
    border-bottom: none;
  }

  button {
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .clear-btn {
    background: var(--error);
  }

  .hint {
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin: 0.25rem 0 0;
  }

  .input-group {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }

  input[type="text"],
  input[type="number"] {
    padding: 0.5rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--background);
    color: var(--text);
  }

  input[type="number"] {
    width: 80px;
  }

  input[type="text"] {
    flex: 1;
    min-width: 200px;
  }
</style>
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

#### Manual Verification
- [ ] "Select all except oldest" works correctly
- [ ] "Select by location" filters by folder path
- [ ] "Select by path depth" filters by directory depth
- [ ] "Select deepest files" selects files in deepest directories
- [ ] Clear selection removes all selections

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent on selection utilities and SmartSelectionPanel.

---

## Phase 6.7: Integrate Deletion in Results View

### Overview
Connect deletion dialogs to the results view.

### Changes Required

Update ResultsView.svelte to:
1. Show confirmation dialog before deletion
2. Call delete_files command
3. Show summary dialog after completion
4. Refresh results

### Success Criteria
- [ ] Complete deletion flow works

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 6.8: Create Deletion History Viewing UI

### Overview
Create a UI component for viewing deletion history. The backend `get_deletion_history` command already exists; this phase adds the frontend UI to display and interact with deletion history.

### Changes Required

#### 6.8.1 Create Deletion History Panel

**File**: `src/lib/components/DeletionHistoryPanel.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface DeletionRecord {
    id: number;
    path: string;
    size: number;
    hash: string;
    duplicate_group_id: number | null;
    session_id: number | null;
    deleted_at: number;
  }

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let history = $state<DeletionRecord[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let page = $state(0);
  let hasMore = $state(true);
  const pageSize = 50;

  onMount(() => {
    loadHistory();
  });

  async function loadHistory(reset: boolean = false) {
    if (reset) {
      page = 0;
      history = [];
      hasMore = true;
    }

    loading = true;
    error = null;

    try {
      const records = await invoke<DeletionRecord[]>('get_deletion_history', {
        limit: pageSize,
        offset: page * pageSize,
      });

      if (records.length < pageSize) {
        hasMore = false;
      }

      history = reset ? records : [...history, ...records];
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function loadMore() {
    page += 1;
    loadHistory();
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function getFileName(path: string): string {
    return path.split('/').pop() || path;
  }

  let totalFreed = $derived(history.reduce((sum, r) => sum + r.size, 0));
</script>

<div class="history-panel">
  <div class="header">
    <div class="header-info">
      <h2>Deletion History</h2>
      <span class="summary">{history.length} files • {formatBytes(totalFreed)} freed</span>
    </div>
    <button class="close-btn" onclick={onClose}>Close</button>
  </div>

  {#if error}
    <div class="error-message">{error}</div>
  {/if}

  {#if loading && history.length === 0}
    <div class="loading">Loading history...</div>
  {:else if history.length === 0}
    <div class="empty-state">
      <p>No deletion history yet</p>
      <p class="hint">Deleted files will appear here</p>
    </div>
  {:else}
    <div class="history-list">
      {#each history as record (record.id)}
        <div class="history-item">
          <div class="item-main">
            <span class="file-name">{getFileName(record.path)}</span>
            <span class="file-size">{formatBytes(record.size)}</span>
          </div>
          <div class="item-details">
            <span class="file-path" title={record.path}>{record.path}</span>
            <span class="delete-time">Deleted: {formatDate(record.deleted_at)}</span>
          </div>
        </div>
      {/each}

      {#if hasMore}
        <button class="load-more-btn" onclick={loadMore} disabled={loading}>
          {loading ? 'Loading...' : 'Load More'}
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .history-panel {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
    max-height: 500px;
    display: flex;
    flex-direction: column;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .header-info h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  .header-info .summary {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .close-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .error-message {
    background: var(--error-bg);
    color: var(--error);
    padding: 0.75rem;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .loading, .empty-state {
    text-align: center;
    padding: 2rem;
    color: var(--text-secondary);
  }

  .empty-state .hint {
    font-size: 0.85rem;
    margin-top: 0.5rem;
  }

  .history-list {
    flex: 1;
    overflow-y: auto;
  }

  .history-item {
    padding: 0.75rem;
    background: var(--background);
    border-radius: 6px;
    margin-bottom: 0.5rem;
  }

  .item-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .file-name {
    font-weight: 500;
    word-break: break-all;
  }

  .file-size {
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin-left: 1rem;
  }

  .item-details {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .file-path {
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 70%;
  }

  .load-more-btn {
    width: 100%;
    padding: 0.75rem;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    cursor: pointer;
    margin-top: 0.5rem;
  }

  .load-more-btn:hover:not(:disabled) {
    background: var(--background);
  }

  .load-more-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

#### Manual Verification
- [ ] Deletion history panel shows list of deleted files
- [ ] Each entry shows filename, size, full path, and deletion timestamp
- [ ] "Load More" pagination works correctly
- [ ] Total freed space is calculated and displayed
- [ ] Empty state is shown when no history exists

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent on DeletionHistoryPanel.svelte.

---

## Phase 6.9-6.10: Tests and Edge Cases

Add tests for:
- Deletion service
- Protected path blocking
- File verification
- Frontend dialogs
- Deletion history UI

---

## End of File 06

After completing all phases:
- Trash integration working
- Pre-deletion verification
- Confirmation dialog with **stronger delete-all-copies warning** (requires checkbox confirmation)
- Post-deletion summary dialog
- Smart selection options (including select by path depth)
- Protected path enforcement
- Deletion history recording
- **Deletion history viewing UI** with pagination

**Next**: Proceed to [07-scan-progress.md](./07-scan-progress.md)
