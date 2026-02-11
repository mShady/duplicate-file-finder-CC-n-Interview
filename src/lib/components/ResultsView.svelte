<script lang="ts">
  import MasterDetailLayout from './MasterDetailLayout.svelte';
  import DuplicateGroupsList from './DuplicateGroupsList.svelte';
  import FileDetailsPanel from './FileDetailsPanel.svelte';
  import SmartSelectionPanel from './SmartSelectionPanel.svelte';
  import { untrack } from 'svelte';
  import type { DuplicateGroup, DetectionResult } from '$lib/types';
  import { formatBytes } from '$lib/utils/format';

  interface Props {
    result: DetectionResult;
    onDeleteSelected: (files: string[]) => void;
  }

  let { result, onDeleteSelected }: Props = $props();

  // Sort groups by wasted space descending (spec: "Default Sort: By total wasted space")
  let sortedGroups = $derived(
    [...result.groups].sort((a, b) => b.wasted_space - a.wasted_space),
  );

  let selectedGroup = $state<DuplicateGroup | null>(null);
  let selectedFiles = $state<Set<string>>(new Set());
  let showSmartSelection = $state(false);

  // Cache file-size map: only recomputed when result.groups changes, not on every selection toggle
  let fileSizeMap = $derived.by(() => {
    const map = new Map<string, number>();
    for (const group of result.groups) {
      for (const file of group.files) {
        map.set(file.path, file.size);
      }
    }
    return map;
  });

  // Keep selectedGroup and selectedFiles in sync when result changes (e.g. after deletion).
  // Tracks `result` (not just result.groups) so any parent re-assignment triggers this,
  // even if groups array identity hasn't changed.
  $effect(() => {
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions -- track result identity
    result;
    untrack(() => {
      // Build set of all valid paths
      const validPaths = new Set<string>();
      for (const group of result.groups) {
        for (const file of group.files) {
          validPaths.add(file.path);
        }
      }

      // Remove stale paths from selection
      if (selectedFiles.size > 0) {
        const cleaned = new Set<string>();
        for (const path of selectedFiles) {
          if (validPaths.has(path)) cleaned.add(path);
        }
        if (cleaned.size !== selectedFiles.size) {
          selectedFiles = cleaned;
        }
      }

      // Update selectedGroup reference
      if (selectedGroup) {
        const updated = result.groups.find(g => g.id === selectedGroup!.id);
        if (updated) {
          selectedGroup = updated;
        } else {
          selectedGroup = null;
        }
      }
    });
  });

  function handleGroupSelect(group: DuplicateGroup) {
    selectedGroup = group;
    // Only clear selection when not in smart selection mode (smart selection is cross-group)
    if (!showSmartSelection) {
      selectedFiles = new Set();
    }
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

  function handleSmartSelectionChange(selected: Set<string>) {
    selectedFiles = selected;
  }

  function handleDeleteSelected() {
    if (selectedFiles.size > 0) {
      onDeleteSelected(Array.from(selectedFiles));
    }
  }

  // Uses cached fileSizeMap so only selection changes trigger the sum recomputation
  let selectedSize = $derived.by(() => {
    if (selectedFiles.size === 0) return 0;
    let sum = 0;
    for (const path of selectedFiles) {
      sum += fileSizeMap.get(path) || 0;
    }
    return sum;
  });
</script>

<div class="results-view">
  <div class="results-header">
    <div class="header-stats" role="region" aria-label="Detection statistics">
      <div class="stat">
        <span class="stat-value" aria-label="{result.groups.length} duplicate groups">{result.groups.length}</span>
        <span class="stat-label">Groups</span>
      </div>
      <div class="stat">
        <span class="stat-value" aria-label="{result.duplicate_count} duplicate files">{result.duplicate_count}</span>
        <span class="stat-label">Duplicates</span>
      </div>
      <div class="stat warning">
        <span class="stat-value" aria-label="{formatBytes(result.total_wasted_space)} wasted space">{formatBytes(result.total_wasted_space)}</span>
        <span class="stat-label">Wasted</span>
      </div>
    </div>

    <div class="header-actions">
      <button
        class="smart-select-toggle"
        class:active={showSmartSelection}
        onclick={() => (showSmartSelection = !showSmartSelection)}
      >
        Smart Select
      </button>
      {#if selectedFiles.size > 0}
        <div class="selection-info">
          <span id="selection-summary">{selectedFiles.size} files selected ({formatBytes(selectedSize)})</span>
          <button
            class="delete-button"
            onclick={handleDeleteSelected}
            aria-describedby="selection-summary"
            aria-label="Delete {selectedFiles.size} selected files"
          >
            Delete Selected
          </button>
        </div>
      {/if}
    </div>
  </div>

  {#if showSmartSelection}
    <SmartSelectionPanel
      groups={result.groups}
      {selectedFiles}
      onSelectionChange={handleSmartSelectionChange}
    />
  {/if}

  <div class="results-content">
    <MasterDetailLayout>
      {#snippet master()}
        <DuplicateGroupsList
          groups={sortedGroups}
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
    flex-wrap: wrap;
    gap: 1rem;
  }

  .header-stats {
    display: flex;
    gap: 2rem;
    flex-wrap: wrap;
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

  .header-actions {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .smart-select-toggle {
    padding: 0.5rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .smart-select-toggle:hover {
    background: var(--background);
  }

  .smart-select-toggle.active {
    background: var(--primary);
    color: white;
    border-color: var(--primary);
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
    transition: opacity 0.2s ease;
  }

  .delete-button:hover {
    opacity: 0.9;
  }

  .delete-button:focus-visible {
    outline: 2px solid white;
    outline-offset: 2px;
  }

  .results-content {
    flex: 1;
    overflow: hidden;
  }

  @media (max-width: 768px) {
    .header-stats {
      gap: 1rem;
    }

    .stat-value {
      font-size: 1.2rem;
    }
  }
</style>
