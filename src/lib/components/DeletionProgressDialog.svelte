<script lang="ts">
  import type { DeletionProgressEvent } from '$lib/types';

  interface Props {
    progress: DeletionProgressEvent | null;
  }

  let { progress }: Props = $props();

  let isIndeterminate = $derived(!progress || progress.phase === 'trashing');

  let percentage = $derived(
    progress && progress.total > 0 ? Math.round((progress.completed / progress.total) * 100) : 0
  );

  let phaseLabel = $derived.by(() => {
    if (!progress) return 'Preparing...';
    return progress.phase === 'verifying' ? 'Verifying files...' : 'Moving to Trash...';
  });
</script>

<div class="dialog-overlay">
  <div class="dialog" role="dialog" aria-modal="true" aria-label="Deletion Progress">
    <h2>Deleting Files</h2>

    <p class="phase-label">{phaseLabel}</p>

    <div class="progress-bar-container">
      {#if isIndeterminate}
        <div class="progress-bar indeterminate"></div>
      {:else}
        <div class="progress-bar" style="width: {percentage}%"></div>
      {/if}
    </div>

    <p class="progress-text">
      {#if !progress}
        Preparing files for deletion...
      {:else if progress.phase === 'verifying'}
        {progress.completed} of {progress.total} files verified
      {:else}
        Moving {progress.total} files to Trash...
      {/if}
    </p>

    {#if progress?.current_path && progress.phase === 'verifying'}
      <p class="current-path" title={progress.current_path}>{progress.current_path}</p>
    {/if}

    <p class="recovery-note">Files will be recoverable from your system Trash.</p>
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
    text-align: center;
  }

  h2 {
    margin: 0 0 1rem;
  }

  .phase-label {
    color: var(--text-secondary);
    margin-bottom: 1rem;
    font-size: 0.95rem;
  }

  .progress-bar-container {
    height: 8px;
    background: var(--background);
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 0.75rem;
  }

  .progress-bar {
    height: 100%;
    background: var(--primary);
    border-radius: 4px;
    transition: width 0.2s ease;
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

  .progress-text {
    font-size: 0.9rem;
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
    margin: 0;
  }

  .recovery-note {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin: 1rem 0 0;
  }
</style>
