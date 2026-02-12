# Issues #6 & #12: Consolidate Format Test Files & Import from Utility Module

## Overview

`formatters.test.ts` re-implements 6 formatting functions inline instead of importing from `$lib/utils/format.ts`. There are also two overlapping test files (`formatters.test.ts` and `format.test.ts`) that both test `formatBytes`. This plan consolidates them into a single test file that imports from the canonical utility module, and adds missing functions to `format.ts`.

## Current State Analysis

### Test Files

| File                             | Lines | Tests                                                                                                                           | Imports from `format.ts`?      |
| -------------------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------- | ------------------------------ |
| `tests/utils/format.test.ts`     | 49    | `formatBytes` (8 tests, including negative + overflow)                                                                          | Yes                            |
| `tests/utils/formatters.test.ts` | 260   | `formatBytes` (7 tests), `getDirectory` (7), `getFileName` (5), `formatDate` (3), `getFileTypeIcon` (5), `getFileExtension` (4) | No — all re-implemented inline |

### Utility Module

`src/lib/utils/format.ts` currently exports 3 functions:

- `formatBytes(bytes: number): string` — includes negative number and overflow-safe clamping
- `formatDate(timestamp: number): string`
- `getFileName(path: string): string`

### Functions Missing from `format.ts`

These are tested in `formatters.test.ts` but don't exist as exports:

- `getDirectory(path: string, maxLength?: number): string` — directory extraction with middle-ellipsis truncation
- `getFileTypeIcon(ext: string): string` — maps file extension to category string
- `getFileExtension(path: string): string` — extracts lowercase extension from path

### Key Discoveries

- `format.ts:6-12` — `formatBytes` already handles negative numbers and index clamping (improvements over the inline version in `formatters.test.ts`)
- `formatters.test.ts:8-14` — inline `formatBytes` lacks negative number handling and has no overflow guard
- `format.test.ts:37-47` — has 2 extra tests (negative input, PB overflow clamping) that `formatters.test.ts` lacks
- The inline `formatDate` in `formatters.test.ts:158-174` is identical to `format.ts:18-34`
- The inline `getFileName` in `formatters.test.ts:127-132` is identical to `format.ts:39-44`

## Desired End State

- **One test file**: `tests/utils/format.test.ts` containing all tests, importing all functions from `$lib/utils/format.ts`
- **One utility module**: `src/lib/utils/format.ts` exporting all 6 functions
- **`formatters.test.ts` deleted**
- All existing tests preserved (union of both files)
- All automated verification passes

## What We're NOT Doing

- Changing any component imports — components already import from `format.ts` (Issue #3 was already fixed)
- Modifying test logic or assertions — only changing where functions come from
- Adding new tests beyond what already exists in both files

## Conflict Assessment with Future Plans (07–15)

| Plan                     | Conflict                                                                                                      | Action Required                                                                                                                      |
| ------------------------ | ------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| **07 (Scan Progress)**   | Phase 7.4 defines `formatBytes`, `formatNumber`, `formatTimeRemaining` inline in `ScanProgressDisplay.svelte` | **None now.** When plan 07 is implemented, these should be imported from `format.ts` instead of defined inline. Add note to plan 07. |
| **08 (Settings)**        | None                                                                                                          | —                                                                                                                                    |
| **09 (File Operations)** | Phase 9.5 `FileInfoDialog.svelte` defines `formatBytes`, `formatDate`, `getFileExtension` inline              | **None now.** When plan 09 is implemented, these should be imported from `format.ts`. Add note to plan 09.                           |
| **10 (Filtering)**       | Phase 10.1 `filters.ts` defines `getExtension()` inline (same logic as our `getFileExtension`)                | **None now.** When plan 10 is implemented, it should import `getFileExtension` from `format.ts`. Add note to plan 10.                |
| **11 (Keyboard Nav)**    | None                                                                                                          | —                                                                                                                                    |
| **12 (Permissions)**     | None                                                                                                          | —                                                                                                                                    |
| **13 (Error Handling)**  | Phase 13.4.3 `DiskFullAlert.svelte` defines `formatBytes` inline                                              | **None now.** When plan 13 is implemented, it should import from `format.ts`. Add note to plan 13.                                   |
| **14 (Platform Polish)** | None                                                                                                          | —                                                                                                                                    |
| **15 (System Tray)**     | None                                                                                                          | —                                                                                                                                    |

**Verdict**: No blocking conflicts. This fix is beneficial — it establishes `format.ts` as the canonical source, which all future plans should use.

## Implementation Approach

Single phase: add missing functions to `format.ts`, merge all tests into `format.test.ts`, delete `formatters.test.ts`.

---

## Phase 1: Consolidate Format Utilities and Tests

### Overview

Add `getDirectory`, `getFileTypeIcon`, and `getFileExtension` to the canonical utility module, merge all tests into one file, and delete the redundant file.

### Changes Required

#### 1.1 Add Missing Functions to `format.ts`

**File**: `src/lib/utils/format.ts`
**Changes**: Add `getDirectory`, `getFileTypeIcon`, and `getFileExtension` exports

```typescript
/**
 * Extract the directory from a file path, with optional middle-ellipsis truncation.
 */
export function getDirectory(path: string, maxLength: number = 50): string {
  if (!path) return '';

  const parts = path.split(PATH_SEP);
  if (parts.length <= 1) return '';

  parts.pop(); // Remove filename
  const dir = parts.join('/');

  if (dir.length <= maxLength) {
    return dir;
  }

  // Middle ellipsis truncation
  const ellipsis = '/...';
  const availableLength = maxLength - ellipsis.length;
  if (availableLength <= 0) return dir.slice(0, maxLength);

  const startLength = Math.ceil(availableLength * 0.4);
  const endLength = Math.floor(availableLength * 0.6);

  const start = dir.slice(0, startLength);
  const end = dir.slice(-endLength);

  return `${start}${ellipsis}${end}`;
}

/**
 * Classify a file extension into a category string.
 */
export function getFileTypeIcon(ext: string): string {
  const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg'];
  const videoExts = ['mp4', 'mov', 'avi', 'mkv', 'webm'];
  const audioExts = ['mp3', 'wav', 'flac', 'aac', 'm4a'];
  const docExts = ['pdf', 'doc', 'docx', 'txt', 'rtf', 'md'];

  if (imageExts.includes(ext)) return 'image';
  if (videoExts.includes(ext)) return 'video';
  if (audioExts.includes(ext)) return 'audio';
  if (docExts.includes(ext)) return 'document';
  return 'file';
}

/**
 * Extract the lowercase file extension from a path.
 */
export function getFileExtension(path: string): string {
  if (!path) return '';
  const ext = path.split('.').pop()?.toLowerCase() || '';
  return ext;
}
```

#### 1.2 Merge Tests into `format.test.ts`

**File**: `tests/utils/format.test.ts`
**Changes**: Replace contents with the union of all tests from both files, importing all functions from `format.ts`

The merged file will contain:

- `formatBytes` — 9 unique tests (union of both files: 7 from `formatters.test.ts` + 2 extras from `format.test.ts`)
- `getDirectory` — 7 tests (from `formatters.test.ts`)
- `getFileName` — 5 tests (from `formatters.test.ts`)
- `formatDate` — 3 tests (from `formatters.test.ts`)
- `getFileTypeIcon` — 5 tests (from `formatters.test.ts`)
- `getFileExtension` — 4 tests (from `formatters.test.ts`)

All functions imported from `../../src/lib/utils/format`.

#### 1.3 Delete `formatters.test.ts`

**File**: `tests/utils/formatters.test.ts`
**Action**: Delete

### Success Criteria

#### Automated Verification

- [ ] `npm test` passes — all tests green
- [ ] `npm run check` passes — svelte-check / TypeScript types
- [ ] `npm run lint` passes — ESLint
- [ ] `npm run build` passes — Vite production build
- [ ] `npx prettier --check .` passes — formatting
- [ ] `cargo test` passes — backend unaffected
- [ ] `cargo clippy` passes — backend unaffected

#### Manual Verification

- [ ] `formatters.test.ts` no longer exists
- [ ] `format.test.ts` imports all 6 functions from `$lib/utils/format.ts`
- [ ] No inline function re-implementations remain in the test file

---

## Phase 2: Update Future Plans with Import Guidance

### Overview

Add notes to plans 07, 09, 10, and 13 reminding implementers to import format utilities from `$lib/utils/format.ts` instead of defining them inline.

### Changes Required

#### 2.1 Update Plan 07 (Phase 7.4)

**File**: `plans/07-scan-progress.md`
**Changes**: Add an import note before the `ScanProgressDisplay.svelte` code block, and replace the inline `formatBytes` function with an import. Also add `formatNumber` and `formatTimeRemaining` to `format.ts` guidance.

#### 2.2 Update Plan 09 (Phase 9.5)

**File**: `plans/09-file-operations.md`
**Changes**: Add an import note before the `FileInfoDialog.svelte` code block. Replace inline `formatBytes`, `formatDate`, `getFileExtension` with imports from `$lib/utils/format`.

#### 2.3 Update Plan 10 (Phase 10.1)

**File**: `plans/10-filtering-search.md`
**Changes**: Replace the inline `getExtension` function in `filters.ts` with an import of `getFileExtension` from `$lib/utils/format`.

#### 2.4 Update Plan 13 (Phase 13.4.3)

**File**: `plans/13-error-handling.md`
**Changes**: Replace the inline `formatBytes` in `DiskFullAlert.svelte` with an import from `$lib/utils/format`.

### Success Criteria

#### Automated Verification

- [ ] No automated checks needed (documentation-only changes)

#### Manual Verification

- [ ] Plans 07, 09, 10, 13 reference `$lib/utils/format` imports instead of inline definitions
- [ ] No inline format function re-implementations remain in plan code blocks

---

## Testing Strategy

### Unit Tests (Phase 1)

All 33 tests in the merged `format.test.ts` must pass:

- `formatBytes`: 0 bytes, bytes, KB, MB, GB, TB, large numbers, negative input, PB overflow clamping
- `getDirectory`: empty path, filename only, short paths, long path truncation, within max length, Windows paths, very short max length
- `getFileName`: Unix path, Windows path, filename only, empty string, trailing separator
- `formatDate`: valid timestamp, zero, negative
- `getFileTypeIcon`: image, video, audio, document, unknown
- `getFileExtension`: basic, no extension, multiple dots, empty string

### Integration Tests

None needed — these are pure utility functions with no side effects.

## References

- Issue #6: `formatters.test.ts` re-implements functions inline instead of importing from utility modules
- Issue #12: Two overlapping test files: `formatters.test.ts` and `format.test.ts`
- Current utility module: `src/lib/utils/format.ts`
