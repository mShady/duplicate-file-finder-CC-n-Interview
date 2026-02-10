# File 05: Scan Results & Groups UI

## Overview

This file covers building the full results UI with a master-detail layout showing duplicate groups on the left and file details on the right. By the end of this file, you'll have a complete results view that matches the specification.

## Prerequisites

- Completed Files 01-04

---

## Phase 5.1: Create Master-Detail Layout Component

### Overview
Create the main layout component with a resizable split panel.

### Changes Required

#### 5.1.1 Create Layout Component

**File**: `src/lib/components/MasterDetailLayout.svelte`

```svelte
<script lang="ts">
  interface Props {
    masterWidth?: number;
    minMasterWidth?: number;
    minDetailWidth?: number;
  }

  let { masterWidth = 400, minMasterWidth = 300, minDetailWidth = 400 }: Props = $props();

  let containerRef: HTMLElement;
  let isDragging = $state(false);
  let currentWidth = $state(masterWidth);

  function startDrag(e: MouseEvent) {
    isDragging = true;
    e.preventDefault();
  }

  function onDrag(e: MouseEvent) {
    if (!isDragging || !containerRef) return;

    const containerRect = containerRef.getBoundingClientRect();
    const newWidth = e.clientX - containerRect.left;

    const maxWidth = containerRect.width - minDetailWidth;
    currentWidth = Math.max(minMasterWidth, Math.min(maxWidth, newWidth));
  }

  function stopDrag() {
    isDragging = false;
  }
</script>

<svelte:window onmousemove={onDrag} onmouseup={stopDrag} />

<div class="master-detail" bind:this={containerRef}>
  <div class="master-panel" style="width: {currentWidth}px">
    <slot name="master" />
  </div>

  <div
    class="divider"
    class:dragging={isDragging}
    onmousedown={startDrag}
    role="separator"
    aria-orientation="vertical"
    tabindex="0"
  ></div>

  <div class="detail-panel">
    <slot name="detail" />
  </div>
</div>

<style>
  .master-detail {
    display: flex;
    height: 100%;
    overflow: hidden;
  }

  .master-panel {
    flex-shrink: 0;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .divider {
    width: 4px;
    background: var(--border);
    cursor: col-resize;
    flex-shrink: 0;
    transition: background 0.2s;
  }

  .divider:hover,
  .divider.dragging {
    background: var(--primary);
  }

  .detail-panel {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

#### Manual Verification
- [ ] Layout renders with two panels
- [ ] Divider can be dragged to resize panels

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.
---

## Phase 5.1.5: Live Duplicate Streaming Subscription Pattern

### Overview
This phase specifies the complete frontend event subscription pattern for receiving live duplicate discovery updates during scanning.

### Event Subscription Architecture

#### Full Event Subscription Setup

**File**: `src/lib/stores/scanStore.ts`

Create a centralized scan store for managing all scan-related events:

```typescript
import { writable, derived } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type {
  DuplicateGroup,
  ScanProgress,
  ScanComplete,
  DetectionResult
} from '$lib/types';

// Event payload types
interface ScanPhaseEvent {
  phase: 'collecting' | 'partial_hashing' | 'full_hashing' | 'storing' | 'complete';
  message: string;
}

interface DetectionProgressEvent {
  partial_hashes: number;
  full_hashes: number;
  groups_found: number;
}

interface ScanErrorEvent {
  session_id: number;
  error: string;
}

// Store state
interface ScanState {
  isScanning: boolean;
  phase: ScanPhaseEvent['phase'] | 'idle' | 'error';
  phaseMessage: string;
  progress: ScanProgress | null;
  detectionProgress: DetectionProgressEvent | null;
  liveGroups: DuplicateGroup[];  // Groups streamed during scan
  finalResult: DetectionResult | null;
  scanComplete: ScanComplete | null;
  error: string | null;
}

function createScanStore() {
  const { subscribe, set, update } = writable<ScanState>({
    isScanning: false,
    phase: 'idle',
    phaseMessage: '',
    progress: null,
    detectionProgress: null,
    liveGroups: [],
    finalResult: null,
    scanComplete: null,
    error: null,
  });

  let unlisteners: UnlistenFn[] = [];

  return {
    subscribe,

    // Initialize event listeners
    async init() {
      // Clean up any existing listeners
      this.cleanup();

      unlisteners = [
        // File discovery progress
        await listen<ScanProgress>('scan-progress', (e) => {
          update(state => ({ ...state, progress: e.payload }));
        }),

        // Phase transitions
        await listen<ScanPhaseEvent>('scan-phase', (e) => {
          update(state => ({
            ...state,
            phase: e.payload.phase,
            phaseMessage: e.payload.message,
          }));
        }),

        // LIVE DUPLICATE STREAMING - Key pattern!
        await listen<DuplicateGroup>('duplicate-found', (e) => {
          update(state => ({
            ...state,
            // Append new group to live list, sorted by wasted space
            liveGroups: [...state.liveGroups, e.payload]
              .sort((a, b) => b.wasted_space - a.wasted_space),
          }));
        }),

        // Detection progress (hashing stats)
        await listen<DetectionProgressEvent>('detection-progress', (e) => {
          update(state => ({ ...state, detectionProgress: e.payload }));
        }),

        // Final results
        await listen<DetectionResult>('scan-results', (e) => {
          update(state => ({
            ...state,
            finalResult: e.payload,
            // Replace live groups with final sorted results
            liveGroups: e.payload.groups,
          }));
        }),

        // Scan completion
        await listen<ScanComplete>('scan-complete', (e) => {
          update(state => ({
            ...state,
            isScanning: false,
            phase: 'complete',
            scanComplete: e.payload,
          }));
        }),

        // Error handling
        await listen<ScanErrorEvent>('scan-error', (e) => {
          update(state => ({
            ...state,
            isScanning: false,
            phase: 'error',
            error: e.payload.error,
          }));
        }),
      ];
    },

    // Start a new scan
    startScan() {
      update(state => ({
        ...state,
        isScanning: true,
        phase: 'collecting',
        phaseMessage: 'Starting scan...',
        progress: null,
        detectionProgress: null,
        liveGroups: [],  // Clear previous results
        finalResult: null,
        scanComplete: null,
        error: null,
      }));
    },

    // Reset store state
    reset() {
      set({
        isScanning: false,
        phase: 'idle',
        phaseMessage: '',
        progress: null,
        detectionProgress: null,
        liveGroups: [],
        finalResult: null,
        scanComplete: null,
        error: null,
      });
    },

    // Cleanup listeners
    cleanup() {
      unlisteners.forEach(fn => fn());
      unlisteners = [];
    },
  };
}

export const scanStore = createScanStore();

// Derived stores for convenience
export const isScanning = derived(scanStore, $s => $s.isScanning);
export const currentPhase = derived(scanStore, $s => $s.phase);
export const duplicateGroups = derived(scanStore, $s =>
  $s.finalResult?.groups ?? $s.liveGroups
);
export const totalWastedSpace = derived(scanStore, $s =>
  $s.finalResult?.total_wasted_space ??
  $s.liveGroups.reduce((sum, g) => sum + g.wasted_space, 0)
);
```

#### Component Usage Pattern

**File**: `src/lib/components/ResultsView.svelte`

Using the store in a component:

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { scanStore, duplicateGroups, totalWastedSpace, isScanning, currentPhase } from '$lib/stores/scanStore';

  // Initialize listeners on mount
  onMount(() => {
    scanStore.init();
  });

  // Cleanup on destroy
  onDestroy(() => {
    scanStore.cleanup();
  });

  // Reactive access to store values
  let groups = $derived($duplicateGroups);
  let wasted = $derived($totalWastedSpace);
  let scanning = $derived($isScanning);
  let phase = $derived($currentPhase);
</script>

<!-- Live updating results list -->
<div class="results">
  {#if scanning}
    <div class="scanning-indicator">
      {phase}: Finding duplicates...
      {#if groups.length > 0}
        <span class="live-count">{groups.length} groups found so far</span>
      {/if}
    </div>
  {/if}

  <!-- Groups update in real-time as duplicate-found events arrive -->
  {#each groups as group (group.id)}
    <DuplicateGroupCard {group} />
  {/each}
</div>
```

### Live Streaming UI Behavior

#### Visual Feedback During Streaming

1. **New groups animate in**: Use CSS transitions when groups are added
2. **List re-sorts**: As new groups arrive, list re-sorts by wasted space
3. **Running totals update**: "X groups, Y wasted" updates in real-time
4. **Phase indicator**: Shows current detection phase

```svelte
<style>
  /* Animate new groups sliding in */
  .group-card {
    animation: slideIn 0.3s ease-out;
  }

  @keyframes slideIn {
    from {
      opacity: 0;
      transform: translateY(-10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
```

#### Performance Considerations

1. **Virtual scrolling**: For large result sets (>100 groups), use virtual scrolling
2. **Batch UI updates**: Svelte's reactivity handles this automatically
3. **Debounce re-sorting**: Only re-sort every 500ms during streaming to reduce jank

```typescript
// Debounced sorting for live updates
let sortTimeout: number | null = null;

await listen<DuplicateGroup>('duplicate-found', (e) => {
  update(state => {
    const newGroups = [...state.liveGroups, e.payload];

    // Debounce sorting during rapid updates
    if (sortTimeout) clearTimeout(sortTimeout);
    sortTimeout = setTimeout(() => {
      update(s => ({
        ...s,
        liveGroups: s.liveGroups.sort((a, b) => b.wasted_space - a.wasted_space)
      }));
    }, 500);

    return { ...state, liveGroups: newGroups };
  });
});
```

### Type Definitions Update

**File**: `src/lib/types.ts`

Add new event types:

```typescript
// Add to existing types.ts

export interface ScanPhaseEvent {
  phase: 'collecting' | 'partial_hashing' | 'full_hashing' | 'storing' | 'complete';
  message: string;
}

export interface DetectionProgressEvent {
  partial_hashes: number;
  full_hashes: number;
  groups_found: number;
}

export interface ScanErrorEvent {
  session_id: number;
  error: string;
}

// Update ScanProgress to include started_at_ms for ETA calculation
export interface ScanProgress {
  total_files: number;
  processed_files: number;
  total_bytes: number;
  current_path: string | null;
  skipped_files: number;
  estimated_total: number | null;
  started_at_ms?: number;
  estimated_time_remaining_ms?: number;
}
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes
- [ ] TypeScript types are correctly defined

#### Manual Verification
- [ ] Live groups appear as they are discovered
- [ ] Groups re-sort by wasted space during streaming
- [ ] Final results replace live groups seamlessly
- [ ] No memory leaks from event listeners
- [ ] UI remains responsive with 100+ live groups

---

## Phase 5.2: Create Duplicate Groups List Component

### Overview
Create the component that displays the list of duplicate groups in the master panel.

### Changes Required

#### 5.2.1 Create Groups List

**File**: `src/lib/components/DuplicateGroupsList.svelte`

```svelte
<script lang="ts">
  import type { DuplicateGroup } from '$lib/types';

  interface Props {
    groups: DuplicateGroup[];
    selectedGroupId: number | null;
    onSelect: (group: DuplicateGroup) => void;
  }

  let { groups, selectedGroupId, onSelect }: Props = $props();

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function getFileExtension(group: DuplicateGroup): string {
    const firstFile = group.files[0];
    if (!firstFile) return '';
    const path = firstFile.path;
    const ext = path.split('.').pop()?.toLowerCase() || '';
    return ext;
  }

  function getFileTypeIcon(ext: string): string {
    const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg'];
    const videoExts = ['mp4', 'mov', 'avi', 'mkv', 'webm'];
    const audioExts = ['mp3', 'wav', 'flac', 'aac', 'm4a'];
    const docExts = ['pdf', 'doc', 'docx', 'txt', 'rtf', 'md'];

    if (imageExts.includes(ext)) return '🖼️';
    if (videoExts.includes(ext)) return '🎬';
    if (audioExts.includes(ext)) return '🎵';
    if (docExts.includes(ext)) return '📄';
    return '📁';
  }
</script>

<div class="groups-list">
  <div class="list-header">
    <span class="header-title">Duplicate Groups</span>
    <span class="header-count">{groups.length}</span>
  </div>

  <div class="list-content">
    {#each groups as group (group.id)}
      <button
        class="group-item"
        class:selected={selectedGroupId === group.id}
        onclick={() => onSelect(group)}
      >
        <span class="group-icon">{getFileTypeIcon(getFileExtension(group))}</span>
        <div class="group-info">
          <div class="group-size">{formatBytes(group.file_size)}</div>
          <div class="group-meta">
            <span class="file-count">{group.files.length} files</span>
            <span class="wasted">{formatBytes(group.wasted_space)} wasted</span>
          </div>
        </div>
      </button>
    {/each}

    {#if groups.length === 0}
      <div class="empty-state">
        No duplicate groups found
      </div>
    {/if}
  </div>
</div>

<style>
  .groups-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--surface);
  }

  .list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    font-weight: 600;
  }

  .header-count {
    background: var(--primary);
    color: white;
    padding: 0.125rem 0.5rem;
    border-radius: 10px;
    font-size: 0.8rem;
  }

  .list-content {
    flex: 1;
    overflow-y: auto;
  }

  .group-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.75rem 1rem;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s;
  }

  .group-item:hover {
    background: var(--background);
  }

  .group-item.selected {
    background: var(--primary);
    color: white;
  }

  .group-item.selected .group-meta {
    color: rgba(255, 255, 255, 0.8);
  }

  .group-icon {
    font-size: 1.5rem;
  }

  .group-info {
    flex: 1;
    min-width: 0;
  }

  .group-size {
    font-weight: 500;
  }

  .group-meta {
    display: flex;
    gap: 0.75rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .wasted {
    color: var(--warning);
  }

  .group-item.selected .wasted {
    color: rgba(255, 255, 255, 0.9);
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary);
  }
</style>
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

#### Manual Verification
- [ ] Groups list renders correctly
- [ ] Selection works

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.
---

## Phase 5.3: Create File Details Panel Component

### Overview
Create the detail panel that shows files within a selected duplicate group.

### Changes Required

#### 5.3.1 Create File Details Panel

**File**: `src/lib/components/FileDetailsPanel.svelte`

```svelte
<script lang="ts">
  import type { DuplicateGroup, DuplicateFile } from '$lib/types';

  interface Props {
    group: DuplicateGroup | null;
    selectedFiles: Set<string>;
    onToggleFile: (path: string) => void;
    onSelectAllExceptOriginal: () => void;
  }

  let { group, selectedFiles, onToggleFile, onSelectAllExceptOriginal }: Props = $props();

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  }

  function getFileName(path: string): string {
    return path.split('/').pop() || path;
  }

  /**
   * Truncate path with middle ellipsis for better readability.
   * Shows beginning and end of path with "..." in the middle.
   * Example: "/Users/john/Documents/.../backups/photos" instead of "...uments/backups/photos"
   */
  function getDirectory(path: string, maxLength: number = 50): string {
    const parts = path.split('/');
    parts.pop(); // Remove filename
    const dir = parts.join('/');

    if (dir.length <= maxLength) {
      return dir;
    }

    // Middle ellipsis truncation: show start and end
    const ellipsis = '/...';
    const availableLength = maxLength - ellipsis.length;
    const startLength = Math.ceil(availableLength * 0.4); // 40% for start
    const endLength = Math.floor(availableLength * 0.6);  // 60% for end (more useful info)

    const start = dir.slice(0, startLength);
    const end = dir.slice(-endLength);

    return `${start}${ellipsis}${end}`;
  }
</script>

<div class="details-panel">
  {#if group}
    <div class="panel-header">
      <div class="header-info">
        <h2>{group.files.length} Files</h2>
        <span class="header-meta">
          {formatBytes(group.file_size)} each • {formatBytes(group.wasted_space)} wasted
        </span>
      </div>
      <button class="action-button" onclick={onSelectAllExceptOriginal}>
        Select All Except Original
      </button>
    </div>

    <div class="files-list">
      {#each group.files as file (file.path)}
        <div class="file-item" class:original={file.is_original}>
          <label class="file-checkbox">
            <input
              type="checkbox"
              checked={selectedFiles.has(file.path)}
              disabled={file.is_original}
              onchange={() => onToggleFile(file.path)}
            />
          </label>

          <div class="file-info">
            <div class="file-name">
              {#if file.is_original}
                <span class="original-badge">Original</span>
              {/if}
              {getFileName(file.path)}
            </div>
            <div class="file-path" title={file.path}>
              {getDirectory(file.path)}
              <span class="full-path-tooltip">{file.path}</span>
            </div>
            <div class="file-dates">
              <span class="date-label">Created:</span>
              <span class="date-value">{formatDate(file.created_at)}</span>
              <span class="date-separator">|</span>
              <span class="date-label">Modified:</span>
              <span class="date-value">{formatDate(file.modified_at)}</span>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-state">
      <p>Select a duplicate group to view files</p>
    </div>
  {/if}
</div>

<style>
  .details-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--background);
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  .header-info h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  .header-meta {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .action-button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    background: var(--primary);
    color: white;
    cursor: pointer;
    font-size: 0.875rem;
  }

  .action-button:hover {
    opacity: 0.9;
  }

  .files-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .file-item {
    display: flex;
    gap: 0.75rem;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    margin-bottom: 0.5rem;
    background: var(--surface);
  }

  .file-item.original {
    border-color: var(--success);
    background: var(--success-bg);
  }

  .file-checkbox {
    display: flex;
    align-items: flex-start;
    padding-top: 0.25rem;
  }

  .file-checkbox input {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }

  .file-checkbox input:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .file-info {
    flex: 1;
    min-width: 0;
  }

  .file-name {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 500;
    word-break: break-all;
  }

  .original-badge {
    font-size: 0.7rem;
    padding: 0.1rem 0.4rem;
    background: var(--success);
    color: white;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .file-path {
    font-size: 0.8rem;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    margin-top: 0.25rem;
    word-break: break-all;
    position: relative;
    cursor: help;
  }

  /* Full path tooltip on hover */
  .full-path-tooltip {
    display: none;
    position: absolute;
    left: 0;
    top: 100%;
    margin-top: 4px;
    padding: 0.5rem 0.75rem;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 4px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    font-size: 0.75rem;
    white-space: nowrap;
    max-width: 500px;
    overflow-x: auto;
    z-index: 100;
  }

  .file-path:hover .full-path-tooltip {
    display: block;
  }

  .file-dates {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin-top: 0.5rem;
    flex-wrap: wrap;
  }

  .file-dates .date-label {
    color: var(--text-secondary);
    opacity: 0.8;
  }

  .file-dates .date-value {
    color: var(--text);
  }

  .file-dates .date-separator {
    color: var(--border);
    margin: 0 0.25rem;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
</style>
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

#### Manual Verification
- [ ] File details show correctly
- [ ] Checkboxes work
- [ ] Original file is highlighted and not selectable
- [ ] Long paths are truncated with middle ellipsis (shows beginning and end)
- [ ] Hovering over truncated path shows full path tooltip
- [ ] Both creation date AND modified date are displayed for each file
- [ ] Dates are clearly labeled and visually distinct

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.
---

## Phase 5.4: Create TypeScript Types

### Overview
Create shared TypeScript types for the frontend.

### Changes Required

#### 5.4.1 Create Types File

**File**: `src/lib/types.ts`

```typescript
// Shared TypeScript types for DupliFind

export interface DuplicateFile {
  path: string;
  size: number;
  created_at: number;
  modified_at: number;
  is_original: boolean;
}

export interface DuplicateGroup {
  id: number;
  hash: string;
  file_size: number;
  files: DuplicateFile[];
  wasted_space: number;
}

export interface DetectionResult {
  groups: DuplicateGroup[];
  duplicate_count: number;
  total_wasted_space: number;
  unique_files: number;
  stats: DetectionStats;
}

export interface DetectionStats {
  size_groups: number;
  size_candidates: number;
  partial_hashes: number;
  full_hashes: number;
  size_grouping_ms: number;
  partial_hashing_ms: number;
  full_hashing_ms: number;
}

export interface ScanProgress {
  total_files: number;
  processed_files: number;
  total_bytes: number;
  current_path: string | null;
  skipped_files: number;
  estimated_total: number | null;
}

export interface ScanComplete {
  session_id: number;
  total_files: number;
  total_bytes: number;
  duplicate_groups: number;
  duplicate_files: number;
  wasted_space: number;
  duration_ms: number;
}

export interface Setting {
  key: string;
  value: string;
}

export interface ProtectedFolder {
  id: number;
  path: string;
  added_at: number;
}

export type FileType = 'images' | 'videos' | 'documents' | 'audio' | 'other' | 'all';

export interface FilterState {
  fileType: FileType;
  minSize: number | null;
  maxSize: number | null;
  searchQuery: string;
}
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.
---

## Phase 5.5: Create Main Results View

### Overview
Create the main results view that combines all components.

### Changes Required

#### 5.5.1 Create Results View

**File**: `src/lib/components/ResultsView.svelte`

```svelte
<script lang="ts">
  import MasterDetailLayout from './MasterDetailLayout.svelte';
  import DuplicateGroupsList from './DuplicateGroupsList.svelte';
  import FileDetailsPanel from './FileDetailsPanel.svelte';
  import type { DuplicateGroup, DetectionResult } from '$lib/types';

  interface Props {
    result: DetectionResult;
    onDeleteSelected: (files: string[]) => void;
  }

  let { result, onDeleteSelected }: Props = $props();

  let selectedGroup = $state<DuplicateGroup | null>(null);
  let selectedFiles = $state<Set<string>>(new Set());

  function handleGroupSelect(group: DuplicateGroup) {
    selectedGroup = group;
    selectedFiles = new Set();
  }

  function handleToggleFile(path: string) {
    const newSet = new Set(selectedFiles);
    if (newSet.has(path)) {
      newSet.delete(path);
    } else {
      newSet.add(path);
    }
    selectedFiles = newSet;
  }

  function handleSelectAllExceptOriginal() {
    if (!selectedGroup) return;
    const newSet = new Set<string>();
    for (const file of selectedGroup.files) {
      if (!file.is_original) {
        newSet.add(file.path);
      }
    }
    selectedFiles = newSet;
  }

  function handleDeleteSelected() {
    if (selectedFiles.size > 0) {
      onDeleteSelected(Array.from(selectedFiles));
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  let selectedSize = $derived(
    selectedGroup
      ? Array.from(selectedFiles).reduce((sum, path) => {
          const file = selectedGroup.files.find((f) => f.path === path);
          return sum + (file?.size || 0);
        }, 0)
      : 0
  );
</script>

<div class="results-view">
  <div class="results-header">
    <div class="header-stats">
      <div class="stat">
        <span class="stat-value">{result.groups.length}</span>
        <span class="stat-label">Groups</span>
      </div>
      <div class="stat">
        <span class="stat-value">{result.duplicate_count}</span>
        <span class="stat-label">Duplicates</span>
      </div>
      <div class="stat warning">
        <span class="stat-value">{formatBytes(result.total_wasted_space)}</span>
        <span class="stat-label">Wasted</span>
      </div>
    </div>

    {#if selectedFiles.size > 0}
      <div class="selection-info">
        <span>{selectedFiles.size} files selected ({formatBytes(selectedSize)})</span>
        <button class="delete-button" onclick={handleDeleteSelected}>
          Delete Selected
        </button>
      </div>
    {/if}
  </div>

  <div class="results-content">
    <MasterDetailLayout>
      <div slot="master">
        <DuplicateGroupsList
          groups={result.groups}
          selectedGroupId={selectedGroup?.id ?? null}
          onSelect={handleGroupSelect}
        />
      </div>
      <div slot="detail">
        <FileDetailsPanel
          group={selectedGroup}
          {selectedFiles}
          onToggleFile={handleToggleFile}
          onSelectAllExceptOriginal={handleSelectAllExceptOriginal}
        />
      </div>
    </MasterDetailLayout>
  </div>
</div>

<style>
  .results-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .results-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .header-stats {
    display: flex;
    gap: 2rem;
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 600;
  }

  .stat-label {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .stat.warning .stat-value {
    color: var(--warning);
  }

  .selection-info {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: white;
    border-radius: 6px;
  }

  .delete-button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    background: var(--error);
    color: white;
    cursor: pointer;
    font-weight: 500;
  }

  .delete-button:hover {
    opacity: 0.9;
  }

  .results-content {
    flex: 1;
    overflow: hidden;
  }
</style>
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

#### Manual Verification
- [ ] Results view renders complete layout
- [ ] Group selection works
- [ ] File selection works

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.
---

## Phase 5.6: Update Main App with Results View

### Overview
Update the main App.svelte to use the results view.

### Changes Required

#### 5.6.1 Update App.svelte

**File**: `src/App.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import ResultsView from './lib/components/ResultsView.svelte';
  import type { DetectionResult, ScanProgress, ScanComplete } from '$lib/types';

  type AppView = 'home' | 'scanning' | 'results';

  let currentView = $state<AppView>('home');
  let isScanning = $state(false);
  let progress = $state<ScanProgress | null>(null);
  let scanResult = $state<ScanComplete | null>(null);
  let detectionResult = $state<DetectionResult | null>(null);
  let error = $state<string | null>(null);

  let unlisteners: UnlistenFn[] = [];

  onMount(async () => {
    unlisteners.push(
      await listen<ScanProgress>('scan-progress', (e) => {
        progress = e.payload;
      }),
      await listen<ScanComplete>('scan-complete', (e) => {
        scanResult = e.payload;
        isScanning = false;
        currentView = 'results';
      }),
      await listen<DetectionResult>('scan-results', (e) => {
        detectionResult = e.payload;
      }),
      await listen<{ error: string }>('scan-error', (e) => {
        error = e.payload.error;
        isScanning = false;
        currentView = 'home';
      })
    );

    // Check for existing results
    try {
      const existing = await invoke<DetectionResult | null>('get_scan_results');
      if (existing && existing.groups.length > 0) {
        detectionResult = existing;
      }
    } catch (e) {
      console.error('Failed to load existing results:', e);
    }
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
  });

  async function startScan() {
    error = null;
    isScanning = true;
    currentView = 'scanning';
    detectionResult = null;

    try {
      await invoke('start_scan', {
        request: {
          paths: ['/Users'],
          parallelism: 'normal',
        },
      });
    } catch (e) {
      error = String(e);
      isScanning = false;
      currentView = 'home';
    }
  }

  async function cancelScan() {
    try {
      await invoke('cancel_scan');
      isScanning = false;
      currentView = 'home';
    } catch (e) {
      error = String(e);
    }
  }

  function handleDeleteSelected(files: string[]) {
    // TODO: Implement in deletion phase
    console.log('Delete selected:', files);
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }
</script>

<main class="app">
  <header class="app-header">
    <h1>DupliFind</h1>
    <nav>
      {#if currentView === 'results' || detectionResult}
        <button class="nav-button" onclick={() => (currentView = 'home')}>
          New Scan
        </button>
        {#if detectionResult}
          <button
            class="nav-button"
            class:active={currentView === 'results'}
            onclick={() => (currentView = 'results')}
          >
            Results ({detectionResult.groups.length})
          </button>
        {/if}
      {/if}
    </nav>
  </header>

  <div class="app-content">
    {#if currentView === 'home'}
      <div class="home-view">
        <div class="home-content">
          <h2>Find Duplicate Files</h2>
          <p>Scan your drives to find and remove duplicate files.</p>

          {#if error}
            <div class="error-banner">{error}</div>
          {/if}

          <button class="scan-button" onclick={startScan}>
            Start Scan
          </button>

          {#if detectionResult}
            <button class="results-link" onclick={() => (currentView = 'results')}>
              View Previous Results ({detectionResult.groups.length} groups)
            </button>
          {/if}
        </div>
      </div>
    {:else if currentView === 'scanning'}
      <div class="scanning-view">
        <div class="scanning-content">
          <div class="spinner"></div>
          <h2>Scanning...</h2>
          {#if progress}
            <p class="progress-info">
              {progress.total_files.toLocaleString()} files •
              {formatBytes(progress.total_bytes)}
            </p>
            {#if progress.current_path}
              <p class="current-path">{progress.current_path}</p>
            {/if}
          {/if}
          <button class="cancel-button" onclick={cancelScan}>Cancel</button>
        </div>
      </div>
    {:else if currentView === 'results' && detectionResult}
      <ResultsView result={detectionResult} onDeleteSelected={handleDeleteSelected} />
    {/if}
  </div>
</main>

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .app-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .app-header h1 {
    font-size: 1.25rem;
    margin: 0;
  }

  .app-header nav {
    display: flex;
    gap: 0.5rem;
  }

  .nav-button {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }

  .nav-button:hover {
    background: var(--background);
  }

  .nav-button.active {
    background: var(--primary);
    color: white;
    border-color: var(--primary);
  }

  .app-content {
    flex: 1;
    overflow: hidden;
  }

  .home-view,
  .scanning-view {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
  }

  .home-content,
  .scanning-content {
    text-align: center;
    max-width: 400px;
  }

  .home-content h2,
  .scanning-content h2 {
    margin-bottom: 0.5rem;
  }

  .home-content p {
    color: var(--text-secondary);
    margin-bottom: 2rem;
  }

  .error-banner {
    padding: 0.75rem;
    background: var(--error-bg);
    color: var(--error);
    border-radius: 4px;
    margin-bottom: 1rem;
  }

  .scan-button {
    width: 100%;
    padding: 1rem;
    font-size: 1.1rem;
    border: none;
    border-radius: 8px;
    background: var(--primary);
    color: white;
    cursor: pointer;
  }

  .scan-button:hover {
    opacity: 0.9;
  }

  .results-link {
    display: block;
    margin-top: 1rem;
    padding: 0.5rem;
    border: none;
    background: transparent;
    color: var(--primary);
    cursor: pointer;
    text-decoration: underline;
  }

  .spinner {
    width: 48px;
    height: 48px;
    border: 4px solid var(--border);
    border-top-color: var(--primary);
    border-radius: 50%;
    margin: 0 auto 1rem;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .progress-info {
    color: var(--text-secondary);
    margin-bottom: 0.5rem;
  }

  .current-path {
    font-size: 0.8rem;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    max-width: 300px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin: 0 auto 1rem;
  }

  .cancel-button {
    padding: 0.5rem 1.5rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }

  .cancel-button:hover {
    background: var(--error);
    color: white;
    border-color: var(--error);
  }
</style>
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

#### Manual Verification
- [ ] `npm run tauri dev` works
- [ ] Complete scan flow works
- [ ] Results are displayed correctly

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.
---

## Phases 5.7-5.8: Tests

### Phase 5.7: Add Component Tests

Create tests for the UI components using vitest.

### Phase 5.8: Add Integration Tests

Create integration tests for the full results flow.

(See testing patterns from Phase 1.8)

---

## End of File 05

After completing all phases, you should have:
- Master-detail layout with resizable panels
- Duplicate groups list with selection
- File details panel with checkboxes
- TypeScript types for frontend
- Complete results view
- Updated main app with scan flow

**Next**: Proceed to [06-selection-deletion.md](./06-selection-deletion.md)
