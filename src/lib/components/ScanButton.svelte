<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import FolderPicker from './FolderPicker.svelte';

  interface ScanProgress {
    total_files: number;
    processed_files: number;
    total_bytes: number;
    current_path: string | null;
    skipped_files: number;
  }

  interface ScanComplete {
    session_id: number;
    total_files: number;
    total_bytes: number;
    duplicate_groups: number;
    duplicate_files: number;
    wasted_space: number;
    duration_ms: number;
  }

  interface DuplicateFile {
    path: string;
    size: number;
    created_at: number;
    modified_at: number;
    is_original: boolean;
  }

  interface DuplicateGroup {
    id: number;
    hash: string;
    file_size: number;
    files: DuplicateFile[];
    wasted_space: number;
  }

  interface DetectionResult {
    groups: DuplicateGroup[];
    duplicate_count: number;
    total_wasted_space: number;
    unique_files: number;
  }

  let isScanning = $state(false);
  let progress = $state<ScanProgress | null>(null);
  let scanResult = $state<ScanComplete | null>(null);
  let detectionResult = $state<DetectionResult | null>(null);
  let error = $state<string | null>(null);
  let phase = $state<string>('idle');
  let selectedPaths = $state<string[]>([]);

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;
  let unlistenResults: UnlistenFn | null = null;
  let unlistenPhase: UnlistenFn | null = null;
  let unlistenError: UnlistenFn | null = null;

  onMount(async () => {
    // Load last scan paths from settings
    await loadLastScanPaths();

    unlistenProgress = await listen<ScanProgress>('scan-progress', (event) => {
      progress = event.payload;
    });

    unlistenComplete = await listen<ScanComplete>('scan-complete', (event) => {
      scanResult = event.payload;
      isScanning = false;
      progress = null;
      phase = 'complete';
    });

    unlistenResults = await listen<DetectionResult>('scan-results', (event) => {
      detectionResult = event.payload;
    });

    unlistenPhase = await listen<{ phase: string; message: string }>('scan-phase', (event) => {
      phase = event.payload.phase;
    });

    unlistenError = await listen<{ session_id: number; error: string }>('scan-error', (event) => {
      error = event.payload.error;
      isScanning = false;
      phase = 'error';
    });
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenComplete?.();
    unlistenResults?.();
    unlistenPhase?.();
    unlistenError?.();
  });

  async function loadLastScanPaths() {
    try {
      const value = await invoke<string | null>('get_setting', { key: 'last_scan_paths' });
      if (value) {
        selectedPaths = JSON.parse(value);
      }
    } catch (e) {
      console.error('Failed to load last scan paths:', e);
    }
  }

  async function saveLastScanPaths() {
    try {
      await invoke('set_setting', {
        key: 'last_scan_paths',
        value: JSON.stringify(selectedPaths),
      });
    } catch (e) {
      console.error('Failed to save scan paths:', e);
    }
  }

  function handlePathsChange(paths: string[]) {
    selectedPaths = paths;
  }

  async function startScan() {
    if (selectedPaths.length === 0) {
      error = 'Please select at least one folder to scan';
      return;
    }

    error = null;
    scanResult = null;
    detectionResult = null;
    phase = 'scanning';

    try {
      // Save paths for next time
      await saveLastScanPaths();

      isScanning = true;
      await invoke('start_scan', {
        request: {
          paths: selectedPaths,
          parallelism: 'normal',
        },
      });
    } catch (e) {
      error = String(e);
      isScanning = false;
      phase = 'error';
    }
  }

  async function cancelScan() {
    try {
      await invoke('cancel_scan');
      isScanning = false;
      progress = null;
      phase = 'cancelled';
    } catch (e) {
      error = String(e);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  function formatDuration(ms: number): string {
    if (ms < 1000) return `${ms}ms`;
    const seconds = Math.floor(ms / 1000);
    if (seconds < 60) return `${seconds}s`;
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    return `${minutes}m ${remainingSeconds}s`;
  }

  function truncatePath(path: string, maxLen: number = 50): string {
    if (path.length <= maxLen) return path;
    const start = path.substring(0, 20);
    const end = path.substring(path.length - 25);
    return `${start}...${end}`;
  }
</script>

<div class="scan-container">
  {#if !isScanning}
    <FolderPicker {selectedPaths} onPathsChange={handlePathsChange} />
  {/if}

  <div class="scan-controls">
    {#if isScanning}
      <button class="cancel-button" onclick={cancelScan}>Cancel Scan</button>
    {:else}
      <button class="scan-button" onclick={startScan} disabled={selectedPaths.length === 0}>
        Start Scan
      </button>
    {/if}
  </div>

  {#if error}
    <div class="error-message">
      Error: {error}
    </div>
  {/if}

  {#if isScanning}
    <div class="progress-container">
      <div class="progress-header">
        {#if phase === 'scanning'}
          Scanning files...
        {:else if phase === 'detecting'}
          Analyzing for duplicates...
        {:else}
          Processing...
        {/if}
      </div>
      {#if progress}
        <div class="progress-stats">
          <div class="stat">
            <span class="label">Files:</span>
            <span class="value">{progress.total_files.toLocaleString()}</span>
          </div>
          <div class="stat">
            <span class="label">Size:</span>
            <span class="value">{formatBytes(progress.total_bytes)}</span>
          </div>
        </div>
        {#if progress.current_path}
          <div class="current-path" title={progress.current_path}>
            {truncatePath(progress.current_path)}
          </div>
        {/if}
      {/if}
    </div>
  {/if}

  {#if scanResult && !isScanning}
    <div class="result-container">
      <div class="result-header">Scan Complete</div>
      <div class="result-stats">
        <div class="stat">
          <span class="label">Total Files:</span>
          <span class="value">{scanResult.total_files.toLocaleString()}</span>
        </div>
        <div class="stat">
          <span class="label">Total Size:</span>
          <span class="value">{formatBytes(scanResult.total_bytes)}</span>
        </div>
        <div class="stat">
          <span class="label">Duration:</span>
          <span class="value">{formatDuration(scanResult.duration_ms)}</span>
        </div>
        <div class="stat highlight">
          <span class="label">Duplicate Groups:</span>
          <span class="value">{scanResult.duplicate_groups}</span>
        </div>
        <div class="stat highlight">
          <span class="label">Duplicate Files:</span>
          <span class="value">{scanResult.duplicate_files}</span>
        </div>
        <div class="stat highlight warning">
          <span class="label">Wasted Space:</span>
          <span class="value">{formatBytes(scanResult.wasted_space)}</span>
        </div>
      </div>
    </div>

    {#if detectionResult && detectionResult.groups.length > 0}
      <div class="groups-container">
        <div class="groups-header">
          Duplicate Groups ({detectionResult.groups.length})
        </div>
        <div class="groups-list">
          {#each detectionResult.groups.slice(0, 10) as group}
            <div class="group-card">
              <div class="group-info">
                <span class="group-size">{formatBytes(group.file_size)} each</span>
                <span class="group-count">{group.files.length} files</span>
                <span class="group-wasted">Wasted: {formatBytes(group.wasted_space)}</span>
              </div>
              <div class="group-files">
                {#each group.files as file}
                  <div class="file-item" class:original={file.is_original}>
                    {#if file.is_original}
                      <span class="original-badge">Original</span>
                    {/if}
                    <span class="file-path" title={file.path}>{truncatePath(file.path, 60)}</span>
                  </div>
                {/each}
              </div>
            </div>
          {/each}
          {#if detectionResult.groups.length > 10}
            <div class="more-groups">
              +{detectionResult.groups.length - 10} more groups...
            </div>
          {/if}
        </div>
      </div>
    {/if}
  {/if}
</div>

<style>
  .scan-container {
    width: 100%;
    max-width: 700px;
    padding: 1rem;
  }

  .scan-controls {
    margin-bottom: 1rem;
  }

  .scan-button,
  .cancel-button {
    width: 100%;
    padding: 0.75rem 1rem;
    border: none;
    border-radius: 6px;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.2s;
  }

  .scan-button {
    background: var(--primary);
    color: white;
  }

  .cancel-button {
    background: var(--error);
    color: white;
  }

  .scan-button:hover,
  .cancel-button:hover {
    opacity: 0.9;
  }

  .scan-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-message {
    padding: 0.75rem;
    background: var(--error-bg);
    color: var(--error);
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .progress-container,
  .result-container {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
    margin-bottom: 1rem;
  }

  .progress-header,
  .result-header,
  .groups-header {
    font-weight: 600;
    margin-bottom: 0.75rem;
  }

  .progress-stats,
  .result-stats {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.5rem;
  }

  .stat {
    display: flex;
    justify-content: space-between;
    padding: 0.25rem 0;
  }

  .stat.highlight {
    background: var(--background);
    padding: 0.5rem;
    border-radius: 4px;
  }

  .stat.warning .value {
    color: var(--warning);
    font-weight: 600;
  }

  .label {
    color: var(--text-secondary);
  }

  .value {
    font-weight: 500;
  }

  .current-path {
    margin-top: 0.75rem;
    padding: 0.5rem;
    background: var(--background);
    border-radius: 4px;
    font-family: var(--font-mono);
    font-size: 0.75rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }

  .result-container {
    background: var(--success-bg);
  }

  .result-header {
    color: var(--success);
  }

  .groups-container {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
  }

  .groups-header {
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
  }

  .groups-list {
    max-height: 400px;
    overflow-y: auto;
  }

  .group-card {
    border: 1px solid var(--border);
    border-radius: 6px;
    margin-top: 0.75rem;
    overflow: hidden;
  }

  .group-info {
    display: flex;
    gap: 1rem;
    padding: 0.75rem;
    background: var(--background);
    font-size: 0.875rem;
  }

  .group-size {
    font-weight: 500;
  }

  .group-count {
    color: var(--text-secondary);
  }

  .group-wasted {
    color: var(--warning);
    margin-left: auto;
  }

  .group-files {
    padding: 0.5rem;
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    font-size: 0.8rem;
    font-family: var(--font-mono);
  }

  .file-item.original {
    background: var(--success-bg);
    border-radius: 4px;
  }

  .original-badge {
    font-size: 0.7rem;
    padding: 0.1rem 0.3rem;
    background: var(--success);
    color: white;
    border-radius: 3px;
    font-family: var(--font-sans);
  }

  .file-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .more-groups {
    text-align: center;
    padding: 1rem;
    color: var(--text-secondary);
    font-style: italic;
  }
</style>
