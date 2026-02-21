<script lang="ts">
  interface Props {
    current: number;
    total: number;
    phase: 'verifying' | 'deleting';
  }

  let { current, total, phase = 'verifying' }: Props = $props();

  // Verification occupies 0–50%.
  // Deletion: bar sits at 50% with a pulse animation while the batch call runs
  // (current === 0), then jumps to 100% when done (current > 0).
  let percent = $derived(
    total > 0
      ? phase === 'verifying'
        ? Math.round((current / total) * 50)
        : current === 0
          ? 50
          : 100
      : 0
  );

  // True while the batch trash call is in flight — no per-file events available,
  // so we show a pulse animation instead of a frozen bar.
  let isWaiting = $derived(phase === 'deleting' && current === 0);

  let phaseLabel = $derived(phase === 'verifying' ? 'Verifying files...' : 'Moving to Trash...');
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
      <span class="progress-label">{phaseLabel}</span>
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
      <div class="progress-bar-fill" class:waiting={isWaiting} style="width: {percent}%"></div>
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

  .progress-bar-fill.waiting {
    animation: pulse 1.2s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }

  .note {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin: 0;
  }
</style>
