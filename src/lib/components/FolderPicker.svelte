<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';

  interface Props {
    selectedPaths: string[];
    onPathsChange: (paths: string[]) => void;
  }

  let { selectedPaths, onPathsChange }: Props = $props();

  async function addFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: true,
        title: 'Select folders to scan',
      });

      if (selected) {
        const newPaths = Array.isArray(selected) ? selected : [selected];
        const uniquePaths = [...new Set([...selectedPaths, ...newPaths])];
        onPathsChange(uniquePaths);
      }
    } catch (e) {
      console.error('Failed to select folder:', e);
    }
  }

  function removePath(path: string) {
    onPathsChange(selectedPaths.filter((p) => p !== path));
  }

  function clearAll() {
    onPathsChange([]);
  }

  function truncatePath(path: string): string {
    if (path.length <= 50) return path;
    const parts = path.split('/');
    if (parts.length <= 3) return path;
    return `${parts[0]}/${parts[1]}/.../${parts.slice(-2).join('/')}`;
  }
</script>

<div class="folder-picker">
  <div class="header">
    <h3>Scan Locations</h3>
    <div class="actions">
      {#if selectedPaths.length > 0}
        <button class="clear-btn" onclick={clearAll}>Clear All</button>
      {/if}
      <button class="add-btn" onclick={addFolder}>Add Folder</button>
    </div>
  </div>

  {#if selectedPaths.length === 0}
    <div class="empty-state">
      <p>No folders selected</p>
      <p class="hint">Click "Add Folder" to select folders to scan for duplicates</p>
    </div>
  {:else}
    <ul class="path-list">
      {#each selectedPaths as path}
        <li>
          <span class="path" title={path}>{truncatePath(path)}</span>
          <button class="remove-btn" onclick={() => removePath(path)} aria-label="Remove {path}">
            &times;
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .folder-picker {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  h3 {
    margin: 0;
    font-size: 1rem;
  }

  .actions {
    display: flex;
    gap: 0.5rem;
  }

  .add-btn {
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .clear-btn {
    padding: 0.5rem 1rem;
    background: transparent;
    color: var(--text-secondary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  .empty-state {
    text-align: center;
    padding: 2rem;
    color: var(--text-secondary);
  }

  .empty-state p {
    margin: 0.25rem 0;
  }

  .empty-state .hint {
    font-size: 0.85rem;
  }

  .path-list {
    list-style: none;
    margin: 0;
    padding: 0;
    max-height: 200px;
    overflow-y: auto;
  }

  .path-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    background: var(--background);
    border-radius: 4px;
    margin-bottom: 0.5rem;
  }

  .path {
    font-family: var(--font-mono);
    font-size: 0.85rem;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .remove-btn {
    background: transparent;
    border: none;
    color: var(--text-secondary);
    font-size: 1.25rem;
    cursor: pointer;
    padding: 0 0.25rem;
    line-height: 1;
  }

  .remove-btn:hover {
    color: var(--error);
  }
</style>
