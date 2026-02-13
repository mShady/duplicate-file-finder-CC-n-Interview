# Issue #5: Extract ScanService from commands/scan.rs

## Overview

The `start_scan` command in `commands/scan.rs` is a ~295-line monolith that mixes Tauri command concerns (state management, event emission) with scan orchestration business logic (file collection, duplicate detection, DB persistence). We'll extract the orchestration into a dedicated `ScanService` in `services/scan.rs`, following the existing `DeletionService` pattern.

## Current State Analysis

**`commands/scan.rs`** (502 lines total):

- `start_scan` (lines 62–355): validates input, creates DB session, builds config, spawns an async task that runs the full 3-phase pipeline (file collection, duplicate detection, DB persistence), manages state cleanup, and emits events.
- `cancel_scan` (lines 358–394): sets cancel flag, updates DB status.
- `get_scan_progress`, `is_scanning`, `get_scan_results`: thin read-only commands (already fine).

**The spawned task** (lines 141–349) contains ~200 lines of pure business logic that directly accesses `AppHandle` to:

1. Emit `scan-progress` events during file collection (every 100 files)
2. Emit `scan-phase` event when entering detection phase
3. Read `ScanState` from Tauri managed state to wire cancel flag to detector
4. Read `AppState` to get DB handle for persistence and error handling
5. Emit `scan-error`, `scan-complete`, `scan-results` events
6. Mutate `AppState.is_scanning` and `ScanState.cancel_flag` for cleanup

**Existing pattern to follow**: `DeletionService` (`services/deletion.rs`) — a struct with methods that perform business logic, called from the command via `tokio::task::spawn_blocking`. The command handles state access and response formatting.

### Key Discoveries:

- `DeletionService` is synchronous (blocking I/O), invoked via `spawn_blocking` — `commands/deletion.rs:71`
- `ScanService` needs to be async (DB writes, channel iteration) — will be invoked via `tauri::async_runtime::spawn`
- The scan task needs to emit events _during_ execution, requiring a callback mechanism
- `ScanState` (cancel flag holder) currently lives in `commands/scan.rs` — it should move to `services/scan.rs` since it's scan business state

## Desired End State

After this change:

- `services/scan.rs` contains a `ScanService` struct that orchestrates the 3-phase scan pipeline, accepts a callback for progress/event reporting, and writes results to the database
- `commands/scan.rs` becomes thin wrappers: validate input, lock state, create service, wire callback to `handle.emit()`, spawn task, return response
- `ScanState` moves to `services/scan.rs` (or `state.rs`) since it's not command-specific
- All existing behavior is preserved — no frontend changes needed
- Existing tests continue to pass; new unit tests cover `ScanService` methods

### How to Verify:

- `cargo build` succeeds with no new warnings
- `cargo test` passes (all existing + new tests)
- `cargo clippy` is clean
- Manual: app scans, shows progress, completes, cancels — all work identically

## What We're NOT Doing

- Changing any frontend code or Tauri event contracts
- Refactoring `get_scan_results` (it's a read-only query, not orchestration)
- Adding new features (incremental scanning, pause/resume, etc.)
- Changing the DB query layer
- Modifying `AppState` structure

## Implementation Approach

Use a **callback trait** to decouple the service from Tauri. The service defines a `ScanEventSink` trait with methods like `on_progress()`, `on_phase()`, `on_complete()`, `on_error()`. The command layer provides an implementation that calls `handle.emit()`. This keeps the service testable without Tauri dependencies.

## Phase 1: Create `ScanService` with Event Sink Trait

### Overview

Create `services/scan.rs` with the `ScanEventSink` trait and `ScanService` struct. Move `ScanState` there. Wire everything together in the command layer.

### Changes Required:

#### 1.1 Create `services/scan.rs`

**File**: `src-tauri/src/services/scan.rs` (new)

This file contains:

1. **`ScanEventSink` trait** — abstraction for progress reporting:

```rust
/// Abstraction for scan event reporting.
/// The command layer provides a Tauri-based implementation;
/// tests can use a no-op or collecting implementation.
pub trait ScanEventSink: Send + 'static {
    fn on_progress(&self, progress: &ScanProgress);
    fn on_phase(&self, phase: &str, message: &str);
    fn on_error(&self, session_id: i64, error: &str);
    fn on_complete(&self, completion: &ScanComplete);
    fn on_results(&self, results: &DetectionResult);
}
```

2. **`ScanState`** — moved from `commands/scan.rs`:

```rust
/// Scan cancellation state
pub struct ScanState {
    pub cancel_flag: Option<Arc<AtomicBool>>,
}
```

3. **`ScanComplete`** — moved from `commands/scan.rs`:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ScanComplete {
    pub session_id: i64,
    pub total_files: u64,
    pub total_bytes: u64,
    pub duplicate_groups: usize,
    pub duplicate_files: u64,
    pub wasted_space: u64,
    pub duration_ms: u64,
}
```

4. **`ScanService`** struct with one public method `run`:

```rust
pub struct ScanService;

impl ScanService {
    /// Execute the full scan pipeline: collect files, detect duplicates, persist results.
    ///
    /// This is the core orchestration extracted from the former `start_scan` spawned task.
    /// It is designed to run inside `tauri::async_runtime::spawn`.
    pub async fn run(
        config: ScanConfig,
        session_id: i64,
        cancel_flag: Arc<AtomicBool>,
        db: Arc<AsyncMutex<Database>>,
        sink: impl ScanEventSink,
    ) {
        // Phase 1: Collect files via walker.walk_channel()
        // Phase 2: Detect duplicates via DuplicateDetector
        // Phase 3: Persist results to DB
        // Cleanup and emit completion/error events via sink
    }
}
```

The body of `run` is the code currently at `commands/scan.rs:141–349`, with every `handle.emit(...)` replaced by a `sink.on_*()` call, and every `handle.state::<Mutex<...>>()` replaced by the parameters passed in (`cancel_flag`, `db`).

Key difference from the current code: state cleanup (`is_scanning = false`, `cancel_flag = None`) is **not** done inside the service — it's the command layer's responsibility (done after the spawned task completes, or via the sink callbacks).

Actually, looking at the current code more carefully: the spawned task _must_ clean up state because `start_scan` returns immediately after spawning. The cleanup happens inside the task. So we have two options:

- **Option A**: Pass state handles into the service → couples it to Tauri state
- **Option B**: Have the service return a result/status via a oneshot channel, and do cleanup in the spawn wrapper in the command

Option B is cleaner but adds complexity. Since state cleanup is 6 lines total and we want to keep this simple, we'll use a practical middle ground: the command layer wraps the service call in the spawned task and does cleanup after `ScanService::run` returns:

```rust
// In commands/scan.rs start_scan:
tauri::async_runtime::spawn(async move {
    ScanService::run(config, session_id, cancel_flag, db_arc.clone(), sink).await;

    // Cleanup state
    let app_state = handle.state::<Mutex<AppState>>();
    if let Ok(mut state) = app_state.lock() {
        state.is_scanning = false;
        state.current_scan_id = None;
    }
    let scan_state_ref = handle.state::<Mutex<ScanState>>();
    if let Ok(mut ss) = scan_state_ref.lock() {
        ss.cancel_flag = None;
    }
});
```

This keeps the service free of Tauri types while still doing cleanup correctly.

#### 1.2 Update `services/mod.rs`

**File**: `src-tauri/src/services/mod.rs`

```rust
pub mod deletion;
pub mod scan;
```

#### 1.3 Update `commands/scan.rs`

**File**: `src-tauri/src/commands/scan.rs`

Changes:

- Remove `ScanState`, `ScanComplete` (now in `services/scan.rs`)
- Import `ScanState`, `ScanComplete`, `ScanService`, `ScanEventSink` from `crate::services::scan`
- Create a `TauriEventSink` struct that implements `ScanEventSink` using `AppHandle`:

```rust
/// Bridges ScanService events to Tauri frontend events
struct TauriEventSink {
    handle: AppHandle,
}

impl ScanEventSink for TauriEventSink {
    fn on_progress(&self, progress: &ScanProgress) {
        let _ = self.handle.emit("scan-progress", progress);
    }
    fn on_phase(&self, phase: &str, message: &str) {
        let _ = self.handle.emit("scan-phase", serde_json::json!({
            "phase": phase,
            "message": message,
        }));
    }
    fn on_error(&self, session_id: i64, error: &str) {
        let _ = self.handle.emit("scan-error", serde_json::json!({
            "session_id": session_id,
            "error": error,
        }));
    }
    fn on_complete(&self, completion: &ScanComplete) {
        let _ = self.handle.emit("scan-complete", completion);
    }
    fn on_results(&self, results: &DetectionResult) {
        let _ = self.handle.emit("scan-results", results);
    }
}
```

- Simplify `start_scan` to: validate → lock state → create session → build config → store cancel handle → create `TauriEventSink` → spawn task calling `ScanService::run` + cleanup → return response

The `start_scan` function shrinks from ~295 lines to ~80 lines.

- `cancel_scan` stays mostly the same but imports `ScanState` from the new location
- `get_scan_progress`, `is_scanning`, `get_scan_results` — no changes (already thin)

#### 1.4 Update `lib.rs`

**File**: `src-tauri/src/lib.rs`

Change import path for `ScanState`:

```rust
// Before:
use commands::scan::ScanState;
// After:
use services::scan::ScanState;
```

### Success Criteria:

#### Automated Verification:

- [x] `cargo build` succeeds with no new warnings
- [x] `cargo test` passes (all existing tests + new `ScanService` tests)
- [x] `cargo clippy -- -D warnings` is clean (no new warnings from our files)

#### Manual Verification:

- [x] Start a scan — progress events appear in UI, results display correctly
- [x] Cancel a mid-scan — cancellation works, UI shows cancelled state
- [x] Start another scan after completion — no "already in progress" errors
- [x] Start another scan after cancellation — works correctly

---

## Testing Strategy

### Unit Tests (in `services/scan.rs`):

A `MockEventSink` that collects events into a `Vec` behind an `Arc<Mutex<...>>`:

```rust
#[cfg(test)]
struct MockEventSink {
    events: Arc<Mutex<Vec<String>>>,
}

impl ScanEventSink for MockEventSink {
    fn on_progress(&self, _progress: &ScanProgress) {
        self.events.lock().unwrap().push("progress".into());
    }
    // ... etc
}
```

Test cases:

- `test_scan_service_empty_directory` — run against empty temp dir, verify completion event fired with 0 duplicates
- `test_scan_service_with_duplicates` — create temp files with identical content, verify correct duplicate groups detected and completion stats
- `test_scan_service_cancellation` — set cancel flag before/during run, verify error event with appropriate message
- `test_scan_complete_serialization` — ensure `ScanComplete` serializes correctly for the frontend

### Existing Tests:

- `commands/scan.rs` tests for `ScanState` move to `services/scan.rs`
- All other existing tests remain unchanged

## Performance Considerations

None — this is a pure refactor. The same code runs in the same async context. No new allocations, no changed data flow, no additional indirection in the hot path (file iteration and hashing).

## References

- Existing service pattern: `src-tauri/src/services/deletion.rs`
- Current scan command: `src-tauri/src/commands/scan.rs:62-355`
- Issue tracker: `ISSUES.md` line 11
