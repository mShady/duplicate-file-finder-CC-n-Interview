# DupliFind - Product Specification

## Overview

DupliFind is a cross-platform desktop application for finding and removing duplicate files on Mac and Windows systems. The app scans storage devices to identify files with identical content, groups them together, and presents them in an intuitive interface where users can review each group, decide which copy to keep, and safely remove the rest to reclaim disk space.

---

## Product Requirements

### Core Functionality

1. **Duplicate Detection**
   - "Similar" is defined by identical file content, regardless of filename
   - Uses multi-stage detection approach:
     1. Filter by file size (files must have identical sizes to be duplicates)
     2. Partial hash comparison (first/last 4KB) for quick elimination
     3. Full content hash only when partial hashes match
   - Hash algorithm: BLAKE3 (fast, cryptographically secure, modern)
   - Empty (zero-byte) files are treated as duplicates of each other

2. **Scan Scope**
   - Scans all mounted volumes: internal drives, external USB drives, network shares, cloud-synced folders
   - No default exclusions - scans everything user has permission to access
   - Symbolic links are skipped entirely (never followed)
   - No file size limits - all file sizes are scanned

3. **Batch Deletion**
   - Users can mark multiple files for deletion across different duplicate groups
   - Single "commit" action deletes all marked files
   - Deleted files go to System Trash/Recycle Bin (not permanent deletion)
   - Post-deletion summary displays what was deleted
   - App highlights that users can undo by accessing system trash

4. **Protected Folders**
   - Users can designate folders as "protected"
   - Files in protected folders cannot be selected for deletion
   - User-defined only - no default protected folders

---

## Technical Requirements

### Technology Stack

- **Framework**: Tauri (Rust backend + web frontend)
- **Frontend**: Svelte + TypeScript
- **Database**: SQLite for all local storage (scan results, file indexes, settings, deletion history)
- **Target Platforms**: macOS, Windows (desktop only for v1)

### Dependencies & Versions

- Use latest stable versions of all dependencies (released at least 1 month prior)
- Prefer modern design patterns and architectures when multiple options exist
- All data stored locally - no external backend services

### Performance

- **CPU Utilization**: User-controlled parallelism with three modes:
  - Light (1-2 threads)
  - Normal/Adaptive (default - ~75% of available cores)
  - Aggressive (all cores)
- **Disk I/O**: Smart throttling that monitors disk queue depth and backs off when system is busy
- **Incremental Scanning**: Optional per-scan choice:
  - "Quick scan" - uses cached file hashes (only rehashes new/modified files based on path + size + mtime)
  - "Full rescan" - ignores cache, rehashes everything

### Build & Distribution

- No auto-update functionality
- No code signing/notarization (users will bypass security warnings)
- Provide build scripts/instructions for Mac and Windows
- Granular git commits: average 5 files or fewer per commit

### Developer Setup

The project must be easy to set up for developers who don't have prerequisites installed:

- **README with setup instructions**: Step-by-step guide for both Mac and Windows
- **Prerequisites script**: Automated script to check/install required tools (Rust, Node.js, etc.)
- **Single command build**: After setup, building should be a single command (e.g., `npm run build`)
- **Development mode**: Easy command to run in dev mode with hot reload (e.g., `npm run dev`)
- **Clear error messages**: If prerequisites are missing, show helpful messages about what to install

### Version Control

- **Granular commits**: Each logical step should be a separate git commit
- **Small commits**: Target average of 5 files or fewer per commit
- **Meaningful messages**: Each commit message should clearly describe what was added/changed
- **Atomic changes**: Each commit should represent a complete, working state when possible

---

## User Interface

### Visual Design

- **Theme**: Follows system dark/light mode by default, with user override option (force light or dark)
- **Style**: Fully native look and feel on each platform (Mac-style on Mac, Windows-style on Windows)
- **Language**: English only for v1

### Main Layout

- **Master-Detail**: Duplicate groups listed on left, selected group details in right side panel
- **Path Display**: Truncate middle of long paths (e.g., `/Users/.../Documents/photo.jpg`), full path on hover

### Results View

- **Default Sort**: By total wasted space (groups with largest reclaimable space first)
- **File Type Filters**: Flat list of all duplicates with filter buttons for file types (Images, Videos, Documents, Audio, Other)
- **Search**: Full text search plus filters (by type, by folder, by size range)
- **Image Thumbnails**: Generate and cache thumbnails for image files in the results list
- **Date Display**: Show both creation date and last modified date for each file
- **"Original" Badge**: Oldest file (by creation date/time) in each group marked as "original" / recommended to keep

### Progress Display

During scans, show:

- Progress percentage
- Files scanned count (X of Y)
- Estimated time remaining
- Current file/folder being processed

### Context Menu (Right-Click)

- Open file (in default app)
- Reveal in Finder/Explorer
- Open containing folder
- Copy file path
- View file info
- Mark for deletion

### Keyboard Support

Full keyboard navigation and shortcuts for all major actions:

- Start/pause/cancel scan
- Navigate between groups and files
- Select/deselect files
- Mark for deletion
- Commit deletion
- Search/filter

---

## Workflows

### Onboarding & Permissions

1. **Blocking Permission Wizard**: App requires necessary permissions before allowing scans
2. Step-by-step guide with screenshots for granting Full Disk Access (Mac) and equivalent Windows permissions
3. No tutorial after permissions - drop user directly into app

### Scanning

1. **Scope Selection**: User selects drives/folders to scan (remembers last scan settings)
2. **Live Results**: Duplicate groups stream into the UI as they're discovered - user can start reviewing while scan continues
3. **Pause/Resume**: Scan state persisted to disk - survives app restarts
4. **Error Handling**:
   - Skip unreadable files (permission denied, corrupted, locked)
   - Show count badge of skipped files
   - Allow user to retry skipped files

### Selection & Deletion

1. **Smart Selection Options**:
   - Select all except oldest (per group)
   - Select by location (all duplicates in specific folders)
   - Select by path depth
   - Manual checkbox selection
2. **Live Space Counter**: Running total of space to be reclaimed updates as selections change
3. **Delete-All Warning**: If user selects all copies in a group, show prominent warning but allow if confirmed
4. **Pre-Delete Verification**: Re-verify file hash before deletion, skip if file changed, notify user
5. **Confirmation Dialog**: Show count, total size, and a few example files (not full list)
6. **Post-Deletion**: Summary of what was deleted, reminder about system trash for undo

### File Preview

- No built-in preview pane
- Files open externally in default app
- Image thumbnails shown in results list

---

## Data Storage

All data in local SQLite database:

1. **Scan Results**: Duplicate groups, file paths, hashes, sizes, dates
2. **File Index Cache**: For incremental scanning (path, size, mtime, hash)
3. **User Settings**: Last scan configuration, protected folders, theme preference, parallelism setting
4. **Deletion History**: Persistent log of all deletions with timestamps, viewable across sessions. Each record includes the path of the retained duplicate copy (if any) so users can locate the remaining file

---

## App Lifecycle

- **Traditional App**: Only runs when explicitly launched (no auto-start)
- **Optional Tray Mode**: User can enable "minimize to tray" in settings
- **No Analytics**: Completely offline, no telemetry or data collection

---

## Error Handling

| Scenario                      | Behavior                                          |
| ----------------------------- | ------------------------------------------------- |
| Permission denied             | Skip file, increment skip counter, allow retry    |
| File corrupted/unreadable     | Skip file, log error, allow retry                 |
| File locked by another app    | Skip file, increment skip counter, allow retry    |
| File changed since scan       | Skip deletion, notify user, continue with others  |
| File moved/deleted since scan | Skip deletion, notify user, continue with others  |
| Disk full during scan         | Pause scan, notify user                           |
| Network drive disconnected    | Skip remaining files on that drive, continue scan |

---

## Future Considerations (Not in v1)

- Mobile apps (Android, iOS)
- Multiple language support / localization
- Export results to CSV/JSON
- Auto-update functionality
- Code signing and notarization
- Scheduled/automated scans
- "Similar" image detection (perceptual hashing, not just byte-identical)

---

## Summary of Key Decisions

| Decision            | Choice                                        |
| ------------------- | --------------------------------------------- |
| Hash algorithm      | BLAKE3                                        |
| Detection approach  | Multi-stage (size → partial hash → full hash) |
| Framework           | Tauri                                         |
| Frontend            | Svelte + TypeScript                           |
| Database            | SQLite                                        |
| Symlinks            | Skip entirely                                 |
| File size limits    | None                                          |
| Default exclusions  | None (scan everything)                        |
| Original detection  | Oldest by creation date/time                  |
| Results sorting     | By total wasted space                         |
| Theme               | System default + user override                |
| UI style            | Fully native per-platform                     |
| Preview             | External only, thumbnails for images          |
| Layout              | Master-detail (side panel)                    |
| Pause/resume        | Yes, persisted to disk                        |
| Protected folders   | User-defined only                             |
| Delete confirmation | Summary + count                               |
| Deletion safety     | Verify hash before delete                     |
| Delete-all-copies   | Warn but allow                                |
| Analytics           | None                                          |
| Auto-update         | None                                          |
| Code signing        | None                                          |
| Tutorial            | None                                          |
| Keyboard support    | Full                                          |
| Commit size         | ~5 files or fewer                             |
| Developer setup     | Automated scripts + README                    |
