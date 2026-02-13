# Issue #9: Consolidate Fragmented Documentation Under `docs/`

## Overview

The project's documentation is fragmented: `Specification.md` lives at the repo root and the `plans/` directory sits alongside source code directories. This plan moves both under a single `docs/` directory to keep the root clean and documentation organized.

## Current State Analysis

```
duplicate-file-finder-CC-n-Interview/
├── Specification.md          # Product specification (260 lines)
├── README.md                 # Project readme (stays at root)
├── CLAUDE.md                 # Claude Code instructions (stays at root)
├── ISSUES.md                 # Issue tracker, untracked (stays at root)
├── plans/                    # 16 plan files + issues/ subdirectory
│   ├── 00-index.md
│   ├── 01-project-foundation.md
│   ├── ...
│   ├── 15-system-tray.md
│   └── issues/
│       ├── 2026-02-12-issue-5-extract-scan-service.md
│       └── 2026-02-12-issue-6-12-consolidate-format-tests.md
├── src/                      # Frontend
└── src-tauri/                # Backend
```

### Key Discoveries

- `plans/00-index.md` uses relative links (e.g., `./01-project-foundation.md`) — these will survive the move intact since the entire directory moves together
- 4 plan files contain code-snippet comments referencing `plans/issues/...` paths — these are inside fenced code blocks and represent proposed future source code, so they need updating to `docs/plans/issues/...`
- `README.md:118` and `plans/01-project-foundation.md:1299` both have directory structure diagrams showing `plans/`
- `plans/issues/2026-02-12-issue-6-12-consolidate-format-tests.md` has internal cross-references using `plans/` prefix
- **No production code, build config, or imports depend on these paths** — zero risk of breaking the build

## Desired End State

```
duplicate-file-finder-CC-n-Interview/
├── README.md
├── CLAUDE.md
├── ISSUES.md
├── docs/
│   ├── Specification.md
│   └── plans/
│       ├── 00-index.md
│       ├── 01-project-foundation.md
│       ├── ...
│       ├── 15-system-tray.md
│       └── issues/
│           ├── 2026-02-12-issue-5-extract-scan-service.md
│           ├── 2026-02-12-issue-6-12-consolidate-format-tests.md
│           └── 2026-02-13-issue-9-consolidate-docs.md
├── src/
└── src-tauri/
```

All internal references updated. All automated verification passes.

## What We're NOT Doing

- Moving `README.md` — it belongs at the repo root per convention
- Moving `CLAUDE.md` — it must stay at the repo root for Claude Code to find it
- Moving `ISSUES.md` — it's a working tracker, not project documentation
- Changing any source code or build configuration
- Modifying plan content beyond updating path references

## Conflict Assessment with Existing Plans

| Plan                    | Conflict?      | Detail                                                                                           |
| ----------------------- | -------------- | ------------------------------------------------------------------------------------------------ |
| 01 (Foundation)         | Path reference | Directory structure diagram at line ~1299 shows `plans/` — needs update to `docs/plans/`         |
| 07 (Progress)           | Path reference | Code comment at line 644 references `plans/issues/...` — needs update to `docs/plans/issues/...` |
| 09 (File Ops)           | Path reference | Code comment at line 411 references `plans/issues/...` — needs update                            |
| 10 (Filtering)          | Path reference | Code comment at line 25 references `plans/issues/...` — needs update                             |
| 13 (Errors)             | Path reference | Code comment at line 587 references `plans/issues/...` — needs update                            |
| 02–06, 08, 11–12, 14–15 | None           | No references to `plans/` as a path                                                              |

**Verdict**: No blocking conflicts. All references are documentation/comments that can be updated in-place with a simple path prefix change.

## Implementation Approach

Single phase: move files via `git mv`, then update all references. This is a documentation-only change with no impact on the build or tests.

---

## Phase 1: Move Documentation and Update References

### Overview

Move `Specification.md` and `plans/` under a new `docs/` directory, then update all internal path references across the repository.

### Changes Required

#### 1.1 Create `docs/` Directory and Move Files

```bash
mkdir docs
git mv Specification.md docs/Specification.md
git mv plans/ docs/plans/
```

#### 1.2 Update `README.md` Directory Structure

**File**: `README.md`
**Changes**: Update the project structure diagram to show `docs/` instead of `plans/`

Replace:

```
├── scripts/               # Setup and build scripts
└── plans/                 # Implementation plans
```

With:

```
├── scripts/               # Setup and build scripts
└── docs/                  # Documentation
    ├── Specification.md   # Product specification
    └── plans/             # Implementation plans
```

#### 1.3 Update `plans/01-project-foundation.md` Directory Structure

**File**: `docs/plans/01-project-foundation.md` (after move)
**Changes**: Update the directory structure diagram in the README code block (around line 1299)

Replace:

```
├── scripts/               # Setup and build scripts
└── plans/                 # Implementation plans
```

With:

```
├── scripts/               # Setup and build scripts
└── docs/                  # Documentation
    ├── Specification.md   # Product specification
    └── plans/             # Implementation plans
```

#### 1.4 Update Code-Snippet Comments in 4 Plan Files

These are inside fenced code blocks — they represent proposed future source code comments.

**Files** (all paths are post-move):

- `docs/plans/07-scan-progress.md:644`
- `docs/plans/09-file-operations.md:411`
- `docs/plans/10-filtering-search.md:25`
- `docs/plans/13-error-handling.md:587`

**Change** (same in all 4 files):
Replace `plans/issues/2026-02-12-issue-6-12-consolidate-format-tests.md` with `docs/plans/issues/2026-02-12-issue-6-12-consolidate-format-tests.md`

#### 1.5 Update Cross-References in Issue Plan

**File**: `docs/plans/issues/2026-02-12-issue-6-12-consolidate-format-tests.md`
**Changes**: Update the 4 `**File**: \`plans/...\`` references in the "Phase 2: Update Future Plans" section (around lines 197–213)

Replace each occurrence of `plans/07-scan-progress.md`, `plans/09-file-operations.md`, `plans/10-filtering-search.md`, `plans/13-error-handling.md` with the `docs/plans/` prefix.

#### 1.6 Update ISSUES.md

**File**: `ISSUES.md`
**Changes**: Update Issue #9 description and status

Replace:

```
| 9   | Fragmented docs: `Specification.md` at root + `plans/` directory should be under `docs/`                                                   | Low    | Docs     | Open                                                  |
```

With:

```
| 9   | Fragmented docs: `Specification.md` at root + `plans/` directory should be under `docs/`                                                   | Low    | Docs     | Fixed                                                 |
```

### Success Criteria

#### Automated Verification

- [ ] `npm test` passes — frontend tests unaffected
- [ ] `npm run check` passes — svelte-check unaffected
- [ ] `npm run lint` passes — ESLint unaffected
- [ ] `npm run build` passes — Vite build unaffected
- [ ] `npx prettier --check .` passes — formatting intact
- [ ] `cargo test` passes (from `src-tauri/`) — backend unaffected
- [ ] `cargo clippy` passes (from `src-tauri/`) — backend unaffected

#### Manual Verification

- [ ] `docs/Specification.md` exists and matches original content
- [ ] `docs/plans/` contains all 16 plan files and `issues/` subdirectory
- [ ] `Specification.md` no longer exists at root
- [ ] `plans/` no longer exists at root
- [ ] `README.md` directory structure shows `docs/` instead of `plans/`
- [ ] All relative links in `docs/plans/00-index.md` still resolve correctly
- [ ] `git log --follow docs/Specification.md` shows full history

**Implementation Note**: This is a single-phase change. After completing all steps and automated verification passes, mark as done.

---

## Testing Strategy

### Unit Tests

No new tests needed — this is a documentation-only reorganization.

### Integration Tests

None needed.

### Manual Testing Steps

1. Verify `docs/Specification.md` content matches the original
2. Verify all 18 files in `docs/plans/` (16 plan files + 2 issue plans) are present
3. Open `docs/plans/00-index.md` and verify all relative links resolve
4. Run full automated verification suite from CLAUDE.md

## Performance Considerations

None — no runtime impact.

## References

- Issue #9 in ISSUES.md: "Fragmented docs: `Specification.md` at root + `plans/` directory should be under `docs/`"
- Files affected: README.md, 5 plan files, 1 issue plan, ISSUES.md
