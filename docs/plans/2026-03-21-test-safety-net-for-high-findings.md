# Test Safety Net for High-Severity Code Review Findings

## Overview

Write tests BEFORE fixing the 8 high-severity findings from CODE-REVIEW.md. The tests serve as a safety net: they document the current (buggy) behavior, then validate the fixes without regressions.

## Current State

- **Rust tests**: 68 passing (scanner: 14 detector + 9 hasher + 11 walker, services: 5 scan + 4 deletion, db: 2, types: 3+)
- **Frontend tests**: 85 passing (4 test files)
- **Test infrastructure**: `MockEventSink` for scan events, `tempfile` for FS fixtures, real SQLite DBs in tests
- **Key gap**: No tests for `ScanService::run()` failure paths, no tests for DB edge cases, no hasher error-path tests

## What We're NOT Doing

- Performance benchmarks (findings 5-7 are performance; we test correctness only)
- npm audit fixes (finding 8; that's a dependency update, not a test)
- Frontend test changes (all 8 findings are in Rust backend)
- Changing any production code in this plan (tests only)

## Implementation Approach

Each phase adds tests for one finding. Tests should initially **pass** (documenting current behavior), then when the fix is applied they'll **continue to pass** (or be updated to assert the fixed behavior). Where the current behavior is clearly wrong, mark the test with a comment `// BUG: ...` and an assertion that captures the buggy output so the fix phase can flip it.

---

## Phase 1: Finding #2 — Unsafe i64→u64 DB Casts

### Overview

The simplest and most isolated finding. `get_scan_results()` uses `as u64` on DB `i64` values. We can test the cast behavior directly in the scan service tests.

### Changes Required

#### 1.1 New test: negative i64 cast behavior

**File**: `src-tauri/src/services/scan.rs` (test module)

Add tests that exercise the full `ScanService::run()` → DB persist → `get_scan_results()` round-trip. Since `get_scan_results` is a Tauri command (requires `State<>`), we can't call it directly. Instead, test the DB layer directly — insert a group with edge-case values and read it back.

**File**: `src-tauri/src/db/mod.rs` (test module, or new `src-tauri/src/db/tests.rs`)

```rust
#[tokio::test]
async fn test_db_roundtrip_large_file_size() {
    // Insert a duplicate group with file_size near i64::MAX
    // Read it back and verify the value is preserved correctly
    // This documents that `as u64` on the read side can produce wrong values
    // for values > i64::MAX that were truncated on write
}

#[tokio::test]
async fn test_db_roundtrip_zero_wasted_space() {
    // Insert a group with wasted_space = 0
    // Verify 0 survives the i64 → u64 round-trip
}

#[tokio::test]
async fn test_db_negative_value_cast_behavior() {
    // Directly insert a row with negative file_size via raw SQL
    // Call the query function and verify the current (buggy) as u64 behavior
    // Mark with // BUG: negative i64 wraps to large u64
}
```

### Success Criteria

#### Automated Verification:

- [x] `cargo test db::tests` passes — 5 tests (2 existing + 3 new)
- [x] `cargo clippy --all-targets -- -D warnings` clean

---

## Phase 2: Finding #3 — Silent Thread Panic in Walker Join

### Overview

`walker_handle.join().unwrap_or_default()` swallows panics. We need a test that demonstrates what happens when the walker thread panics.

### Changes Required

#### 2.1 Test walker panic propagation

**File**: `src-tauri/src/services/scan.rs` (test module)

We can't easily make `DirectoryWalker` panic in a controlled way without modifying production code. Instead, test the `unwrap_or_default()` behavior directly by verifying that a panicked walker produces default (zeroed) stats — documenting the silent failure.

```rust
#[tokio::test]
async fn test_scan_service_walker_default_stats() {
    // Scan a directory with a single file
    // Verify walker_stats.total_files > 0 (normal case)
    // This establishes the baseline that the fix can reference
}

#[test]
fn test_walk_result_default_is_zeroed() {
    // Verify WalkResult::default() produces all-zero fields
    // This documents what unwrap_or_default() returns on panic
    let default = WalkResult::default();
    assert_eq!(default.total_files, 0);
    assert_eq!(default.total_bytes, 0);
    // etc.
}
```

#### 2.2 Test error callback in walker

**File**: `src-tauri/src/scanner/tests.rs`

```rust
#[test]
fn test_walk_with_callback_error_handling() {
    // Create a walk_with_callback where the callback returns Err
    // Verify stats.errors is incremented and walk continues
}
```

### Success Criteria

#### Automated Verification:

- [x] `cargo test services::scan::tests` passes — 10 tests (8 existing + 2 new)
- [x] `cargo test scanner::tests` passes — 12 tests (11 existing + 1 new)

---

## Phase 3: Finding #4 — Silent DB Write Failures in Phase 3

### Overview

If all DB inserts fail during Phase 3 persistence, the scan still reports success. We need tests that verify the current behavior and establish a baseline for the fix.

### Changes Required

#### 3.1 Test successful persistence round-trip

**File**: `src-tauri/src/services/scan.rs` (test module)

```rust
#[tokio::test]
async fn test_scan_service_persists_groups_to_db() {
    // Run a scan with known duplicates
    // After ScanService::run(), query the DB directly
    // Verify groups and files were persisted
    // This is the "happy path" safety net
}

#[tokio::test]
async fn test_scan_service_session_status_completed() {
    // Run a scan
    // After completion, verify session status is "completed" in DB
}
```

#### 3.2 Test event sink receives correct events

**File**: `src-tauri/src/services/scan.rs` (test module)

Enhance `MockEventSink` to capture event payloads (not just names):

```rust
struct DetailedMockEventSink {
    events: Arc<Mutex<Vec<String>>>,
    completions: Arc<Mutex<Vec<ScanComplete>>>,
    errors: Arc<Mutex<Vec<(i64, String)>>>,
}
```

```rust
#[tokio::test]
async fn test_scan_complete_event_has_correct_counts() {
    // Run a scan with 3 files, 2 duplicates
    // Verify ScanComplete event has duplicate_groups=1, duplicate_files=1
}

#[tokio::test]
async fn test_scan_service_no_error_on_success() {
    // Run a normal scan
    // Verify no "error" events were emitted
    // This is the baseline — after fix, DB failures WILL emit errors
}
```

### Success Criteria

#### Automated Verification:

- [x] `cargo test services::scan::tests` passes — 14 tests (10 existing + 4 new)
- [x] All tests pass — 78 total (68 original + 10 new across phases 1-3)

---

## Phase 4: Finding #1 — Race Condition in cancel_scan DB Status

### Overview

Both `cancel_scan` and `ScanService::run()` update session status in the DB. This is the hardest to test because it involves concurrent async operations. We focus on testing the observable behavior.

### Changes Required

#### 4.1 Test cancellation sets correct DB status

**File**: `src-tauri/src/services/scan.rs` (test module)

```rust
#[tokio::test]
async fn test_scan_service_cancelled_status() {
    // Create files, start scan with cancel_flag = true (pre-cancelled)
    // After ScanService::run(), check DB session status
    // Current behavior: ScanService sees cancellation in detector,
    // detector returns Err(Cancelled), service sets status to Failed
    // Meanwhile cancel_scan also sets Cancelled
    // Document whichever status wins
}

#[tokio::test]
async fn test_scan_cancellation_mid_detection() {
    // Start scan, set cancel_flag after a delay
    // Verify the final DB status and that no panic occurs
}

#[tokio::test]
async fn test_scan_service_cancelled_emits_error_event() {
    // Pre-cancel, run scan
    // Verify either error or complete event is emitted (not both, not neither)
}
```

### Success Criteria

#### Automated Verification:

- [x] `cargo test services::scan::tests` passes — 17 tests (14 existing + 3 new)
- [x] No test hangs or deadlocks — all completed in <1s

---

## Phase 5: Findings #5-7 — Correctness-Preserving Tests for Performance Fixes

### Overview

These are performance issues. We don't benchmark — we write correctness tests that ensure the optimization doesn't change hashing results or detection outcomes.

### Changes Required

#### 5.1 Finding #5: Parallel hashing produces same results

**File**: `src-tauri/src/scanner/hasher.rs` (test module)

```rust
#[test]
fn test_full_hash_parallel_matches_sequential() {
    // Create a large file (>1MB)
    // Hash with full_hash() and full_hash_parallel()
    // Assert identical results
}
```

#### 5.2 Finding #6: Buffer reuse doesn't change hash output

**File**: `src-tauri/src/scanner/hasher.rs` (test module)

```rust
#[test]
fn test_partial_hash_deterministic_across_calls() {
    // Call partial_hash() on the same file multiple times
    // Assert all results are identical
    // This proves buffer reuse (when implemented) doesn't corrupt state
}

#[test]
fn test_partial_hash_different_files_interleaved() {
    // Hash file_a, then file_b, then file_a again
    // Assert file_a's hash is the same both times
    // This catches buffer contamination bugs
}
```

#### 5.3 Finding #7: DB persistence correctness (already covered by Phase 3 tests)

The Phase 3 tests (persists_groups_to_db, session_status_completed) already serve as the safety net for batched inserts. If batching changes break persistence, those tests will catch it.

### Success Criteria

#### Automated Verification:

- [ ] `cargo test scanner::hasher::tests` passes
- [ ] All 68+ existing tests still pass

---

## Phase 6: Finding #8 — npm Dependency Update

### Overview

No tests needed. Just update dependencies.

### Changes Required

Run `npm audit fix` or manually update `vitest` and `vite` in `package.json`.

### Success Criteria

#### Automated Verification:

- [ ] `npm audit` reports no high-severity vulnerabilities
- [ ] `npm test -- --run` still passes (85 tests)
- [ ] `npm run build` succeeds

---

## Testing Strategy Summary

| Finding               | Test Type          | New Tests | Files Modified                         |
| --------------------- | ------------------ | --------- | -------------------------------------- |
| #1 Race condition     | Integration        | 3         | `services/scan.rs`                     |
| #2 Unsafe casts       | Unit               | 3         | `db/mod.rs` or `db/tests.rs`           |
| #3 Silent panic       | Unit + Integration | 3         | `services/scan.rs`, `scanner/tests.rs` |
| #4 Silent DB failures | Integration        | 4         | `services/scan.rs`                     |
| #5 Parallel hashing   | Unit               | 1         | `scanner/hasher.rs`                    |
| #6 Buffer reuse       | Unit               | 2         | `scanner/hasher.rs`                    |
| #7 Batch inserts      | (covered by #4)    | 0         | —                                      |
| #8 npm audit          | —                  | 0         | `package.json`                         |
| **Total**             |                    | **~16**   | **4 files**                            |

## Verification

After all phases complete:

```bash
cargo test            # All Rust tests pass (68 existing + ~16 new = ~84)
cargo clippy --all-targets -- -D warnings   # Zero warnings
npm test -- --run     # 85 frontend tests pass
npm run check         # svelte-check clean
npm run lint          # ESLint clean
npm run build         # Vite build succeeds
npx prettier --check .  # Formatting clean
```
