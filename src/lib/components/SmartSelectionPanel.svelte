<script lang="ts">
  import type { DuplicateGroup } from '$lib/types';
  import {
    selectAllExceptOldest,
    selectByLocation,
    selectByPathDepth,
    selectDeepestInGroup,
  } from '$lib/utils/selection';

  interface Props {
    groups: DuplicateGroup[];
    selectedFiles: Set<string>;
    onSelectionChange: (selected: Set<string>) => void;
  }

  let { groups, selectedFiles, onSelectionChange }: Props = $props();

  let pathDepthMin = $state(1);
  let pathDepthMax = $state<number | null>(null);
  let folderPath = $state('');

  function handleSelectAllExceptOldest() {
    onSelectionChange(selectAllExceptOldest(groups));
  }

  function handleSelectByLocation() {
    if (folderPath.trim()) {
      onSelectionChange(selectByLocation(groups, folderPath.trim(), selectedFiles));
    }
  }

  function handleSelectByPathDepth() {
    onSelectionChange(selectByPathDepth(groups, pathDepthMin, pathDepthMax, selectedFiles));
  }

  function handleSelectDeepest() {
    onSelectionChange(selectDeepestInGroup(groups));
  }
</script>

<div class="smart-selection">
  <h3>Smart Selection</h3>

  <div class="selection-option">
    <button onclick={handleSelectAllExceptOldest}> Select All Except Oldest </button>
    <p class="hint">Keep the original (oldest) file in each group</p>
  </div>

  <div class="selection-option">
    <button onclick={handleSelectDeepest}> Select Deepest Files </button>
    <p class="hint">Select files in the deepest directory levels</p>
  </div>

  <div class="selection-option">
    <div class="input-group">
      <label>
        Path depth range:
        <input type="number" bind:value={pathDepthMin} min="1" placeholder="Min" />
        to
        <input type="number" bind:value={pathDepthMax} min="1" placeholder="Max (optional)" />
      </label>
      <button onclick={handleSelectByPathDepth}>Select by Depth</button>
    </div>
    <p class="hint">Select files at specific directory depth levels</p>
  </div>

  <div class="selection-option">
    <div class="input-group">
      <input type="text" bind:value={folderPath} placeholder="Enter folder path..." />
      <button onclick={handleSelectByLocation} disabled={!folderPath.trim()}>
        Select by Location
      </button>
    </div>
    <p class="hint">Select all duplicates in a specific folder</p>
  </div>
</div>

<style>
  .smart-selection {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
  }

  h3 {
    margin: 0 0 1rem;
    font-size: 1rem;
  }

  .selection-option {
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }

  .selection-option:last-child {
    margin-bottom: 0;
    padding-bottom: 0;
    border-bottom: none;
  }

  button {
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }

  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .hint {
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin: 0.25rem 0 0;
  }

  .input-group {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    flex-wrap: wrap;
  }

  input[type='text'],
  input[type='number'] {
    padding: 0.5rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--background);
    color: var(--text);
  }

  input[type='number'] {
    width: 80px;
  }

  input[type='text'] {
    flex: 1;
    min-width: 200px;
  }
</style>
