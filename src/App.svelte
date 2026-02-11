<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import ResultsView from './lib/components/ResultsView.svelte';
  import FolderPicker from './lib/components/FolderPicker.svelte';
  import type { DetectionResult, ScanProgress, ScanComplete, ScanPhaseEvent, ScanErrorEvent } from '$lib/types';

  type AppView = 'home' | 'scanning' | 'results';

  let currentView = $state<AppView>('home');
  let isScanning = $state(false);
  let progress = $state<ScanProgress | null>(null);
  let scanResult = $state<ScanComplete | null>(null);
  let detectionResult = $state<DetectionResult | null>(null);
  let error = $state<string | null>(null);
  let phase = $state<ScanPhaseEvent['phase'] | 'idle'>('idle');
  let selectedPaths = $state<string[]>([]);

  let unlisteners: UnlistenFn[] = [];

  onMount(async () => {
    // Load last scan paths
    await loadLastScanPaths();

    unlisteners.push(
      await listen<ScanProgress>('scan-progress', (e) => {
        progress = e.payload;
      }),
      await listen<ScanComplete>('scan-complete', (e) => {
        scanResult = e.payload;
        isScanning = false;
        phase = 'complete';
        currentView = 'results';
      }),
      await listen<DetectionResult>('scan-results', (e) => {
        detectionResult = e.payload;
      }),
      await listen<ScanPhaseEvent>('scan-phase', (e) => {
        phase = e.payload.phase;
      }),
      await listen<ScanErrorEvent>('scan-error', (e) => {
        error = e.payload.error;
        isScanning = false;
        phase = 'idle';
        currentView = 'home';
      }),
    );

    // Check for existing results
    try {
      const existing = await invoke<DetectionResult | null>('get_scan_results');
      if (existing && existing.groups.length > 0) {
        detectionResult = existing;
      }
    } catch (e) {
      console.error('Failed to load existing results:', e);
      // Don't show error to user for background load failure
    }
  });

  onDestroy(() => {
    unlisteners.forEach((u) => u());
  });

  async function loadLastScanPaths() {
    try {
      const value = await invoke<string | null>('get_setting', { key: 'last_scan_paths' });
      if (value) {
        selectedPaths = JSON.parse(value);
      }
    } catch (e) {
      console.error('Failed to load last scan paths:', e);
      // Don't show error to user for background load failure
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
      // Non-critical failure, don't interrupt user flow
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
    isScanning = true;
    currentView = 'scanning';
    detectionResult = null;
    scanResult = null;
    progress = null;
    phase = 'collecting';

    try {
      await saveLastScanPaths();
      await invoke('start_scan', {
        request: {
          paths: selectedPaths,
          parallelism: 'normal',
        },
      });
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
      isScanning = false;
      phase = 'idle';
      currentView = 'home';
    }
  }

  async function cancelScan() {
    try {
      await invoke('cancel_scan');
      isScanning = false;
      phase = 'idle';
      currentView = 'home';
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    }
  }

  function handleDeleteSelected(files: string[]) {
    // TODO: Implement in deletion phase (Phase 6)
    console.log('Delete selected:', files);
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  function getPhaseLabel(currentPhase: typeof phase): string {
    switch (currentPhase) {
      case 'collecting':
        return 'Scanning files...';
      case 'partial_hashing':
        return 'Computing partial hashes...';
      case 'full_hashing':
        return 'Computing full hashes...';
      case 'storing':
        return 'Analyzing duplicates...';
      case 'complete':
        return 'Complete';
      default:
        return 'Scanning...';
    }
  }

  function handleNewScan() {
    currentView = 'home';
    error = null;
    progress = null;
    phase = 'idle';
  }
</script>

<main class="app">
  <header class="app-header">
    <h1>DupliFind</h1>
    <nav>
      {#if currentView === 'results' || detectionResult}
        <button class="nav-button" onclick={handleNewScan}>
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
            <div class="error-banner" role="alert">{error}</div>
          {/if}

          <FolderPicker {selectedPaths} onPathsChange={handlePathsChange} />

          <button
            class="scan-button"
            onclick={startScan}
            disabled={selectedPaths.length === 0 || isScanning}
          >
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
          <h2>{getPhaseLabel(phase)}</h2>
          {#if progress}
            <p class="progress-info">
              {progress.total_files.toLocaleString()} files &bull;
              {formatBytes(progress.total_bytes)}
              {#if scanResult}
                &bull; completed in {(scanResult.duration_ms / 1000).toFixed(1)}s
              {/if}
            </p>
            {#if progress.current_path}
              <p class="current-path" title={progress.current_path}>{progress.current_path}</p>
            {/if}
          {/if}
          <button class="cancel-button" onclick={cancelScan}>Cancel</button>
        </div>
      </div>
    {:else if currentView === 'results'}
      {#if detectionResult}
        <ResultsView result={detectionResult} onDeleteSelected={handleDeleteSelected} />
      {:else}
        <div class="empty-results">
          <p>No results available</p>
          <button class="nav-button" onclick={handleNewScan}>Start New Scan</button>
        </div>
      {/if}
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
    font-size: 0.875rem;
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
    padding: 1rem;
  }

  .home-content,
  .scanning-content {
    text-align: center;
    max-width: 500px;
    width: 100%;
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
    text-align: left;
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
    margin-top: 1rem;
  }

  .scan-button:hover:not(:disabled) {
    opacity: 0.9;
  }

  .scan-button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
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
    font-size: 0.9rem;
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
    max-width: 100%;
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
    font-size: 0.9rem;
  }

  .cancel-button:hover {
    background: var(--error);
    color: white;
    border-color: var(--error);
  }

  .empty-results {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    gap: 1rem;
  }

  .empty-results p {
    color: var(--text-secondary);
    font-size: 1.1rem;
  }

  @media (max-width: 768px) {
    .app-header h1 {
      font-size: 1rem;
    }

    .nav-button {
      padding: 0.4rem 0.75rem;
      font-size: 0.8rem;
    }

    .home-content,
    .scanning-content {
      max-width: 100%;
      padding: 0.5rem;
    }

    .scan-button {
      font-size: 1rem;
      padding: 0.875rem;
    }

    .current-path {
      font-size: 0.7rem;
    }
  }

  @media (max-width: 480px) {
    .app-header {
      flex-direction: column;
      gap: 0.5rem;
      align-items: flex-start;
    }

    .app-header nav {
      width: 100%;
      justify-content: flex-end;
    }

    .progress-info {
      font-size: 0.85rem;
    }
  }
</style>
