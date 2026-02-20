<script lang="ts">
  interface Props {
    current: number;
    total: number;
  }

  let { current, total }: Props = $props();

  let percent = $derived(total > 0 ? Math.round((current / total) * 100) : 0);
</script>

<div class="dialog-overlay">
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Deleting files"
    aria-live="polite"
  >
    <h2>Moving to Trash</h2>

    <div class="progress-info">
      <span class="progress-label">Verifying and deleting files...</span>
      <span class="progress-count" aria-label="{current} of {total} files processed"
        >{current} / {total}</span
      >
    </div>

    <div
      class="progress-bar-track"
      role="progressbar"
      aria-valuenow={percent}
      aria-valuemin={0}
      aria-valuemax={100}
    >
      <div class="progress-bar-fill" style="width: {percent}%"></div>
    </div>

    <p class="note">Please wait — files will be moved to the system Trash.</p>
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
  }

  .progress-info {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
    font-size: 0.9rem;
  }

  .progress-label {
    color: var(--text-secondary);
  }

  .progress-count {
    font-weight: 600;
    font-variant-numeric: tabular-nums;
  }

  .progress-bar-track {
    height: 8px;
    background: var(--background);
    border-radius: 4px;
    overflow: hidden;
    margin-bottom: 1rem;
  }

  .progress-bar-fill {
    height: 100%;
    background: var(--primary);
    border-radius: 4px;
    transition: width 0.15s ease;
  }

  .note {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin: 0;
  }
</style>
