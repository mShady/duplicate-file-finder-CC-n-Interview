# Issue #11: Typed API Wrapper Layer — Implementation Plan

## Overview

Create a typed API wrapper layer (`src/lib/api/`) that centralises all `invoke()` calls behind type-safe functions. Every frontend component currently calls `invoke()` directly with inline type assertions; this plan extracts those calls into domain-grouped modules so callers get autocomplete, compile-time safety, and a single place to audit the frontend↔backend contract.

## Current State Analysis

**8 raw `invoke()` calls** scattered across 2 files:

| File | Commands called |
|---|---|
| `src/App.svelte` | `get_scan_results`, `get_setting`, `set_setting`, `start_scan`, `cancel_scan`, `delete_files` |
| `src/lib/components/DeletionHistoryPanel.svelte` | `get_deletion_history_summary`, `get_deletion_history` |

**7 additional registered commands** not yet called from the frontend (reserved for future plan phases):
`get_all_settings`, `add_protected_folder`, `remove_protected_folder`, `get_protected_folders`, `is_path_protected`, `get_scan_progress`, `is_scanning`

**Types already exist** in `src/lib/types.ts` (128 lines) and mirror the Rust structs correctly. The gap is not missing types — it's the absence of a wrapper layer around the raw `invoke()` calls.

### Key Discoveries:
- `src/App.svelte:47` — `invoke<DetectionResult | null>('get_scan_results')` with inline type param
- `src/App.svelte:62` — `invoke<string | null>('get_setting', { key: 'last_scan_paths' })` with inline type param
- `src/App.svelte:97` — `invoke('start_scan', { request: { ... } })` with **no** return type assertion (return value is ignored)
- `src/lib/components/DeletionHistoryPanel.svelte:29` — `invoke<[number, number]>('get_deletion_history_summary')` with tuple type
- `src/lib/types.ts` has all response types but no `ScanRequest` type (it's built inline in `App.svelte`)
- The Rust backend groups commands into 4 files: `scan.rs`, `settings.rs`, `deletion.rs`, `protected.rs` — we mirror this

## Desired End State

A `src/lib/api/` directory with domain-grouped wrapper modules covering all 15 registered Tauri commands. Frontend components import typed functions (e.g. `startScan(request)`) instead of calling `invoke()` directly. Existing call sites in `App.svelte` and `DeletionHistoryPanel.svelte` are migrated.

### Verification:
- `npm run check` passes (svelte-check confirms type correctness)
- `npm run lint` passes
- `npm run build` produces a clean production build
- `npm test` passes (existing tests still work)
- `cargo test` and `cargo clippy` pass (backend unchanged)
- `npx prettier --check .` passes

## What We're NOT Doing

- **Not changing** the Rust backend in any way
- **Not modifying** `scanStore.svelte.ts` event listeners (those use `listen()`, not `invoke()`)
- **Not adding** runtime validation or error transformation — the wrappers are thin typed pass-throughs
- **Not wiring up** the 7 currently-unused commands to UI components (that's for future plan phases)
- **Not adding** unit tests for the API module itself — these are thin `invoke()` wrappers; testing them would just be testing Tauri's `invoke`

## Implementation Approach

Single phase. Create the API modules, add the missing `ScanRequest` type, migrate existing call sites, then verify.

---

## Phase 1: Typed API Layer + Migration

### Overview

Create `src/lib/api/` with four domain modules plus an index barrel, add the missing `ScanRequest` type to `types.ts`, then update `App.svelte` and `DeletionHistoryPanel.svelte` to import from the new API layer.

### Changes Required:

#### 1.1 Add missing request type to `src/lib/types.ts`

**File**: `src/lib/types.ts`
**Changes**: Add `ScanRequest` interface (mirrors Rust `ScanRequest` in `commands/scan.rs:17-21`)

```typescript
export interface ScanRequest {
  paths: string[];
  parallelism?: string;
}
```

#### 1.2 Create `src/lib/api/scan.ts`

**File**: `src/lib/api/scan.ts` *(new)*
**Changes**: Typed wrappers for the 5 scan commands

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { DetectionResult, ScanProgress, ScanRequest } from '$lib/types';

interface ScanResponse {
  session_id: number;
  message: string;
}

export async function startScan(request: ScanRequest): Promise<ScanResponse> {
  return invoke<ScanResponse>('start_scan', { request });
}

export async function cancelScan(): Promise<void> {
  return invoke<void>('cancel_scan');
}

export async function getScanProgress(): Promise<ScanProgress | null> {
  return invoke<ScanProgress | null>('get_scan_progress');
}

export async function isScanning(): Promise<boolean> {
  return invoke<boolean>('is_scanning');
}

export async function getScanResults(): Promise<DetectionResult | null> {
  return invoke<DetectionResult | null>('get_scan_results');
}
```

#### 1.3 Create `src/lib/api/settings.ts`

**File**: `src/lib/api/settings.ts` *(new)*
**Changes**: Typed wrappers for the 3 settings commands

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { Setting } from '$lib/types';

export async function getSetting(key: string): Promise<string | null> {
  return invoke<string | null>('get_setting', { key });
}

export async function setSetting(key: string, value: string): Promise<void> {
  return invoke<void>('set_setting', { key, value });
}

export async function getAllSettings(): Promise<Setting[]> {
  return invoke<Setting[]>('get_all_settings');
}
```

#### 1.4 Create `src/lib/api/deletion.ts`

**File**: `src/lib/api/deletion.ts` *(new)*
**Changes**: Typed wrappers for the 3 deletion commands

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { DeletionRequest, DeleteFilesResponse, DeletionRecord } from '$lib/types';

interface DeleteFilesRequestPayload {
  files: DeletionRequest[];
  kept_paths: Record<string, string>;
  group_ids: Record<string, number>;
}

export async function deleteFiles(
  request: DeleteFilesRequestPayload,
): Promise<DeleteFilesResponse> {
  return invoke<DeleteFilesResponse>('delete_files', { request });
}

export async function getDeletionHistorySummary(): Promise<[number, number]> {
  return invoke<[number, number]>('get_deletion_history_summary');
}

export async function getDeletionHistory(
  limit: number,
  offset: number,
): Promise<DeletionRecord[]> {
  return invoke<DeletionRecord[]>('get_deletion_history', { limit, offset });
}
```

#### 1.5 Create `src/lib/api/protected.ts`

**File**: `src/lib/api/protected.ts` *(new)*
**Changes**: Typed wrappers for the 4 protected-folder commands

```typescript
import { invoke } from '@tauri-apps/api/core';
import type { ProtectedFolder } from '$lib/types';

export async function addProtectedFolder(path: string): Promise<number> {
  return invoke<number>('add_protected_folder', { path });
}

export async function removeProtectedFolder(id: number): Promise<boolean> {
  return invoke<boolean>('remove_protected_folder', { id });
}

export async function getProtectedFolders(): Promise<ProtectedFolder[]> {
  return invoke<ProtectedFolder[]>('get_protected_folders');
}

export async function isPathProtected(path: string): Promise<boolean> {
  return invoke<boolean>('is_path_protected', { path });
}
```

#### 1.6 Create `src/lib/api/index.ts`

**File**: `src/lib/api/index.ts` *(new)*
**Changes**: Barrel re-export

```typescript
export { startScan, cancelScan, getScanProgress, isScanning, getScanResults } from './scan';
export { getSetting, setSetting, getAllSettings } from './settings';
export { deleteFiles, getDeletionHistorySummary, getDeletionHistory } from './deletion';
export {
  addProtectedFolder,
  removeProtectedFolder,
  getProtectedFolders,
  isPathProtected,
} from './protected';
```

#### 1.7 Migrate `src/App.svelte`

**File**: `src/App.svelte`
**Changes**:
- Remove `import { invoke } from '@tauri-apps/api/core'`
- Add `import { startScan, cancelScan, getScanResults, getSetting, setSetting, deleteFiles } from '$lib/api'`
- Replace all 6 `invoke()` calls with the typed wrapper functions

Before → After examples:
```typescript
// Before (line 47):
const existing = await invoke<DetectionResult | null>('get_scan_results');
// After:
const existing = await getScanResults();

// Before (line 62):
const value = await invoke<string | null>('get_setting', { key: 'last_scan_paths' });
// After:
const value = await getSetting('last_scan_paths');

// Before (lines 73-76):
await invoke('set_setting', { key: 'last_scan_paths', value: JSON.stringify(selectedPaths) });
// After:
await setSetting('last_scan_paths', JSON.stringify(selectedPaths));

// Before (lines 97-102):
await invoke('start_scan', { request: { paths: selectedPaths, parallelism: 'normal' } });
// After:
await startScan({ paths: selectedPaths, parallelism: 'normal' });

// Before (line 111):
await invoke('cancel_scan');
// After:
await cancelScan();

// Before (line 148):
const response = await invoke<DeleteFilesResponse>('delete_files', {
  request: { files: requests, kept_paths: keptPaths, group_ids: groupIds },
});
// After:
const response = await deleteFiles({
  files: requests, kept_paths: keptPaths, group_ids: groupIds,
});
```

#### 1.8 Migrate `src/lib/components/DeletionHistoryPanel.svelte`

**File**: `src/lib/components/DeletionHistoryPanel.svelte`
**Changes**:
- Remove `import { invoke } from '@tauri-apps/api/core'`
- Add `import { getDeletionHistorySummary, getDeletionHistory } from '$lib/api'`
- Replace both `invoke()` calls

Before → After:
```typescript
// Before (line 29):
const [count, freed] = await invoke<[number, number]>('get_deletion_history_summary');
// After:
const [count, freed] = await getDeletionHistorySummary();

// Before (lines 48-51):
const records = await invoke<DeletionRecord[]>('get_deletion_history', {
  limit: pageSize, offset: page * pageSize,
});
// After:
const records = await getDeletionHistory(pageSize, page * pageSize);
```

### Success Criteria:

#### Automated Verification:
- [ ] `npm run check` passes (svelte-check)
- [ ] `npm run lint` passes (ESLint)
- [ ] `npm run build` succeeds (Vite production build)
- [ ] `npm test` passes (Vitest)
- [ ] `cargo test` passes (Rust unit tests)
- [ ] `cargo clippy` produces zero warnings
- [ ] `npx prettier --check .` passes

#### Manual Verification:
- [ ] No remaining `import { invoke }` from `@tauri-apps/api/core` in component files (only in `src/lib/api/*.ts`)
- [ ] App starts and scans a folder successfully
- [ ] Deletion flow works end-to-end
- [ ] Deletion history panel loads correctly

---

## Conflict Assessment with Existing Plans

**No conflicts.** This plan operates at a layer below the existing plans:

- **Plans 08 (Settings), 12 (Permissions)** will add new components that call settings/protected-folder commands. With this plan in place, those components will import from `$lib/api` instead of calling `invoke()` directly — strictly beneficial, not conflicting.
- **Plans 09 (File Operations), 10 (Filtering)** may add new backend commands. When they do, new wrapper functions get added to the appropriate `api/*.ts` module. No existing API functions need to change.
- **Plans 05, 06, 07** are already implemented. This plan migrates their existing `invoke()` call sites without changing behaviour.

## References

- Issue: `ISSUES.md` — Issue #11
- Rust command registration: `src-tauri/src/lib.rs:56-76`
- Rust command implementations: `src-tauri/src/commands/{scan,settings,deletion,protected}.rs`
- Existing frontend types: `src/lib/types.ts`
- Existing `invoke()` call sites: `src/App.svelte`, `src/lib/components/DeletionHistoryPanel.svelte`
