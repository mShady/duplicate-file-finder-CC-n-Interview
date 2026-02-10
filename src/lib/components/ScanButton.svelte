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

  interface ScanStats {
    total_files: number;
    total_bytes: number;
    directories: number;
    symlinks_skipped: number;
    errors: number;
    duration_ms: number;
  }

  let isScanning = $state(false);
  let progress = $state<ScanProgress | null>(null);
  let scanResult = $state<{ session_id: number; stats: ScanStats } | null>(null);
  let error = $state<string | null>(null);
  let selectedPaths = $state<string[]>([]);

  let unlistenProgress: UnlistenFn | null = null;
  let unlistenComplete: UnlistenFn | null = null;

  onMount(async () => {
    // Load last scan paths from settings
    await loadLastScanPaths();

    // Listen for progress events
    unlistenProgress = await listen<ScanProgress>('scan-progress', (event) => {
      progress = event.payload;
    });

    // Listen for completion events
    unlistenComplete = await listen<{ session_id: number; stats: ScanStats }>(
      'scan-complete',
      (event) => {
        scanResult = event.payload;
        isScanning = false;
        progress = null;
      },
    );
  });

  onDestroy(() => {
    unlistenProgress?.();
    unlistenComplete?.();
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
    }
  }

  async function cancelScan() {
    try {
      await invoke('cancel_scan');
      isScanning = false;
      progress = null;
    } catch (e) {
      error = String(e);
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
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

  {#if isScanning && progress}
    <div class="progress-container">
      <div class="progress-header">Scanning...</div>
      <div class="progress-stats">
        <div class="stat">
          <span class="label">Files:</span>
          <span class="value">{progress.total_files.toLocaleString()}</span>
        </div>
        <div class="stat">
          <span class="label">Size:</span>
          <span class="value">{formatBytes(progress.total_bytes)}</span>
        </div>
        <div class="stat">
          <span class="label">Skipped:</span>
          <span class="value">{progress.skipped_files}</span>
        </div>
      </div>
      {#if progress.current_path}
        <div class="current-path" title={progress.current_path}>
          {progress.current_path}
        </div>
      {/if}
    </div>
  {/if}

  {#if scanResult}
    <div class="result-container">
      <div class="result-header">Scan Complete</div>
      <div class="result-stats">
        <div class="stat">
          <span class="label">Total Files:</span>
          <span class="value">{scanResult.stats.total_files.toLocaleString()}</span>
        </div>
        <div class="stat">
          <span class="label">Total Size:</span>
          <span class="value">{formatBytes(scanResult.stats.total_bytes)}</span>
        </div>
        <div class="stat">
          <span class="label">Directories:</span>
          <span class="value">{scanResult.stats.directories.toLocaleString()}</span>
        </div>
        <div class="stat">
          <span class="label">Duration:</span>
          <span class="value">{formatDuration(scanResult.stats.duration_ms)}</span>
        </div>
        <div class="stat">
          <span class="label">Symlinks Skipped:</span>
          <span class="value">{scanResult.stats.symlinks_skipped}</span>
        </div>
        <div class="stat">
          <span class="label">Errors:</span>
          <span class="value">{scanResult.stats.errors}</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .scan-container {
    width: 100%;
    max-width: 500px;
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
  }

  .progress-header,
  .result-header {
    font-weight: 600;
    margin-bottom: 0.75rem;
    font-size: 1.1rem;
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
    font-size: 0.8rem;
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
</style>
