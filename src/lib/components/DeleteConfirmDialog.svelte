<script lang="ts">
  interface Props {
    fileCount: number;
    totalSize: number;
    sampleFiles: string[];
    allInGroup: boolean;
    onConfirm: () => void;
    onCancel: () => void;
  }

  let { fileCount, totalSize, sampleFiles, allInGroup, onConfirm, onCancel }: Props = $props();

  // Extra confirmation required when deleting all copies
  let confirmAllCopies = $state(false);

  // Determine if confirm button should be enabled
  let canConfirm = $derived(!allInGroup || confirmAllCopies);

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="dialog-overlay" onclick={onCancel} onkeydown={(e) => { if (e.key === 'Escape') onCancel(); }} role="dialog" aria-modal="true" aria-label="Confirm Deletion" tabindex="-1">
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="dialog" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
    <h2>Confirm Deletion</h2>

    {#if allInGroup}
      <div class="danger-banner">
        <div class="danger-icon">⚠️</div>
        <div class="danger-content">
          <strong>DANGER: You are deleting ALL copies!</strong>
          <p>This will permanently remove these files from your system. There will be NO remaining copies anywhere.</p>
        </div>
      </div>

      <div class="confirmation-checkbox">
        <label>
          <input type="checkbox" bind:checked={confirmAllCopies} />
          <span>I understand that ALL copies will be deleted and this action cannot be undone</span>
        </label>
      </div>
    {/if}

    <div class="summary">
      <p>
        <strong>{fileCount}</strong> files will be moved to Trash
        ({formatBytes(totalSize)})
      </p>
    </div>

    <div class="sample-files">
      <p>Files to delete:</p>
      <ul>
        {#each sampleFiles.slice(0, 5) as file}
          <li>{file}</li>
        {/each}
        {#if sampleFiles.length > 5}
          <li class="more">...and {sampleFiles.length - 5} more</li>
        {/if}
      </ul>
    </div>

    <div class="note">
      Files will be moved to the system Trash. You can restore them from there if needed.
    </div>

    <div class="actions">
      <button class="cancel-btn" onclick={onCancel}>Cancel</button>
      <button
        class="confirm-btn"
        onclick={onConfirm}
        disabled={!canConfirm}
        class:disabled={!canConfirm}
      >
        {allInGroup ? 'Delete ALL Copies' : 'Delete to Trash'}
      </button>
    </div>
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

  h2 {
    margin: 0 0 1rem;
  }

  /* Stronger danger banner for deleting all copies */
  .danger-banner {
    display: flex;
    gap: 1rem;
    align-items: flex-start;
    background: var(--error);
    color: white;
    padding: 1rem;
    border-radius: 8px;
    margin-bottom: 1rem;
  }

  .danger-icon {
    font-size: 2rem;
    line-height: 1;
  }

  .danger-content strong {
    display: block;
    font-size: 1.1rem;
    margin-bottom: 0.25rem;
  }

  .danger-content p {
    margin: 0;
    font-size: 0.9rem;
    opacity: 0.9;
  }

  .confirmation-checkbox {
    background: var(--error-bg);
    border: 2px solid var(--error);
    padding: 0.75rem 1rem;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .confirmation-checkbox label {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .confirmation-checkbox input[type="checkbox"] {
    width: 1.25rem;
    height: 1.25rem;
    margin-top: 0.125rem;
    flex-shrink: 0;
    cursor: pointer;
  }

  .confirm-btn.disabled,
  .confirm-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .summary {
    margin-bottom: 1rem;
  }

  .sample-files {
    background: var(--background);
    padding: 0.75rem;
    border-radius: 6px;
    margin-bottom: 1rem;
    max-height: 200px;
    overflow-y: auto;
  }

  .sample-files ul {
    margin: 0.5rem 0 0;
    padding-left: 1.5rem;
    font-size: 0.85rem;
    font-family: var(--font-mono);
  }

  .sample-files .more {
    color: var(--text-secondary);
    font-style: italic;
  }

  .note {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 1.5rem;
  }

  .actions {
    display: flex;
    gap: 0.75rem;
    justify-content: flex-end;
  }

  button {
    padding: 0.75rem 1.5rem;
    border-radius: 6px;
    font-weight: 500;
    cursor: pointer;
  }

  .cancel-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
  }

  .confirm-btn {
    background: var(--error);
    border: none;
    color: white;
  }
</style>
