# File 07: Scan Progress & Controls

## Overview

This file covers implementing the scan progress display with pause/resume functionality and scan state persistence that survives app restarts.

## Prerequisites

- Completed Files 01-06

---

## Phase 7.1: Create Progress State Model

### Overview
Create the data model for tracking scan progress with persistence.

### Changes Required

#### 7.1.1 Update Database Schema

Add scan progress persistence table (already in initial migration).

#### 7.1.2 Create Progress Queries

**File**: `src-tauri/src/db/queries.rs`

Add scan_progress module:

```rust
pub mod scan_progress {
    use super::*;

    pub async fn save_progress(
        pool: &SqlitePool,
        session_id: i64,
        current_path: Option<&str>,
        pending_paths: &[String],
        processed_count: i64,
        skipped_count: i64,
        error_count: i64,
        skipped_files: &[String],
    ) -> Result<(), sqlx::Error> {
        let pending_json = serde_json::to_string(pending_paths).unwrap_or_default();
        let skipped_json = serde_json::to_string(skipped_files).unwrap_or_default();

        sqlx::query(
            "INSERT INTO scan_progress (scan_session_id, current_path, pending_paths, processed_count, skipped_count, error_count, skipped_files)
             VALUES (?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(scan_session_id) DO UPDATE SET
                current_path = excluded.current_path,
                pending_paths = excluded.pending_paths,
                processed_count = excluded.processed_count,
                skipped_count = excluded.skipped_count,
                error_count = excluded.error_count,
                skipped_files = excluded.skipped_files,
                updated_at = strftime('%s', 'now')"
        )
        .bind(session_id)
        .bind(current_path)
        .bind(pending_json)
        .bind(processed_count)
        .bind(skipped_count)
        .bind(error_count)
        .bind(skipped_json)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn get_progress(
        pool: &SqlitePool,
        session_id: i64,
    ) -> Result<Option<ScanProgressData>, sqlx::Error> {
        // Implementation to retrieve saved progress
        todo!()
    }

    pub async fn clear_progress(pool: &SqlitePool, session_id: i64) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM scan_progress WHERE scan_session_id = ?")
            .bind(session_id)
            .execute(pool)
            .await?;
        Ok(())
    }
}
```

### Success Criteria
- [ ] `cargo check` passes

### Commit
Execute `/cl:commit`

---

## Phase 7.2: Update Scanner for Pause/Resume

### Overview
Add pause and resume capabilities to the directory walker.

### Changes Required

#### 7.2.1 Update Walker with Pause Support

**File**: `src-tauri/src/scanner/walker.rs`

Add pause functionality:

```rust
pub struct DirectoryWalker {
    config: ScanConfig,
    cancelled: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    progress: Arc<ScanProgressTracker>,
}

impl DirectoryWalker {
    pub fn pause_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.paused)
    }

    pub fn is_paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    // In walk methods, check for pause:
    // while self.paused.load(Ordering::Relaxed) {
    //     std::thread::sleep(std::time::Duration::from_millis(100));
    //     if self.cancelled.load(Ordering::Relaxed) {
    //         return Err(ScanError::Cancelled);
    //     }
    // }
}
```

### Success Criteria
- [ ] `cargo check` passes
- [ ] Pause/resume works in tests

### Commit
Execute `/cl:commit`

---

## Phase 7.3: Create Pause/Resume Commands

### Overview
Create Tauri commands for pausing and resuming scans.

### Changes Required

#### 7.3.1 Update Scan Commands

**File**: `src-tauri/src/commands/scan.rs`

Add:

```rust
#[tauri::command]
pub async fn pause_scan(
    state: State<'_, Mutex<AppState>>,
    scan_state: State<'_, Mutex<ScanState>>,
) -> Result<(), String> {
    let scan_state = scan_state.lock().map_err(|e| e.to_string())?;
    if let Some(pause_flag) = &scan_state.pause_flag {
        pause_flag.store(true, Ordering::Relaxed);
    }

    // Update session status
    let state = state.lock().map_err(|e| e.to_string())?;
    if let (Some(db), Some(session_id)) = (state.database(), state.current_scan_id) {
        let db = db.blocking_lock();
        tauri::async_runtime::block_on(async {
            queries::scan_sessions::update_status(db.pool(), session_id, ScanStatus::Paused).await
        })
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn resume_scan(
    state: State<'_, Mutex<AppState>>,
    scan_state: State<'_, Mutex<ScanState>>,
) -> Result<(), String> {
    let scan_state = scan_state.lock().map_err(|e| e.to_string())?;
    if let Some(pause_flag) = &scan_state.pause_flag {
        pause_flag.store(false, Ordering::Relaxed);
    }

    // Update session status
    let state = state.lock().map_err(|e| e.to_string())?;
    if let (Some(db), Some(session_id)) = (state.database(), state.current_scan_id) {
        let db = db.blocking_lock();
        tauri::async_runtime::block_on(async {
            queries::scan_sessions::update_status(db.pool(), session_id, ScanStatus::Running).await
        })
        .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_paused_scan(
    state: State<'_, Mutex<AppState>>,
) -> Result<Option<ScanSession>, String> {
    let state = state.lock().map_err(|e| e.to_string())?;
    let db = state.database().ok_or("Database not initialized")?;
    let db = tauri::async_runtime::block_on(db.lock());

    tauri::async_runtime::block_on(async { queries::scan_sessions::get_paused(db.pool()).await })
        .map_err(|e| e.to_string())
}
```

### Success Criteria
- [ ] `cargo check` passes

### Commit
Execute `/cl:commit`

---

## Phase 7.4: Create Progress Display Component

### Overview
Create a detailed progress display component.

### Changes Required

#### 7.4.1 Create Progress Component

**File**: `src/lib/components/ScanProgressDisplay.svelte`

```svelte
<script lang="ts">
  import type { ScanProgress } from '$lib/types';

  interface Props {
    progress: ScanProgress;
    isPaused: boolean;
    onPause: () => void;
    onResume: () => void;
    onCancel: () => void;
  }

  let { progress, isPaused, onPause, onResume, onCancel }: Props = $props();

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function formatNumber(n: number): string {
    return n.toLocaleString();
  }

  let progressPercent = $derived(
    progress.estimated_total
      ? Math.min(100, (progress.processed_files / progress.estimated_total) * 100)
      : 0
  );

  function truncatePath(path: string): string {
    if (!path) return '';
    if (path.length <= 60) return path;
    const parts = path.split('/');
    if (parts.length <= 3) return path;
    return `${parts[0]}/.../${parts.slice(-2).join('/')}`;
  }
</script>

<div class="progress-display">
  <div class="progress-header">
    <h2>{isPaused ? 'Scan Paused' : 'Scanning...'}</h2>
    <div class="controls">
      {#if isPaused}
        <button class="control-btn resume" onclick={onResume}>Resume</button>
      {:else}
        <button class="control-btn pause" onclick={onPause}>Pause</button>
      {/if}
      <button class="control-btn cancel" onclick={onCancel}>Cancel</button>
    </div>
  </div>

  {#if progress.estimated_total}
    <div class="progress-bar-container">
      <div class="progress-bar" style="width: {progressPercent}%"></div>
    </div>
    <div class="progress-percent">{progressPercent.toFixed(1)}%</div>
  {/if}

  <div class="stats-grid">
    <div class="stat">
      <span class="stat-value">{formatNumber(progress.total_files)}</span>
      <span class="stat-label">Files Found</span>
    </div>
    <div class="stat">
      <span class="stat-value">{formatNumber(progress.processed_files)}</span>
      <span class="stat-label">Processed</span>
    </div>
    <div class="stat">
      <span class="stat-value">{formatBytes(progress.total_bytes)}</span>
      <span class="stat-label">Total Size</span>
    </div>
    <div class="stat">
      <span class="stat-value">{formatNumber(progress.skipped_files)}</span>
      <span class="stat-label">Skipped</span>
    </div>
  </div>

  {#if progress.current_path}
    <div class="current-file">
      <span class="label">Current:</span>
      <span class="path" title={progress.current_path}>
        {truncatePath(progress.current_path)}
      </span>
    </div>
  {/if}
</div>

<style>
  .progress-display {
    background: var(--surface);
    border-radius: 12px;
    padding: 1.5rem;
    max-width: 600px;
    margin: 0 auto;
  }

  .progress-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1.5rem;
  }

  .progress-header h2 {
    margin: 0;
  }

  .controls {
    display: flex;
    gap: 0.5rem;
  }

  .control-btn {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-weight: 500;
  }

  .control-btn.pause {
    background: var(--warning);
    color: white;
  }

  .control-btn.resume {
    background: var(--success);
    color: white;
  }

  .control-btn.cancel {
    background: var(--error);
    color: white;
  }

  .progress-bar-container {
    height: 8px;
    background: var(--border);
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 0.5rem;
  }

  .progress-bar {
    height: 100%;
    background: var(--primary);
    transition: width 0.3s ease;
  }

  .progress-percent {
    text-align: center;
    font-size: 0.9rem;
    color: var(--text-secondary);
    margin-bottom: 1.5rem;
  }

  .stats-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 1rem;
    margin-bottom: 1.5rem;
  }

  .stat {
    text-align: center;
  }

  .stat-value {
    display: block;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .stat-label {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .current-file {
    background: var(--background);
    padding: 0.75rem;
    border-radius: 6px;
    font-size: 0.85rem;
  }

  .current-file .label {
    color: var(--text-secondary);
    margin-right: 0.5rem;
  }

  .current-file .path {
    font-family: var(--font-mono);
  }
</style>
```

### Success Criteria
- [ ] `npm run check` passes

### Commit
Execute `/cl:commit`

---

## Phase 7.5: Integrate Progress in App

### Overview
Update the main app to use the progress display with controls.

### Changes Required

Update App.svelte to use ScanProgressDisplay with pause/resume.

### Success Criteria
- [ ] Progress display works
- [ ] Pause/resume buttons work

### Commit
Execute `/cl:commit`

---

## Phase 7.6: Add Progress Persistence

### Overview
Save scan progress periodically and restore on app restart.

### Changes Required

1. Periodically save progress to database during scan
2. On app startup, check for paused scans
3. Offer to resume or discard

### Success Criteria
- [ ] Progress is saved periodically
- [ ] Paused scans can be resumed after restart

### Commit
Execute `/cl:commit`

---

## End of File 07

After completing all phases:
- Detailed progress display
- Pause/resume functionality
- Progress state persistence
- Resume after app restart

**Next**: Proceed to [08-settings-protected.md](./08-settings-protected.md)
