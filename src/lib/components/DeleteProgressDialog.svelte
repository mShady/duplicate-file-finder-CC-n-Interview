<script lang="ts">
  import type { DeletionProgressEvent } from '$lib/types';

  interface Props {
    progress: DeletionProgressEvent | null;
  }

  let { progress }: Props = $props();

  let percent = $derived(
    progress && progress.phase === 'verifying' && progress.total > 0
      ? Math.round((progress.current / progress.total) * 100)
      : 0
  );

  let statusText = $derived.by(() => {
    if (!progress) return 'Preparing...';
    if (progress.phase === 'verifying') {
      return `Verifying file ${progress.current} of ${progress.total}...`;
    }
    if (progress.phase === 'trashing') {
      return `Moving ${progress.total} files to Trash...`;
    }
    return 'Completing...';
  });
</script>

<div class="dialog-overlay">
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Deletion in Progress">
    <h2>Deleting Files</h2>

    <div class="progress-section">
      <p class="status">{statusText}</p>

      <div class="progress-bar-container">
        {#if progress?.phase === 'trashing'}
          <div class="progress-bar indeterminate"></div>
        {:else}
          <div class="progress-bar" style="width: {percent}%"></div>
        {/if}
      </div>

      {#if progress?.phase === 'verifying'}
        <p class="percent">{percent}%</p>
      {/if}
    </div>

    <div class="note">Files will be recoverable from your system Trash.</div>
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
    max-width: 450px;
    width: 90%;
  }

  h2 {
    margin: 0 0 1rem;
  }

  .progress-section {
    margin-bottom: 1rem;
  }

  .status {
    margin: 0 0 0.75rem;
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .progress-bar-container {
    height: 8px;
    background: var(--background);
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-bar {
    height: 100%;
    background: var(--primary);
    border-radius: 4px;
    transition: width 0.15s ease;
  }

  .progress-bar.indeterminate {
    width: 40%;
    animation: indeterminate 1.2s ease-in-out infinite;
  }

  @keyframes indeterminate {
    0% {
      transform: translateX(-100%);
    }
    100% {
      transform: translateX(350%);
    }
  }

  .percent {
    margin: 0.5rem 0 0;
    text-align: center;
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .note {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }
</style>
