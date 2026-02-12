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

#### 7.1.2 Total File Count Estimation Strategy

The progress display shows "X of Y files" where Y (estimated total) is challenging to calculate efficiently. This section defines the strategy for providing accurate estimates without blocking the scan.

##### The Challenge

- **Pre-counting is slow**: Walking the entire directory tree just to count files before processing doubles the work
- **Unknown total during streaming**: Files are discovered incrementally; we don't know the total upfront
- **User expectation**: Users want to see "X of Y" progress, not just "X files processed"

##### Strategy: Rolling Estimate with Statistical Sampling

Use a **rolling estimate** that improves over time without requiring a pre-scan.

###### Phase 1: Initial Estimate (First 5 seconds)

During the first 5 seconds of scanning:

1. Track files discovered per second
2. Track directories discovered per second
3. Calculate average files per directory

```rust
struct EstimationState {
    start_time: Instant,
    files_discovered: u64,
    directories_discovered: u64,
    pending_directories: u64,  // Directories queued but not yet scanned
    estimation_complete: bool,
}

impl EstimationState {
    fn estimate_total(&self) -> Option<u64> {
        // Need at least 100 files and 10 directories for meaningful estimate
        if self.files_discovered < 100 || self.directories_discovered < 10 {
            return None;
        }

        // Average files per directory
        let avg_files_per_dir = self.files_discovered as f64 / self.directories_discovered as f64;

        // Estimate remaining files from pending directories
        let estimated_remaining = (self.pending_directories as f64 * avg_files_per_dir) as u64;

        Some(self.files_discovered + estimated_remaining)
    }
}
```

###### Phase 2: Continuous Refinement

As the scan progresses, refine the estimate using exponential moving average:

```rust
fn refine_estimate(
    current_estimate: u64,
    files_discovered: u64,
    directories_remaining: u64,
    alpha: f64,  // 0.3 = responsive refinement
) -> u64 {
    let files_per_remaining_dir = if directories_remaining > 0 {
        // Use recent discovery rate
        files_discovered as f64 / (directories_remaining as f64).max(1.0)
    } else {
        0.0
    };

    let new_estimate = files_discovered + (directories_remaining as f64 * files_per_remaining_dir) as u64;

    // EMA blending
    let blended = alpha * new_estimate as f64 + (1.0 - alpha) * current_estimate as f64;
    blended.round() as u64
}
```

###### Phase 3: Finalization

When all directories have been scanned, `estimated_total` equals `files_discovered`.

##### Implementation in Progress Tracker

Update `ScanProgressTracker` to include estimation:

```rust
pub struct ScanProgressTracker {
    // ... existing fields ...

    // Estimation fields
    directories_discovered: AtomicU64,
    pending_directories: AtomicU64,
    estimated_total: AtomicU64,
    estimation_start: Instant,
}

impl ScanProgressTracker {
    pub fn get_progress(&self) -> ScanProgress {
        let files = self.total_files.load(Ordering::Relaxed);
        let dirs = self.directories_discovered.load(Ordering::Relaxed);
        let pending = self.pending_directories.load(Ordering::Relaxed);

        // Calculate estimate
        let estimated_total = if pending == 0 {
            // Scan complete - exact count
            Some(files)
        } else if files >= 100 && dirs >= 10 {
            // Have enough data for estimate
            let avg_per_dir = files as f64 / dirs as f64;
            Some(files + (pending as f64 * avg_per_dir).round() as u64)
        } else {
            // Not enough data yet
            None
        };

        ScanProgress {
            total_files: files,
            processed_files: self.processed_files.load(Ordering::Relaxed),
            total_bytes: self.total_bytes.load(Ordering::Relaxed),
            current_path: None,
            skipped_files: self.skipped_files.load(Ordering::Relaxed),
            estimated_total,
        }
    }

    pub fn push_directory(&self) {
        self.pending_directories.fetch_add(1, Ordering::Relaxed);
    }

    pub fn pop_directory(&self) {
        self.pending_directories.fetch_sub(1, Ordering::Relaxed);
        self.directories_discovered.fetch_add(1, Ordering::Relaxed);
    }
}
```

##### Walker Integration

Update `DirectoryWalker` to track directory queue:

```rust
// In walk methods, track directory discovery
for entry_result in walker {
    match entry_result {
        Ok(entry) => {
            if entry.file_type().is_dir() {
                self.progress.push_directory();  // Directory discovered
                // ... later when processed ...
                self.progress.pop_directory();   // Directory completed
            }
            // ... rest of handling ...
        }
        // ...
    }
}
```

##### Frontend Display

The frontend uses `estimated_total` when available:

```typescript
// In progress display
let progressText = $derived(() => {
  if (progress?.estimated_total) {
    return `${progress.processed_files.toLocaleString()} of ~${progress.estimated_total.toLocaleString()} files`;
  }
  return `${progress?.processed_files.toLocaleString() ?? 0} files processed`;
});

let progressPercent = $derived(() => {
  if (!progress?.estimated_total || progress.estimated_total === 0) {
    return null; // Show indeterminate progress bar
  }
  return Math.min(99, (progress.processed_files / progress.estimated_total) * 100);
});
```

##### Edge Cases

| Scenario                 | Handling                                                                               |
| ------------------------ | -------------------------------------------------------------------------------------- |
| Empty directory tree     | Show "0 files" immediately, no estimate needed                                         |
| Single large directory   | Estimate may fluctuate; use high alpha (0.4) for quick stabilization                   |
| Many nested empty dirs   | `pending_directories` decreases faster than files increase; estimate converges quickly |
| Network drives (slow)    | Estimation still works; just takes longer to accumulate data                           |
| Cancel during estimation | Return last known estimate in progress state                                           |

##### Success Criteria

- [ ] Estimate appears within 5 seconds of scan start
- [ ] Estimate accuracy within 20% of final count after 10% of files scanned
- [ ] Progress bar never exceeds 99% until scan truly complete
- [ ] No blocking pre-scan required

#### 7.1.3 Create Progress Queries

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

### Code Review

Run code-review-fix-loop agent.

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

### Code Review

Run code-review-fix-loop agent.

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

### Code Review

Run code-review-fix-loop agent.

---

## Phase 7.4: Create Progress Display Component

### Overview

Create a detailed progress display component with estimated time remaining.

### ETA Calculation Algorithm

The ETA (Estimated Time of Arrival) calculation uses an **Exponential Moving Average (EMA)** approach to provide smooth, accurate time estimates that adapt to changing scan speeds.

#### Why EMA Over Simple Linear Calculation

A simple linear approach (`remaining_files / files_per_second`) produces jumpy estimates because:

- File sizes vary dramatically (1KB text file vs 1GB video)
- Disk I/O speed fluctuates based on file location and system load
- Hashing large files takes longer, causing sudden rate drops

EMA smooths these fluctuations while still adapting to sustained speed changes.

#### Algorithm Specification

```typescript
// ETA Calculator State (maintained across progress updates)
interface ETAState {
  emaRate: number; // Exponential moving average of processing rate (bytes/ms)
  lastUpdateTime: number; // Timestamp of last update
  lastProcessedBytes: number; // Bytes processed at last update
  alpha: number; // EMA smoothing factor (0.1 = smooth, 0.3 = responsive)
}

// Initialize when scan starts
function initETAState(): ETAState {
  return {
    emaRate: 0,
    lastUpdateTime: Date.now(),
    lastProcessedBytes: 0,
    alpha: 0.2, // Balance between smoothness and responsiveness
  };
}

// Calculate ETA on each progress update
function calculateETA(
  state: ETAState,
  currentProcessedBytes: number,
  totalBytes: number
): { etaMs: number; state: ETAState } {
  const now = Date.now();
  const timeDelta = now - state.lastUpdateTime;
  const bytesDelta = currentProcessedBytes - state.lastProcessedBytes;

  // Minimum time between updates to avoid division instability
  if (timeDelta < 100) {
    // Return previous estimate
    const remainingBytes = totalBytes - currentProcessedBytes;
    return {
      etaMs: state.emaRate > 0 ? remainingBytes / state.emaRate : null,
      state,
    };
  }

  // Calculate instantaneous rate (bytes per millisecond)
  const instantRate = bytesDelta / timeDelta;

  // Update EMA rate
  // First update: use instantaneous rate directly
  // Subsequent updates: blend with previous rate
  const newEmaRate =
    state.emaRate === 0
      ? instantRate
      : state.alpha * instantRate + (1 - state.alpha) * state.emaRate;

  // Calculate remaining time
  const remainingBytes = totalBytes - currentProcessedBytes;
  const etaMs = newEmaRate > 0 ? Math.round(remainingBytes / newEmaRate) : null;

  return {
    etaMs,
    state: {
      ...state,
      emaRate: newEmaRate,
      lastUpdateTime: now,
      lastProcessedBytes: currentProcessedBytes,
    },
  };
}
```

#### Backend vs Frontend Calculation

| Aspect               | Backend (Rust)                  | Frontend (Svelte)        |
| -------------------- | ------------------------------- | ------------------------ |
| **Responsibility**   | Primary ETA calculation         | Fallback/display         |
| **Data available**   | Precise byte counts, file queue | Progress events only     |
| **Update frequency** | Every 100 files or 1 second     | On event receipt         |
| **Sent to frontend** | `estimated_time_remaining_ms`   | Used directly if present |

**Backend implementation** (in `src-tauri/src/scanner/progress.rs`):

```rust
pub struct ProgressTracker {
    start_time: Instant,
    ema_rate: f64,           // bytes per millisecond
    last_update: Instant,
    last_bytes: u64,
    alpha: f64,              // EMA smoothing factor
}

impl ProgressTracker {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            ema_rate: 0.0,
            last_update: Instant::now(),
            last_bytes: 0,
            alpha: 0.2,
        }
    }

    pub fn update(&mut self, processed_bytes: u64, total_bytes: u64) -> Option<u64> {
        let now = Instant::now();
        let time_delta = now.duration_since(self.last_update).as_millis() as f64;

        // Skip if too soon (avoid instability)
        if time_delta < 100.0 {
            return self.calculate_eta(processed_bytes, total_bytes);
        }

        let bytes_delta = (processed_bytes - self.last_bytes) as f64;
        let instant_rate = bytes_delta / time_delta;

        // Update EMA
        self.ema_rate = if self.ema_rate == 0.0 {
            instant_rate
        } else {
            self.alpha * instant_rate + (1.0 - self.alpha) * self.ema_rate
        };

        self.last_update = now;
        self.last_bytes = processed_bytes;

        self.calculate_eta(processed_bytes, total_bytes)
    }

    fn calculate_eta(&self, processed_bytes: u64, total_bytes: u64) -> Option<u64> {
        if self.ema_rate <= 0.0 || processed_bytes >= total_bytes {
            return None;
        }

        let remaining = (total_bytes - processed_bytes) as f64;
        Some((remaining / self.ema_rate) as u64)
    }
}
```

#### Frontend Fallback

When `estimated_time_remaining_ms` is not provided by the backend (e.g., during the discovery phase before total size is known), the frontend uses a simpler file-count-based calculation:

```typescript
function fallbackETA(progress: ScanProgress): number | null {
  if (!progress.started_at_ms || !progress.estimated_total || progress.processed_files === 0) {
    return null;
  }

  const elapsedMs = Date.now() - progress.started_at_ms;
  const filesPerMs = progress.processed_files / elapsedMs;
  const remainingFiles = progress.estimated_total - progress.processed_files;

  if (filesPerMs <= 0) return null;
  return Math.round(remainingFiles / filesPerMs);
}
```

#### Edge Cases

| Scenario                          | Handling                                   |
| --------------------------------- | ------------------------------------------ |
| Scan just started (< 1 second)    | Show "Calculating..."                      |
| ETA > 24 hours                    | Show "More than a day"                     |
| Scan paused                       | Freeze ETA display, don't update state     |
| Scan resumed                      | Reset `lastUpdateTime`, preserve `emaRate` |
| Zero bytes processed              | Return null (show "Calculating...")        |
| Negative remaining (overestimate) | Show "Almost done..."                      |

#### Tuning the Alpha Parameter

- **α = 0.1**: Very smooth, slow to adapt. Good for consistent workloads.
- **α = 0.2**: Balanced (recommended default). Smooths spikes while adapting.
- **α = 0.3**: More responsive. Better for highly variable file sizes.

For duplicate file scanning where file sizes vary dramatically, **α = 0.2** provides the best balance.

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

  function formatTimeRemaining(ms: number | null | undefined): string {
    if (!ms || ms <= 0) return 'Calculating...';

    const seconds = Math.floor(ms / 1000);
    if (seconds < 60) return `${seconds}s remaining`;

    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    if (minutes < 60) return `${minutes}m ${remainingSeconds}s remaining`;

    const hours = Math.floor(minutes / 60);
    const remainingMinutes = minutes % 60;
    return `${hours}h ${remainingMinutes}m remaining`;
  }

  let progressPercent = $derived(
    progress.estimated_total
      ? Math.min(100, (progress.processed_files / progress.estimated_total) * 100)
      : 0
  );

  // Calculate estimated time remaining based on current progress rate
  let estimatedTimeRemaining = $derived(() => {
    // Use backend-provided estimate if available
    if (progress.estimated_time_remaining_ms) {
      return progress.estimated_time_remaining_ms;
    }

    // Otherwise calculate from progress
    if (!progress.started_at_ms || !progress.estimated_total || progress.processed_files === 0) {
      return null;
    }

    const elapsedMs = Date.now() - progress.started_at_ms;
    const filesPerMs = progress.processed_files / elapsedMs;
    const remainingFiles = progress.estimated_total - progress.processed_files;

    if (filesPerMs <= 0) return null;
    return Math.round(remainingFiles / filesPerMs);
  });

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

  <div class="time-remaining">
    <span class="time-icon">⏱️</span>
    <span class="time-value">{formatTimeRemaining(estimatedTimeRemaining())}</span>
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

  .time-remaining {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
    padding: 0.75rem;
    background: var(--background);
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .time-icon {
    font-size: 1rem;
  }

  .time-value {
    font-weight: 500;
    color: var(--text);
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

### Code Review

Run code-review-fix-loop agent.

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

### Code Review

Run code-review-fix-loop agent.

---

## End of File 07

After completing all phases:

- Detailed progress display with estimated time remaining
- Pause/resume functionality
- Progress state persistence
- Resume after app restart

**Next**: Proceed to [08-settings-protected.md](./08-settings-protected.md)
