<script lang="ts">
  import type { BatchDeletionResult } from '$lib/types';
  import { formatBytes } from '$lib/utils/format';

  interface Props {
    result: BatchDeletionResult;
    onClose: () => void;
  }

  let { result, onClose }: Props = $props();

  // Auto-focus the dialog on mount
  let dialogRef: HTMLDivElement | undefined = $state();

  $effect(() => {
    if (dialogRef) {
      dialogRef.focus();
    }
  });

  function handleDialogKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
      return;
    }
    if (e.key === 'Tab') {
      trapFocus(e);
    }
  }

  function trapFocus(e: KeyboardEvent) {
    if (!dialogRef) return;
    const focusable = dialogRef.querySelectorAll<HTMLElement>(
      'button:not([disabled]), [tabindex]:not([tabindex="-1"])'
    );
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];

    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<!-- svelte-ignore a11y_click_events_have_key_events -->
<div class="dialog-overlay" onclick={onClose}>
  <div
    class="dialog"
    role="dialog"
    aria-modal="true"
    aria-label="Deletion Complete"
    bind:this={dialogRef}
    tabindex="-1"
    onclick={(e) => e.stopPropagation()}
    onkeydown={handleDialogKeydown}
  >
    <h2>Deletion Complete</h2>

    <div class="summary">
      <div class="stat success">
        <span class="value">{result.successful.length}</span>
        <span class="label">Files deleted</span>
      </div>
      <div class="stat">
        <span class="value">{formatBytes(result.total_freed)}</span>
        <span class="label">Space freed</span>
      </div>
      {#if result.failed.length > 0}
        <div class="stat error">
          <span class="value">{result.failed.length}</span>
          <span class="label">Failed</span>
        </div>
      {/if}
    </div>

    {#if result.failed.length > 0}
      <div class="failed-section">
        <h3>Failed Deletions</h3>
        <ul>
          {#each result.failed as item}
            <li>
              <span class="path">{item.path}</span>
              <span class="error-msg">{item.error}</span>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    <div class="note">
      Deleted files have been moved to your system Trash. You can restore them from there if needed.
    </div>

    <button class="close-btn" onclick={onClose}>Done</button>
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
    max-width: 500px;
    width: 90%;
  }

  .dialog:focus {
    outline: none;
  }

  h2 {
    margin: 0 0 1rem;
  }

  .summary {
    display: flex;
    gap: 2rem;
    margin-bottom: 1.5rem;
  }

  .stat {
    text-align: center;
  }

  .stat .value {
    display: block;
    font-size: 1.5rem;
    font-weight: 600;
  }

  .stat .label {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .stat.success .value {
    color: var(--success);
  }

  .stat.error .value {
    color: var(--error);
  }

  .failed-section {
    background: var(--error-bg);
    padding: 1rem;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .failed-section h3 {
    margin: 0 0 0.5rem;
    font-size: 0.9rem;
    color: var(--error);
  }

  .failed-section ul {
    margin: 0;
    padding: 0;
    list-style: none;
    font-size: 0.85rem;
  }

  .failed-section li {
    margin-bottom: 0.5rem;
  }

  .failed-section .path {
    display: block;
    font-family: var(--font-mono);
  }

  .failed-section .error-msg {
    color: var(--error);
    font-size: 0.8rem;
  }

  .note {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 1.5rem;
  }

  .close-btn {
    width: 100%;
    padding: 0.75rem;
    background: var(--primary);
    border: none;
    border-radius: 6px;
    color: white;
    font-weight: 500;
    cursor: pointer;
  }
</style>
