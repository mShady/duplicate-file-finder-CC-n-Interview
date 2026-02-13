# DupliFind Implementation Plan - Index

## Overview

This document serves as the master index for the DupliFind implementation plan. The plan is divided into multiple files, each focusing on a specific feature group. Phases are numbered based on their file (e.g., Phase 2.3 = File 02, Phase 3).

## Project Summary

**DupliFind** is a cross-platform desktop application for finding and removing duplicate files on Mac and Windows systems using:

- **Framework**: Tauri 2.x (Rust backend + web frontend)
- **Frontend**: Svelte 5 + TypeScript
- **Database**: SQLite (via sqlx)
- **Hash Algorithm**: BLAKE3

## Implementation Strategy

We follow a **vertical slice** approach - delivering complete, testable features end-to-end before moving to the next. Each feature includes:

1. Backend (Rust) implementation
2. Frontend (Svelte) implementation
3. Unit tests
4. E2E tests (where applicable)
5. Git commit via `/cl:commit`
6. Code review (via code-review-fix-loop agent)

## Plan Files

| File                                                     | Feature Group        | Phases      | Description                                                                                                                                                                                                    |
| -------------------------------------------------------- | -------------------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [01-project-foundation.md](./01-project-foundation.md)   | Project Foundation   | 1.1 - 1.8   | Tauri scaffolding, project structure, developer tooling                                                                                                                                                        |
| [02-database-foundation.md](./02-database-foundation.md) | Database             | 2.1 - 2.6   | SQLite setup, schema, migrations                                                                                                                                                                               |
| [03-file-scanning.md](./03-file-scanning.md)             | File Scanning        | 3.1 - 3.10  | Directory traversal, file metadata collection, folder picker UI                                                                                                                                                |
| [04-duplicate-detection.md](./04-duplicate-detection.md) | Detection            | 4.1 - 4.10  | Size grouping, partial/full hashing, BLAKE3, empty file handling (4.2.3), live streaming events (4.3.3)                                                                                                        |
| [05-results-ui.md](./05-results-ui.md)                   | Results UI           | 5.1 - 5.8   | Master-detail layout, duplicate groups display, live streaming subscription (5.1.5), **path truncation with middle ellipsis + hover tooltip**, **creation AND modified dates**                                 |
| [06-selection-deletion.md](./06-selection-deletion.md)   | Selection & Deletion | 6.1 - 6.10  | Selection logic (incl. path depth), batch deletion, trash integration, **stronger delete-all-copies warning with checkbox confirmation**, **deletion history viewing UI**                                      |
| [07-scan-progress.md](./07-scan-progress.md)             | Progress & Controls  | 7.1 - 7.6   | Progress display with ETA (7.1.2 total count estimation), pause/resume, persistence                                                                                                                            |
| [08-settings-protected.md](./08-settings-protected.md)   | Settings             | 8.1 - 8.6   | Theme, parallelism, protected folders, **last scan settings restoration on app launch**                                                                                                                        |
| [09-file-operations.md](./09-file-operations.md)         | File Operations      | 9.1 - 9.7   | Open, reveal, copy path, view file info, context menu                                                                                                                                                          |
| [10-filtering-search.md](./10-filtering-search.md)       | Filtering & Search   | 10.1 - 10.8 | File type filters, search, thumbnails                                                                                                                                                                          |
| [11-keyboard-nav.md](./11-keyboard-nav.md)               | Keyboard & A11y      | 11.1 - 11.6 | Keyboard shortcuts, focus management                                                                                                                                                                           |
| [12-permissions.md](./12-permissions.md)                 | Permissions          | 12.1 - 12.6 | Permission wizard, Full Disk Access guide, **Windows permissions wizard with Controlled Folder Access detection**                                                                                              |
| [13-error-handling.md](./13-error-handling.md)           | Error Handling       | 13.1 - 13.7 | Skip/retry, disk full handling, error display, recovery, **clearer file moved/deleted error messaging with user-friendly explanations**, **detailed disk I/O throttling with adaptive queue depth monitoring** |
| [14-platform-polish.md](./14-platform-polish.md)         | Polish & Final       | 14.1 - 14.8 | Native styling, platform tweaks, final E2E, **comprehensive E2E tests covering actual duplicate detection logic**                                                                                              |
| [15-system-tray.md](./15-system-tray.md)                 | System Tray          | 15.1 - 15.8 | Minimize to tray feature, tray icon and menu, **tray tooltip with scan progress % and ETA**                                                                                                                    |

## Phase Structure

Each phase follows this structure:

```markdown
### Phase X.Y: [Name]

#### Overview

[What this phase accomplishes]

#### Changes Required

[Detailed file-by-file changes with code snippets]

#### Success Criteria

##### Automated Verification

- [ ] Command 1
- [ ] Command 2

##### Manual Verification

- [ ] Step 1
- [ ] Step 2

#### Commit

Execute `/cl:commit` to commit changes with meaningful message.

#### Code Review

Run code-review-fix-loop agent.
```

## Key Technical Decisions

| Decision         | Choice                     | Rationale                              |
| ---------------- | -------------------------- | -------------------------------------- |
| Tauri Version    | 2.9.x                      | Latest stable with mobile support      |
| Svelte Version   | 5.x                        | Runes for reactivity, latest features  |
| SQLite Library   | sqlx 0.8.x                 | Compile-time checked queries, async    |
| Hash Algorithm   | BLAKE3 1.8.x               | Fast, secure, parallelizable           |
| Component Tests  | vitest-browser-svelte      | Real browser testing, Svelte 5 support |
| E2E Tests        | WebdriverIO + tauri-driver | Official Tauri recommendation          |
| State Management | Tauri managed state        | Built-in, thread-safe                  |

## Development Commands

After setup, these commands will be available:

```bash
# Development
npm run dev          # Start dev server with hot reload
npm run tauri dev    # Start Tauri dev mode

# Testing
npm run test         # Run Svelte component tests
cargo test           # Run Rust unit tests
npm run test:e2e     # Run E2E tests

# Building
npm run build        # Build frontend
npm run tauri build  # Build complete app

# Linting
npm run lint         # Lint frontend
cargo clippy         # Lint Rust code
```

## Directory Structure (Target)

```
duplicate-file-finder-CC-n-Interview/
├── package.json
├── vite.config.ts
├── svelte.config.js
├── tsconfig.json
├── src/                          # Frontend (Svelte)
│   ├── app.html
│   ├── app.css
│   ├── main.ts
│   ├── App.svelte
│   ├── lib/
│   │   ├── components/           # Svelte components
│   │   │   ├── DuplicateList.svelte
│   │   │   ├── FileDetails.svelte
│   │   │   ├── ProgressBar.svelte
│   │   │   └── ...
│   │   ├── stores/               # Svelte stores
│   │   └── utils/                # Frontend utilities
│   └── routes/                   # SvelteKit routes (if using)
├── src-tauri/                    # Backend (Rust)
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── capabilities/
│   ├── build.rs
│   ├── migrations/               # SQLite migrations
│   └── src/
│       ├── lib.rs                # Main entry point
│       ├── main.rs               # Desktop entry (don't modify)
│       ├── commands/             # Tauri commands
│       │   ├── mod.rs
│       │   ├── scan.rs
│       │   ├── files.rs
│       │   └── settings.rs
│       ├── db/                   # Database layer
│       │   ├── mod.rs
│       │   ├── models.rs
│       │   ├── schema.rs
│       │   └── queries.rs
│       ├── scanner/              # File scanning logic
│       │   ├── mod.rs
│       │   ├── walker.rs
│       │   ├── hasher.rs
│       │   └── detector.rs
│       ├── services/             # Business logic
│       └── state.rs              # App state
├── tests/                        # Frontend tests
│   └── components/
├── e2e/                          # E2E tests
│   ├── package.json
│   ├── wdio.conf.js
│   └── specs/
└── scripts/                      # Setup/build scripts
    ├── setup-mac.sh
    └── setup-windows.ps1
```

## Getting Started

1. Start with [01-project-foundation.md](./01-project-foundation.md)
2. Complete each phase in order within each file
3. After completing all phases in a file, proceed to the next file
4. Each phase must be committed and pass code review before proceeding

## Notes

- **Commit Size**: Each phase targets ~5 files or fewer per commit
- **Test Separation**: When tests would make a commit too large, they're in separate phases
- **Vertical Slices**: Backend + Frontend + Tests together for each feature
- **Commits**: Every phase includes `/cl:commit` to commit implementation work
- **Code Review**: Every phase ends with code-review-fix-loop agent review (after commit)
- **Sub-phases**: Some phases have detailed sub-phases (e.g., 4.2.3, 5.1.5, 7.1.2) for complex specifications

## Recent Additions

The following sub-phases were added to address specification gaps:

| Sub-Phase | File                      | Description                                                                        |
| --------- | ------------------------- | ---------------------------------------------------------------------------------- |
| 4.2.3     | 04-duplicate-detection.md | Empty file handling specification - zero-byte files grouped as duplicates          |
| 4.3.3     | 04-duplicate-detection.md | Live streaming event emission - `duplicate-found` event and throttling             |
| 5.1.5     | 05-results-ui.md          | Frontend live streaming subscription pattern - `scanStore.ts` with event listeners |
| 7.1.2     | 07-scan-progress.md       | Total file count estimation strategy - rolling estimate without pre-scan           |

## January 2026 Updates

The following updates address 10 specification gaps identified in the implementation plans:

| #   | Issue                                                     | Plan Location                                     | Resolution                                                                                                                                |
| --- | --------------------------------------------------------- | ------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| 1   | Path truncation (middle ellipsis + hover) not implemented | `05-results-ui.md`                                | Added `getDirectory()` function with middle-ellipsis truncation + CSS hover tooltip showing full path                                     |
| 2   | Both creation AND modified dates not displayed            | `05-results-ui.md`                                | Updated file item template to display both dates with clear labels                                                                        |
| 3   | Last scan settings not restored on app launch             | `03-file-scanning.md`, `08-settings-protected.md` | Added explicit app-launch restoration documentation and success criteria                                                                  |
| 4   | Delete-all-copies warning could be stronger               | `06-selection-deletion.md`                        | Added danger banner with red styling, mandatory checkbox confirmation, disabled confirm button until checkbox is checked                  |
| 5   | File moved/deleted error messaging unclear                | `13-error-handling.md`                            | Added `FileMoved` and `FileDeleted` error variants with `user_message()` and `suggested_action()` methods                                 |
| 6   | Windows permissions wizard missing                        | `12-permissions.md`                               | Added Phase 12.4 with Windows-specific Controlled Folder Access detection and step-by-step wizard                                         |
| 7   | Tray tooltip doesn't show scan progress %                 | `15-system-tray.md`                               | Added `update_tray_progress()` function with percentage, phase, ETA, and file count in tooltip                                            |
| 8   | Disk I/O throttling implementation details vague          | `13-error-handling.md`                            | Completely rewrote Phase 13.5 with `ThrottleProfile`, queue depth monitoring, adaptive throttling based on latency                        |
| 9   | Deletion history viewing UI not planned                   | `06-selection-deletion.md`                        | Added Phase 6.8 with `DeletionHistoryPanel.svelte` component with pagination                                                              |
| 10  | E2E tests don't cover actual duplicate detection logic    | `14-platform-polish.md`                           | Added `duplicate-detection.spec.js` with tests for detection accuracy, edge cases (empty files, same-size different content, large files) |
