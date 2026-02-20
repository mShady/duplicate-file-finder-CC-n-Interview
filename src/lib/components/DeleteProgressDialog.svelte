<script lang="ts">
  import type { DeletionProgressEvent } from '$lib/types';

  interface Props {
    progress: DeletionProgressEvent | null;
    fileCount: number;
  }

  let { progress, fileCount }: Props = $props();

  let percent = $derived(
    progress && progress.total > 0 ? Math.round((progress.current / progress.total) * 100) : 0
  );

  let displayTotal = $derived(progress?.total ?? fileCount);
  let displayCurrent = $derived(progress?.current ?? 0);
</script>

<div class="dialog-overlay">
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Deleting files"
    aria-live="polite"
  >
    <div class="spinner"></div>
    <h2>Moving to Trash...</h2>

    {#if progress}
      <p class="status">Verifying {displayCurrent} of {displayTotal} files</p>
      <div
        class="progress-bar"
        role="progressbar"
        aria-valuenow={percent}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        <div class="progress-fill" style="width: {percent}%"></div>
      </div>
      {#if progress.current_path}
        <p class="current-path" title={progress.current_path}>{progress.current_path}</p>
      {/if}
    {:else}
      <p class="status">Preparing {fileCount} files...</p>
    {/if}
  </div>
</div>

<style>
  .dialog-overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--surface);
    border-radius: 12px;
    padding: 1.5rem;
    max-width: 420px;
    width: 90%;
    text-align: center;
  }

  h2 {
    margin: 0 0 0.75rem;
  }

  .spinner {
    width: 40px;
    height: 40px;
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

  .status {
    color: var(--text-secondary);
    margin-bottom: 0.75rem;
    font-size: 0.9rem;
  }

  .progress-bar {
    background: var(--border);
    border-radius: 4px;
    height: 6px;
    overflow: hidden;
    margin-bottom: 0.75rem;
  }

  .progress-fill {
    background: var(--primary);
    height: 100%;
    border-radius: 4px;
    transition: width 0.15s ease;
  }

  .current-path {
    font-size: 0.75rem;
    font-family: var(--font-mono);
    color: var(--text-secondary);
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    margin: 0;
  }
</style>
