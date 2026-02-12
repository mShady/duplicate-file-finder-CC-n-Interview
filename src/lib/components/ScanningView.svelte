<script lang="ts">
  import type { ScanProgress, ScanComplete, ScanPhaseEvent } from '$lib/types';
  import { formatBytes } from '$lib/utils/format';

  interface Props {
    phase: ScanPhaseEvent['phase'] | 'idle';
    progress: ScanProgress | null;
    scanResult: ScanComplete | null;
    onCancel: () => void;
  }

  let { phase, progress, scanResult, onCancel }: Props = $props();

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
</script>

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
    <button class="cancel-button" onclick={onCancel}>Cancel</button>
  </div>
</div>

<style>
  .scanning-view {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 1rem;
  }

  .scanning-content {
    text-align: center;
    max-width: 500px;
    width: 100%;
  }

  .scanning-content h2 {
    margin-bottom: 0.5rem;
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

  @media (max-width: 768px) {
    .scanning-content {
      max-width: 100%;
      padding: 0.5rem;
    }

    .current-path {
      font-size: 0.7rem;
    }
  }

  @media (max-width: 480px) {
    .progress-info {
      font-size: 0.85rem;
    }
  }
</style>
