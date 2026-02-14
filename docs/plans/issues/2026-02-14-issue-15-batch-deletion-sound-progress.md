# Issue #15: Batch Deletion Sound & Progress Fix

## Overview

Deleting N files causes the macOS "move to trash" sound to play N times, and there is no progress feedback during batch deletion. This plan restructures `DeletionService` to use `trash::delete_all()` (which batches all files into a single Finder/AppleScript operation, producing one sound), adds Tauri event-based progress reporting, and introduces a `DeletionProgressDialog` component in the frontend.

## Current State Analysis

### Backend

- `DeletionService::delete_batch()` (`src-tauri/src/services/deletion.rs:106-126`) iterates through files calling `delete_to_trash()` one at a time.
- Each `delete_to_trash()` call invokes `trash::delete(path)` (line 89). The `trash` crate v5.x defaults to `DeleteMethod::Finder` on macOS, which uses AppleScript (`tell application "Finder" to delete { POSIX file ... }`). Each individual call produces its own AppleScript invocation, triggering the system trash sound once per file.
- The command handler `delete_files()` (`src-tauri/src/commands/deletion.rs:30-111`) runs the batch on a blocking thread, awaits the result, records history, and returns.

### Frontend

- `App.svelte:135-165`: `handleConfirmDelete()` calls `deleteFiles()` API, immediately shows `DeleteSummaryDialog` on success. No progress feedback during the operation.
- Two existing dialog components: `DeleteConfirmDialog.svelte` and `DeleteSummaryDialog.svelte`, both following a consistent overlay + dialog pattern with focus trapping.

### Existing Pattern: Scan Progress Events

The scan feature provides a well-established event pattern:

- Rust: `ScanEventSink` trait (`src-tauri/src/services/scan.rs`) with `TauriEventSink` impl (`src-tauri/src/commands/scan.rs:31-67`) using `AppHandle::emit()`.
- Frontend: `scanStore.svelte.ts` uses `listen()` from `@tauri-apps/api/event` to subscribe to events and update Svelte 5 runes.

### Key Discoveries

- **`trash` crate defaults to `DeleteMethod::Finder`** on macOS — each `trash::delete(path)` call generates a separate AppleScript invocation, which is why the sound plays once per file.
- **`trash::delete_all()`** with the default Finder method batches all paths into a **single** AppleScript command (`tell application "Finder" to delete { POSIX file "path1", POSIX file "path2", ... }`). This produces **one sound** for the entire batch and preserves full **"Put Back"** functionality for all files.
- **`trash::delete_all()` is fail-fast**: returns `Result<(), Error>` — stops at first failure with no per-file breakdown. This means we must **verify hashes first** (individually), then batch-trash only verified files, then construct per-file results ourselves.
- **`NsFileManager` method was considered but rejected**: While it produces zero sounds, it breaks "Put Back" for all files except the first. Since the app's UI explicitly tells users _"You can restore them from Trash if needed"_ (`DeleteConfirmDialog.svelte:114`, `DeleteSummaryDialog.svelte:97`), breaking "Put Back" would be a user-facing regression.

### Approach Comparison

| Approach                              | Sound       | "Put Back"    | Complexity                       | Chosen?         |
| ------------------------------------- | ----------- | ------------- | -------------------------------- | --------------- |
| Current: `delete()` per file (Finder) | N sounds    | All files     | Baseline                         | No — bug        |
| `NsFileManager` + `delete_all()`      | 0 sounds    | Only 1st file | Platform-specific `#[cfg]` block | No — regression |
| **Finder (default) + `delete_all()`** | **1 sound** | **All files** | **Simple — no platform code**    | **Yes**         |

### Behavioral Change Summary

| Aspect               | Before                                        | After                                                                                   |
| -------------------- | --------------------------------------------- | --------------------------------------------------------------------------------------- |
| Trash sound          | Plays N times (once per file)                 | Plays once for the entire batch                                                         |
| "Put Back" in Finder | Works for all files                           | Works for all files (unchanged)                                                         |
| Progress feedback    | None — UI freezes between confirm and summary | Progress dialog with verification progress bar                                          |
| Deletion method      | Finder/AppleScript (default)                  | Finder/AppleScript (default, unchanged)                                                 |
| Error granularity    | Per-file success/failure                      | Per-file for verification; batch for trash (with post-hoc file-exists check on failure) |

### Conflict Assessment with Existing Plans and Specification

#### Plans

| Plan                                       | Conflict?                                                                                                                                                                                                                                                                                                                                                                                              | Notes |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----- |
| `06-selection-deletion.md`                 | **Already implemented** — plan is historical. Our changes to `services/deletion.rs` and `commands/deletion.rs` modify the completed code, not the plan. No conflict.                                                                                                                                                                                                                                   |
| `09-file-operations.md`                    | No conflict. File operations (open, reveal, context menu) are unrelated to deletion mechanics. The context menu's "Mark for Deletion" action (`09-file-operations.md:300-370`) feeds into the same selection flow — it does not call deletion service directly.                                                                                                                                        |
| `11-keyboard-nav.md`                       | No conflict. The Delete key shortcut (line 224) triggers `handleDeleteSelected()` which opens the confirmation dialog — same entry point, unaffected by backend changes.                                                                                                                                                                                                                               |
| `13-error-handling.md`                     | **Low risk**. Plan 13 defines `AppError` types and a `SkippedFilesManager` that don't overlap with deletion service. The `DeletionError` enum in `services/deletion.rs` is separate from the planned `AppError` in `error.rs`. **Action**: No changes needed now, but when Plan 13 is implemented, the deletion progress events could be extended to include richer error details from `AppError`.     |
| `14-platform-polish.md`                    | **Low risk**. Plan 14 adds platform-specific CSS and E2E tests. The new `DeletionProgressDialog` component follows existing dialog patterns so it will inherit platform styles. The E2E test for "selecting duplicates for deletion" (line 489) would need to account for the new progress dialog step, but Plan 14 hasn't been implemented yet. No conflict.                                          |
| `2026-02-14-issue-11-typed-api-layer.md`   | **Low risk**. Issue 11 wraps `invoke()` calls in typed API functions. The `deleteFiles()` wrapper in `src/lib/api/deletion.ts` is already implemented and its signature doesn't change — we're only changing backend internals and adding a new event listener in `App.svelte`. If Issue 11 adds a typed wrapper for `listen()` events later, the `deletion-progress` event would need to be included. |
| All other plans (01-05, 07-08, 10, 12, 15) | No conflict. These don't touch deletion code or the deletion UI flow.                                                                                                                                                                                                                                                                                                                                  |

#### Specification (`docs/Specification.md`)

| Spec Section                            | Alignment                                                                                                                                                                                                                                                                                             | Notes |
| --------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| **Batch Deletion** (line 28-33)         | **Fully aligned**. Spec says: "Deleted files go to System Trash/Recycle Bin", "Post-deletion summary displays what was deleted", "App highlights that users can undo by accessing system trash". Our fix preserves all three — same Finder-based trash, same summary dialog, full "Put Back" support. |
| **Pre-Delete Verification** (line 174)  | **Fully aligned**. Spec says: "Re-verify file hash before deletion, skip if file changed, notify user". Our two-phase approach does exactly this — verify all hashes first, only trash verified files, report failures.                                                                               |
| **Progress Display** (line 117-123)     | **Enhances alignment**. The spec defines progress display for scans but not for deletion. Adding deletion progress is an improvement that follows the same spirit.                                                                                                                                    |
| **Error Handling table** (line 207-215) | **Fully aligned**. "File changed since scan → Skip deletion, notify user, continue with others" and "File moved/deleted since scan → Skip deletion, notify user, continue with others" — both handled by the verification phase.                                                                      |

No specification conflicts. The fix improves spec alignment by adding progress feedback to deletion.

## What We're NOT Doing

- Not adding a cancellation mechanism for in-progress deletions (files are verified sequentially but trashed in one atomic batch — cancellation between verify and trash is the only window, and it's not worth the complexity).
- Not changing the `BatchDeletionResult` response type — the frontend contract stays the same.
- Not switching to `NsFileManager` — it would break "Put Back" functionality that users rely on.
- Not making the deletion method user-configurable (the Finder default is correct for all users).

## Implementation Approach

**Two-phase batch deletion**: Verify all files individually (with progress events), then call `trash::delete_all()` once for the verified batch. Files that fail verification are reported as failures.

**Simplified progress**: Emit events directly from the command handler using `AppHandle` — no trait abstraction needed since deletion is simpler than scanning and runs from a single command.

---

## Phase 1: Backend — Batch Deletion with Progress Events

### Overview

Restructure `DeletionService` to separate verification from trashing. Use `trash::delete_all()` (default Finder method) for a single batch trash operation that plays one sound and preserves "Put Back". Emit progress events from the command handler.

### Changes Required

#### 1.1 Update DeletionService

**File**: `src-tauri/src/services/deletion.rs`
**Changes**: Add a `verify_batch()` method that verifies all files and returns verified/failed splits. Add `trash_verified()` that calls `trash::delete_all()` once for the batch. Update `delete_batch()` to use the new two-phase approach.

```rust
use std::path::PathBuf;

/// Result of the verification phase
pub struct VerificationResult {
    /// Files that passed verification (exist + hash matches)
    pub verified: Vec<DeletionRequest>,
    /// Files that failed verification
    pub failed: Vec<DeletionResult>,
}

impl DeletionService {
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

    /// Trash all given files in a single OS operation via trash::delete_all().
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
                // All succeeded
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
                // delete_all is fail-fast — we don't know which file failed.
                // Check which files were actually removed and report accordingly.
                let mut successful = Vec::new();
                let mut failed = Vec::new();
                let mut total_freed = 0u64;

                for request in verified {
                    let path = Path::new(&request.path);
                    if !path.exists() {
                        // File was successfully trashed (no longer at original path)
                        total_freed += request.size;
                        successful.push(DeletionResult {
                            path: request.path.clone(),
                            success: true,
                            error: None,
                            size: request.size,
                        });
                    } else {
                        // File still exists — it failed or wasn't reached
                        failed.push(DeletionResult {
                            path: request.path.clone(),
                            success: false,
                            error: Some(e.to_string()),
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

    /// Delete multiple files to trash (updated to use two-phase approach)
    pub fn delete_batch(&mut self, requests: &[DeletionRequest]) -> BatchDeletionResult {
        let verification = self.verify_batch(requests, |_, _| {});
        let mut result = Self::trash_verified(&verification.verified);
        result.failed.extend(verification.failed);
        result
    }
}
```

The existing `delete_to_trash()` method is kept for backward compatibility with tests but the main batch path now uses `verify_batch()` + `trash_verified()`.

#### 1.2 Define Deletion Progress Event Type

**File**: `src-tauri/src/services/deletion.rs`
**Changes**: Add a serializable event struct.

```rust
#[derive(Debug, Clone, Serialize)]
pub struct DeletionProgressEvent {
    pub phase: String,          // "verifying" or "trashing"
    pub completed: usize,
    pub total: usize,
    pub current_path: Option<String>,
}
```

#### 1.3 Update Command Handler to Emit Progress

**File**: `src-tauri/src/commands/deletion.rs`
**Changes**: Accept `AppHandle`, emit `deletion-progress` events during verification, then emit a "trashing" event before the batch trash call. Import `Emitter` trait.

```rust
use tauri::{AppHandle, Emitter, State};
use crate::services::deletion::DeletionProgressEvent;

#[tauri::command]
pub async fn delete_files(
    request: DeleteFilesRequest,
    app_handle: AppHandle,
    state: State<'_, Mutex<AppState>>,
) -> Result<DeleteFilesResponse, String> {
    // ... existing validation and protected path checks (unchanged) ...

    let files = request.files;
    let handle = app_handle.clone();

    let result = tokio::task::spawn_blocking(move || {
        let mut service = DeletionService::new();

        // Phase 1: Verify all files (with progress)
        let verification = service.verify_batch(&files, |completed, total| {
            let _ = handle.emit(
                "deletion-progress",
                DeletionProgressEvent {
                    phase: "verifying".to_string(),
                    completed,
                    total,
                    current_path: files.get(completed.saturating_sub(1))
                        .map(|f| f.path.clone()),
                },
            );
        });

        // Phase 2: Emit trashing event and batch-trash verified files
        let _ = handle.emit(
            "deletion-progress",
            DeletionProgressEvent {
                phase: "trashing".to_string(),
                completed: 0,
                total: verification.verified.len(),
                current_path: None,
            },
        );

        let mut batch_result = DeletionService::trash_verified(&verification.verified);
        batch_result.failed.extend(verification.failed);
        batch_result
    })
    .await
    .map_err(|e| e.to_string())?;

    // ... existing history recording and response (unchanged) ...
}
```

### Success Criteria

#### Automated Verification

- [ ] `cargo test` — all existing tests pass (68+ tests)
- [ ] `cargo clippy` — zero warnings
- [ ] `npm run build` — frontend build still works (no breaking type changes)

#### Manual Verification

- [ ] Deleting multiple files produces exactly one macOS trash sound (not N sounds)
- [ ] Files are actually moved to system Trash (verifiable in Finder)
- [ ] "Put Back" works for trashed files (right-click in Trash → "Put Back")

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 2: Frontend — Deletion Progress Dialog

### Overview

Add a `DeletionProgressDialog` component that displays between the confirmation dialog closing and the summary dialog appearing. Listen for `deletion-progress` Tauri events.

### Changes Required

#### 2.1 Add Deletion Progress Types

**File**: `src/lib/types.ts`
**Changes**: Add the progress event type.

```typescript
export interface DeletionProgressEvent {
  phase: 'verifying' | 'trashing';
  completed: number;
  total: number;
  current_path: string | null;
}
```

#### 2.2 Create DeletionProgressDialog Component

**File**: `src/lib/components/DeletionProgressDialog.svelte`
**Changes**: New component following the existing dialog pattern from `DeleteConfirmDialog.svelte` and `DeleteSummaryDialog.svelte`.

```svelte
<script lang="ts">
  import type { DeletionProgressEvent } from '$lib/types';

  interface Props {
    progress: DeletionProgressEvent;
  }

  let { progress }: Props = $props();

  let percentage = $derived(
    progress.total > 0 ? Math.round((progress.completed / progress.total) * 100) : 0
  );

  let phaseLabel = $derived(
    progress.phase === 'verifying' ? 'Verifying files...' : 'Moving to Trash...'
  );
</script>

<div class="dialog-overlay">
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Deletion Progress">
    <h2>Deleting Files</h2>

    <p class="phase-label">{phaseLabel}</p>

    <div class="progress-bar-container">
      <div class="progress-bar" style="width: {percentage}%"></div>
    </div>

    <p class="progress-text">
      {#if progress.phase === 'verifying'}
        {progress.completed} of {progress.total} files verified
      {:else}
        Moving {progress.total} files to Trash...
      {/if}
    </p>

    {#if progress.current_path && progress.phase === 'verifying'}
      <p class="current-path" title={progress.current_path}>{progress.current_path}</p>
    {/if}
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
    max-width: 450px;
    width: 90%;
    text-align: center;
  }

  h2 {
    margin: 0 0 1rem;
  }

  .phase-label {
    color: var(--text-secondary);
    margin-bottom: 1rem;
    font-size: 0.95rem;
  }

  .progress-bar-container {
    height: 8px;
    background: var(--background);
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 0.75rem;
  }

  .progress-bar {
    height: 100%;
    background: var(--primary);
    border-radius: 4px;
    transition: width 0.2s ease;
  }

  .progress-text {
    font-size: 0.9rem;
    margin-bottom: 0.5rem;
  }

  .current-path {
    font-size: 0.8rem;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin: 0;
  }
</style>
```

#### 2.3 Update App.svelte to Show Progress

**File**: `src/App.svelte`
**Changes**: Add deletion progress state, listen for `deletion-progress` events, and show the progress dialog during deletion.

Add imports:

```typescript
import DeletionProgressDialog from './lib/components/DeletionProgressDialog.svelte';
import { listen } from '@tauri-apps/api/event';
import type { DeletionProgressEvent } from '$lib/types';
import type { UnlistenFn } from '@tauri-apps/api/event';
```

Add state:

```typescript
let isDeleting = $state(false);
let deletionProgress = $state<DeletionProgressEvent | null>(null);
```

Update `handleConfirmDelete()`:

```typescript
async function handleConfirmDelete() {
  if (!scanStore.detectionResult) return;
  showDeleteConfirm = false;
  scanStore.error = null;
  isDeleting = true;
  deletionProgress = null;

  const requests = buildDeletionRequests(scanStore.detectionResult, pendingDeletionFiles);
  const { keptPaths, groupIds } = buildKeptPathsAndGroupIds(
    scanStore.detectionResult,
    pendingDeletionFiles
  );

  // Listen for progress events during deletion
  let unlisten: UnlistenFn | null = null;

  try {
    unlisten = await listen<DeletionProgressEvent>('deletion-progress', (e) => {
      deletionProgress = e.payload;
    });

    const response = await deleteFiles({
      files: requests,
      kept_paths: keptPaths,
      group_ids: groupIds,
    });

    deletionResult = response.result;
    showDeleteSummary = true;

    if (response.result.successful.length > 0) {
      const deletedPaths = new Set(response.result.successful.map((r) => r.path));
      scanStore.detectionResult = computeUpdatedResults(scanStore.detectionResult, deletedPaths);
    }
  } catch (e) {
    scanStore.error = e instanceof Error ? e.message : String(e);
  } finally {
    unlisten?.();
    isDeleting = false;
    deletionProgress = null;
    pendingDeletionFiles = [];
  }
}
```

Add dialog to template (after the delete confirm dialog block):

```svelte
{#if isDeleting && deletionProgress}
  <DeletionProgressDialog progress={deletionProgress} />
{/if}
```

### Success Criteria

#### Automated Verification

- [ ] `npm test` — all frontend tests pass
- [ ] `npm run check` — svelte-check passes
- [ ] `npm run lint` — ESLint passes
- [ ] `npm run build` — production build succeeds

#### Manual Verification

- [ ] Progress dialog appears after confirming deletion
- [ ] Progress bar updates as files are verified
- [ ] Phase label changes from "Verifying files..." to "Moving to Trash..."
- [ ] Current file path is displayed during verification
- [ ] Summary dialog appears after deletion completes
- [ ] Exactly one macOS trash sound is heard (at the end, during "Moving to Trash" phase)

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the manual testing was successful before proceeding to the next phase.

---

## Phase 3: Verification

### Overview

Run all automated checks defined in CLAUDE.md.

### Success Criteria

#### Automated Verification

```bash
# Frontend
npm test              # Vitest
npm run check         # svelte-check
npm run lint          # ESLint
npm run build         # Vite production build

# Backend
cargo test            # Rust unit tests (from src-tauri/)
cargo clippy          # Zero warnings (from src-tauri/)

# Formatting
npx prettier --check .
```

All must pass.

---

## Testing Strategy

### Unit Tests (Rust)

Existing tests in `services/deletion.rs` cover:

- `test_verify_file_matching_hash`
- `test_verify_file_wrong_hash`
- `test_verify_file_not_found`
- `test_delete_to_trash_missing_file`
- `test_delete_to_trash_hash_mismatch`
- `test_delete_to_trash_success`
- `test_delete_batch_mixed_results`

The `delete_batch()` refactoring preserves the same public API and behavior, so existing tests should continue to pass. Add new tests:

- `test_verify_batch_all_valid` — all files pass verification
- `test_verify_batch_mixed` — some pass, some fail
- `test_verify_batch_progress_callback` — callback is called with correct values
- `test_trash_verified_empty` — empty input returns empty result

### Manual Testing Steps

1. Select 5+ files for deletion, confirm — verify exactly one trash sound plays
2. Select 20+ files for deletion — verify progress bar updates smoothly
3. Select files including one that was externally deleted — verify it appears in failed list while others succeed
4. Delete a single file — verify progress is brief but still shown
5. After deletion, open Trash in Finder — verify "Put Back" works on the deleted files

## Performance Considerations

- Hash verification is the bottleneck (reads full file content). Progress events during this phase give meaningful feedback.
- `trash::delete_all()` with Finder batches all paths into a single AppleScript command, which is faster than N individual AppleScript invocations.
- Tauri event emission is non-blocking and lightweight — emitting per-file during verification adds negligible overhead.

## References

- Issue: `ISSUES.md` — Issue #15
- Existing deletion plan: `docs/plans/06-selection-deletion.md` (completed/historical)
- Scan progress pattern: `src-tauri/src/commands/scan.rs:31-67` (TauriEventSink)
- `trash` crate docs: https://docs.rs/trash/latest/trash/
- `trash::delete_all`: https://docs.rs/trash/latest/trash/fn.delete_all.html
