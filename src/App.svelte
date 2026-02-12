<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';
  import type { UnlistenFn } from '@tauri-apps/api/event';
  import AppHeader from './lib/components/AppHeader.svelte';
  import HomeView from './lib/components/HomeView.svelte';
  import ScanningView from './lib/components/ScanningView.svelte';
  import ResultsView from './lib/components/ResultsView.svelte';
  import DeleteConfirmDialog from './lib/components/DeleteConfirmDialog.svelte';
  import DeleteSummaryDialog from './lib/components/DeleteSummaryDialog.svelte';
  import HistoryDialog from './lib/components/HistoryDialog.svelte';
  import type { DetectionResult, ScanProgress, ScanComplete, ScanPhaseEvent, ScanErrorEvent, DeleteFilesResponse, BatchDeletionResult } from '$lib/types';
  import {
    buildDeletionRequests,
    buildKeptPathsAndGroupIds,
    updateResultsAfterDeletion as computeUpdatedResults,
    computePendingDeletionSize,
    isDeletingAllInGroup,
  } from '$lib/utils/deletionOrchestrator';

  type AppView = 'home' | 'scanning' | 'results';

  let currentView = $state<AppView>('home');
  let isScanning = $state(false);
  let progress = $state<ScanProgress | null>(null);
  let scanResult = $state<ScanComplete | null>(null);
  let detectionResult = $state<DetectionResult | null>(null);
  let error = $state<string | null>(null);
  let phase = $state<ScanPhaseEvent['phase'] | 'idle'>('idle');
  let selectedPaths = $state<string[]>([]);

  // Deletion dialog state
  let showDeleteConfirm = $state(false);
  let showDeleteSummary = $state(false);
  let pendingDeletionFiles = $state<string[]>([]);
  let deletionResult = $state<BatchDeletionResult | null>(null);
  let showDeletionHistory = $state(false);

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
    pendingDeletionFiles = files;
    showDeleteConfirm = true;
  }

  let deletingAllInGroup = $derived.by(() =>
    detectionResult ? isDeletingAllInGroup(detectionResult, pendingDeletionFiles) : false
  );

  let pendingDeletionSize = $derived.by(() =>
    detectionResult ? computePendingDeletionSize(detectionResult, pendingDeletionFiles) : 0
  );

  async function handleConfirmDelete() {
    if (!detectionResult) return;
    showDeleteConfirm = false;
    error = null;

    const requests = buildDeletionRequests(detectionResult, pendingDeletionFiles);
    const { keptPaths, groupIds } = buildKeptPathsAndGroupIds(detectionResult, pendingDeletionFiles);

    try {
      const response = await invoke<DeleteFilesResponse>('delete_files', {
        request: { files: requests, kept_paths: keptPaths, group_ids: groupIds },
      });

      deletionResult = response.result;
      showDeleteSummary = true;

      if (response.result.successful.length > 0) {
        const deletedPaths = new Set(response.result.successful.map(r => r.path));
        detectionResult = computeUpdatedResults(detectionResult, deletedPaths);
      }
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      pendingDeletionFiles = [];
    }
  }

  function handleCancelDelete() {
    showDeleteConfirm = false;
    pendingDeletionFiles = [];
  }

  function handleCloseSummary() {
    showDeleteSummary = false;
    deletionResult = null;
  }

  function handleNewScan() {
    currentView = 'home';
    error = null;
    progress = null;
    phase = 'idle';
  }
</script>

<main class="app">
  <AppHeader
    {currentView}
    {detectionResult}
    onNewScan={handleNewScan}
    onViewResults={() => (currentView = 'results')}
    onToggleHistory={() => (showDeletionHistory = !showDeletionHistory)}
  />

  <div class="app-content">
    {#if currentView === 'home'}
      <HomeView
        {selectedPaths}
        {isScanning}
        {error}
        {detectionResult}
        onPathsChange={handlePathsChange}
        onStartScan={startScan}
        onViewResults={() => (currentView = 'results')}
      />
    {:else if currentView === 'scanning'}
      <ScanningView {phase} {progress} {scanResult} onCancel={cancelScan} />
    {:else if currentView === 'results'}
      {#if error}
        <div class="error-banner" role="alert">
          {error}
          <button class="error-dismiss" onclick={() => (error = null)} aria-label="Dismiss error">&times;</button>
        </div>
      {/if}
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

{#if showDeletionHistory}
  <HistoryDialog onClose={() => (showDeletionHistory = false)} />
{/if}

{#if showDeleteConfirm}
  <DeleteConfirmDialog
    fileCount={pendingDeletionFiles.length}
    totalSize={pendingDeletionSize}
    sampleFiles={pendingDeletionFiles}
    allInGroup={deletingAllInGroup}
    onConfirm={handleConfirmDelete}
    onCancel={handleCancelDelete}
  />
{/if}

{#if showDeleteSummary && deletionResult}
  <DeleteSummaryDialog
    result={deletionResult}
    onClose={handleCloseSummary}
  />
{/if}

<style>
  .app {
    display: flex;
    flex-direction: column;
    height: 100vh;
  }

  .app-content {
    flex: 1;
    overflow: hidden;
  }

  .error-banner {
    padding: 0.75rem;
    background: var(--error-bg);
    color: var(--error);
    border-radius: 4px;
    margin-bottom: 1rem;
    text-align: left;
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .error-dismiss {
    background: none;
    border: none;
    color: var(--error);
    font-size: 1.25rem;
    cursor: pointer;
    padding: 0 0.25rem;
    line-height: 1;
    flex-shrink: 0;
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
</style>
