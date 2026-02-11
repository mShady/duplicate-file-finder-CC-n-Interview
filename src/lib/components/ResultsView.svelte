<script lang="ts">
  import MasterDetailLayout from './MasterDetailLayout.svelte';
  import DuplicateGroupsList from './DuplicateGroupsList.svelte';
  import FileDetailsPanel from './FileDetailsPanel.svelte';
  import type { DuplicateGroup, DetectionResult } from '$lib/types';

  interface Props {
    result: DetectionResult;
    onDeleteSelected: (files: string[]) => void;
  }

  let { result, onDeleteSelected }: Props = $props();

  let selectedGroup = $state<DuplicateGroup | null>(null);
  let selectedFiles = $state<Set<string>>(new Set());

  function handleGroupSelect(group: DuplicateGroup) {
    selectedGroup = group;
    selectedFiles = new Set();
  }

  function handleToggleFile(path: string) {
    const newSet = new Set(selectedFiles);
    if (newSet.has(path)) {
      newSet.delete(path);
    } else {
      newSet.add(path);
    }
    selectedFiles = newSet;
  }

  function handleSelectAllExceptOriginal() {
    if (!selectedGroup) return;
    const newSet = new Set<string>();
    for (const file of selectedGroup.files) {
      if (!file.is_original) {
        newSet.add(file.path);
      }
    }
    selectedFiles = newSet;
  }

  function handleDeleteSelected() {
    if (selectedFiles.size > 0) {
      onDeleteSelected(Array.from(selectedFiles));
    }
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  let selectedSize = $derived(
    selectedGroup
      ? Array.from(selectedFiles).reduce((sum, path) => {
          const file = selectedGroup!.files.find((f) => f.path === path);
          return sum + (file?.size || 0);
        }, 0)
      : 0,
  );
</script>

<div class="results-view">
  <div class="results-header">
    <div class="header-stats">
      <div class="stat">
        <span class="stat-value">{result.groups.length}</span>
        <span class="stat-label">Groups</span>
      </div>
      <div class="stat">
        <span class="stat-value">{result.duplicate_count}</span>
        <span class="stat-label">Duplicates</span>
      </div>
      <div class="stat warning">
        <span class="stat-value">{formatBytes(result.total_wasted_space)}</span>
        <span class="stat-label">Wasted</span>
      </div>
    </div>

    {#if selectedFiles.size > 0}
      <div class="selection-info">
        <span>{selectedFiles.size} files selected ({formatBytes(selectedSize)})</span>
        <button class="delete-button" onclick={handleDeleteSelected}>
          Delete Selected
        </button>
      </div>
    {/if}
  </div>

  <div class="results-content">
    <MasterDetailLayout>
      {#snippet master()}
        <DuplicateGroupsList
          groups={result.groups}
          selectedGroupId={selectedGroup?.id ?? null}
          onSelect={handleGroupSelect}
        />
      {/snippet}
      {#snippet detail()}
        <FileDetailsPanel
          group={selectedGroup}
          {selectedFiles}
          onToggleFile={handleToggleFile}
          onSelectAllExceptOriginal={handleSelectAllExceptOriginal}
        />
      {/snippet}
    </MasterDetailLayout>
  </div>
</div>

<style>
  .results-view {
    display: flex;
    flex-direction: column;
    height: 100%;
  }

  .results-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .header-stats {
    display: flex;
    gap: 2rem;
  }

  .stat {
    display: flex;
    flex-direction: column;
    align-items: center;
  }

  .stat-value {
    font-size: 1.5rem;
    font-weight: 600;
  }

  .stat-label {
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .stat.warning .stat-value {
    color: var(--warning);
  }

  .selection-info {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: white;
    border-radius: 6px;
  }

  .delete-button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    background: var(--error);
    color: white;
    cursor: pointer;
    font-weight: 500;
  }

  .delete-button:hover {
    opacity: 0.9;
  }

  .results-content {
    flex: 1;
    overflow: hidden;
  }
</style>
