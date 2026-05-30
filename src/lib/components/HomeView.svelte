<script lang="ts">
  import type { DetectionResult } from '$lib/types';
  import FolderPicker from './FolderPicker.svelte';

  interface Props {
    selectedPaths: string[];
    isScanning: boolean;
    error: string | null;
    detectionResult: DetectionResult | null;
    onPathsChange: (paths: string[]) => void;
    onStartScan: () => void;
    onViewResults: () => void;
    onDismissError: () => void;
  }

  let {
    selectedPaths,
    isScanning,
    error,
    detectionResult,
    onPathsChange,
    onStartScan,
    onViewResults,
    onDismissError,
  }: Props = $props();
</script>

<div class="home-view">
  <div class="home-content">
    <h2>Find Duplicate Files</h2>
    <p>Scan your drives to find and remove duplicate files.</p>

    {#if error}
      <div class="error-banner" role="alert">
        <span class="error-message">{error}</span>
        <button class="error-dismiss" onclick={onDismissError} aria-label="Dismiss error"
          >&times;</button
        >
      </div>
    {/if}

    <FolderPicker {selectedPaths} {onPathsChange} />

    <button
      class="scan-button"
      onclick={onStartScan}
      disabled={selectedPaths.length === 0 || isScanning}
    >
      Start Scan
    </button>

    {#if detectionResult}
      <button class="results-link" onclick={onViewResults}>
        View Previous Results ({detectionResult.groups.length}
        {detectionResult.groups.length === 1 ? 'group' : 'groups'})
      </button>
    {/if}
  </div>
</div>

<style>
  .home-view {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 1rem;
  }

  .home-content {
    text-align: center;
    max-width: 500px;
    width: 100%;
  }

  .home-content h2 {
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
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
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

  @media (max-width: 768px) {
    .home-content {
      max-width: 100%;
      padding: 0.5rem;
    }

    .scan-button {
      font-size: 1rem;
      padding: 0.875rem;
    }
  }
</style>
