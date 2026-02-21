<script lang="ts">
  import type { DeletionProgressEvent } from '$lib/types';

  interface Props {
    progress: DeletionProgressEvent;
    fileCount: number;
  }

  let { progress, fileCount }: Props = $props();

  // Use progress.total once events arrive, fall back to fileCount for the initial render
  let total = $derived(progress.total > 0 ? progress.total : fileCount);
  let percent = $derived(
    total > 0 ? Math.min(100, Math.round((progress.current / total) * 100)) : 0
  );
  let isVerifying = $derived(progress.current < total);
</script>

<!-- Non-dismissable: no onclick on overlay, no keyboard handler to close -->
<div class="dialog-overlay">
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Deletion in progress"
    aria-live="polite"
  >
    <h2>{isVerifying ? 'Preparing Deletion...' : 'Moving to Trash...'}</h2>

    <div
      class="progress-bar-container"
      role="progressbar"
      aria-valuenow={percent}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div
        class="progress-bar"
        class:indeterminate={!isVerifying}
        style="width: {isVerifying ? percent : 100}%"
      ></div>
    </div>

    <p class="progress-text">
      {#if isVerifying}
        Verifying {progress.current} of {total} files...
      {:else}
        Moving {total} {total === 1 ? 'file' : 'files'} to Trash...
      {/if}
    </p>

    <p class="note">Files will be recoverable from your system Trash.</p>
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
    max-width: 400px;
    width: 90%;
  }

  h2 {
    margin: 0 0 1rem;
    font-size: 1.1rem;
  }

  .progress-bar-container {
    background: var(--border);
    border-radius: 4px;
    height: 8px;
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
    animation: indeterminate 1.5s ease-in-out infinite;
    width: 40% !important;
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
    color: var(--text-secondary);
    margin: 0 0 0.75rem;
  }

  .note {
    font-size: 0.8rem;
    color: var(--text-secondary);
    margin: 0;
    opacity: 0.75;
  }
</style>
