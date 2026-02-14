# DupliFind - Claude Code Project Instructions

## Automated Verification

When asked to run automated verification (or when plan steps call for it), run **all** of the following checks. All must pass before considering verification complete.

### Frontend

```bash
npm test              # Vitest - all frontend unit tests
npm run check         # svelte-check - TypeScript/Svelte type checking
npm run lint          # ESLint - linting (includes type-checked rules)
npm run build         # Vite production build
```

### Backend (from src-tauri/)

```bash
cargo test            # Rust unit tests (68+ tests)
cargo clippy          # Rust linter - must produce zero warnings
```

### Formatting

```bash
npx prettier --check .   # Prettier - verify formatting (do not auto-fix without asking)
```

If any check fails, fix the issues and re-run until all checks pass.

## Pre-Commit Formatting

Before committing **any** files (including markdown/plan files), run `npx prettier --write .` to auto-format. When making multiple commits in a row, it's fine to run it once before the first commit.

## Dead Code Annotations

The following items have `#[allow(dead_code)]` or `#[allow(unused_imports)]` because they are not yet used in production code but are needed in future plan phases. When a phase starts **using** one of these items, remove its suppression annotation:

- `scanner/mod.rs` — `#![allow(unused_imports)]` for the `HashError` re-export (needed once hash errors are surfaced to callers)
- `scanner/detector.rs` — `#[allow(dead_code)]` on `DuplicateGroup::original()` (needed for "Original" badge in Results UI)
- `scanner/hasher.rs` — `#[allow(dead_code)]` on `HashResult`, `full_hash_parallel()`, `compute_hashes()`, `hash_data()`
