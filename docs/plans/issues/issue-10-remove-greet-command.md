# Issue #10: Remove Leftover `greet` Command

## Overview

Remove the leftover `greet` command from production code. This is a test/demo command introduced during initial project scaffolding (Phase 1.4) that was never cleaned up. It serves no purpose now that the real command infrastructure is fully built out.

## Current State Analysis

The `greet` command exists in three locations:

| File                            | Lines | What                                                                                                                     |
| ------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------ |
| `src-tauri/src/commands/mod.rs` | 8–16  | Function definition                                                                                                      |
| `src-tauri/src/commands/mod.rs` | 24–51 | 4 unit tests (`test_greet`, `test_greet_empty`, `test_greet_whitespace_only`, `test_greet_with_leading_trailing_spaces`) |
| `src-tauri/src/lib.rs`          | 57    | Registration in `invoke_handler`                                                                                         |
| `tests/example.test.ts`         | 1–25  | Frontend test file using greet as a mock example (entire file is disposable)                                             |

### Key Discoveries:

- No production frontend code calls the `greet` command — only the example test file references it
- The `tests/example.test.ts` file contains only trivial tests (vitest mock verification + arithmetic) with no real value
- Plans 01–04 reference `greet` historically but no future plan phase depends on it

## Desired End State

- The `greet` function, its tests, and its command registration are completely removed
- The `tests/example.test.ts` file is deleted
- All automated verification passes (cargo test, cargo clippy, npm test, npm run check, npm run lint, npm run build, prettier)
- ISSUES.md updated to mark #10 as Fixed

## What We're NOT Doing

- Not modifying the plan docs (01–04) that reference `greet` historically — they document project evolution
- Not adding any replacement test infrastructure — real tests already exist elsewhere

## Conflict Assessment

**No conflicts with existing plans.** The `greet` command appears in plans 01-04 only as historical context. No future plan phase (05–15) references, depends on, or modifies the `greet` command or the files it lives in (beyond adding new commands to the same `invoke_handler`).

## Phase 1: Remove `greet` Command (Single Phase)

### Overview

Remove all traces of the greet command from backend and frontend.

### Changes Required:

#### 1.1 Remove function and tests from `commands/mod.rs`

**File**: `src-tauri/src/commands/mod.rs`
**Changes**: Delete the `greet` function (lines 8–16) and the entire `tests` module (lines 24–51), since all 4 tests only test `greet`.

**Before:**

```rust
//! Tauri command handlers

pub mod deletion;
pub mod protected;
pub mod scan;
pub mod settings;

/// Simple greet command for testing
#[tauri::command]
pub fn greet(name: &str) -> String {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return "Hello! Welcome to DupliFind.".to_string();
    }
    format!("Hello, {trimmed}! Welcome to DupliFind.")
}

// Re-export command functions for convenience
pub use deletion::*;
pub use protected::*;
pub use scan::*;
pub use settings::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() { ... }

    #[test]
    fn test_greet_empty() { ... }

    #[test]
    fn test_greet_whitespace_only() { ... }

    #[test]
    fn test_greet_with_leading_trailing_spaces() { ... }
}
```

**After:**

```rust
//! Tauri command handlers

pub mod deletion;
pub mod protected;
pub mod scan;
pub mod settings;

// Re-export command functions for convenience
pub use deletion::*;
pub use protected::*;
pub use scan::*;
pub use settings::*;
```

#### 1.2 Remove registration from `lib.rs`

**File**: `src-tauri/src/lib.rs`
**Changes**: Remove `commands::greet,` from the `invoke_handler` macro (line 57).

**Before:**

```rust
.invoke_handler(tauri::generate_handler![
    commands::greet,
    // Settings commands
    commands::get_setting,
```

**After:**

```rust
.invoke_handler(tauri::generate_handler![
    // Settings commands
    commands::get_setting,
```

#### 1.3 Delete `tests/example.test.ts`

**File**: `tests/example.test.ts`
**Changes**: Delete the entire file. It contains only:

- A mock test for the `greet` invoke call (no longer relevant)
- Trivial arithmetic/string tests that provide no value

#### 1.4 Update ISSUES.md

**File**: `ISSUES.md`
**Changes**: Change issue #10 status from `Open` to `Fixed`.

### Success Criteria:

#### Automated Verification:

- [x] Rust tests pass: `cd src-tauri && cargo test`
- [x] Rust linter clean: `cd src-tauri && cargo clippy`
- [x] Frontend tests pass: `npm test`
- [x] Svelte type checking: `npm run check`
- [x] ESLint passes: `npm run lint`
- [x] Production build succeeds: `npm run build`
- [x] Formatting check: `npx prettier --check .`

#### Manual Verification:

- [x] Confirm the app still starts and functions normally (`npm run tauri dev`)

## Testing Strategy

### Automated:

- Existing `cargo test` suite (68+ tests) should still pass — only the 4 `greet` tests are removed
- Existing frontend tests should still pass — only `example.test.ts` is removed
- Clippy should have no new warnings since `greet` was the only item in the `commands/mod.rs` tests module

### Manual:

- Quick smoke test: launch the app, start a scan, verify it works

## References

- Issue tracker: `ISSUES.md` line 14
- Original implementation: `docs/plans/01-project-foundation.md` Phase 1.4
