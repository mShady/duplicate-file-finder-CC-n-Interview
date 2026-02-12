<script lang="ts">
  import type { DuplicateGroup } from '$lib/types';
  import { formatBytes } from '$lib/utils/format';

  interface Props {
    groups: DuplicateGroup[];
    selectedGroupId: number | null;
    onSelect: (group: DuplicateGroup) => void;
  }

  let { groups, selectedGroupId, onSelect }: Props = $props();

  function getFileExtension(group: DuplicateGroup): string {
    // Defensive: check if files array exists and has items
    if (!group.files || group.files.length === 0) return '';

    const firstFile = group.files[0];
    if (!firstFile || !firstFile.path) return '';

    const path = firstFile.path;
    const ext = path.split('.').pop()?.toLowerCase() || '';
    return ext;
  }

  function getFileTypeIcon(ext: string): string {
    const imageExts = ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg'];
    const videoExts = ['mp4', 'mov', 'avi', 'mkv', 'webm'];
    const audioExts = ['mp3', 'wav', 'flac', 'aac', 'm4a'];
    const docExts = ['pdf', 'doc', 'docx', 'txt', 'rtf', 'md'];

    if (imageExts.includes(ext)) return '\u{1F5BC}\uFE0F';
    if (videoExts.includes(ext)) return '\u{1F3AC}';
    if (audioExts.includes(ext)) return '\u{1F3B5}';
    if (docExts.includes(ext)) return '\u{1F4C4}';
    return '\u{1F4C1}';
  }

  function getGroupLabel(group: DuplicateGroup): string {
    const ext = getFileExtension(group);
    const size = formatBytes(group.file_size);
    const count = group.files?.length || 0;
    return `${count} duplicate ${ext || 'files'}, ${size} each, ${formatBytes(group.wasted_space)} wasted`;
  }
</script>

<div class="groups-list" role="listbox" aria-label="Duplicate file groups">
  <div class="list-header">
    <span class="header-title">Duplicate Groups</span>
    <span class="header-count">{groups.length}</span>
  </div>

  <div class="list-content">
    {#each groups as group (group.id)}
      <button
        class="group-item"
        class:selected={selectedGroupId === group.id}
        onclick={() => onSelect(group)}
        role="option"
        aria-selected={selectedGroupId === group.id}
        aria-label={getGroupLabel(group)}
      >
        <span class="group-icon" aria-hidden="true">{getFileTypeIcon(getFileExtension(group))}</span
        >
        <div class="group-info">
          <div class="group-size">{formatBytes(group.file_size)}</div>
          <div class="group-meta">
            <span class="file-count">{group.files?.length || 0} files</span>
            <span class="wasted">{formatBytes(group.wasted_space)} wasted</span>
          </div>
        </div>
      </button>
    {/each}

    {#if groups.length === 0}
      <div class="empty-state" role="status">No duplicate groups found</div>
    {/if}
  </div>
</div>

<style>
  .groups-list {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--surface);
  }

  .list-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--border);
    font-weight: 600;
  }

  .header-count {
    background: var(--primary);
    color: white;
    padding: 0.125rem 0.5rem;
    border-radius: 10px;
    font-size: 0.8rem;
  }

  .list-content {
    flex: 1;
    overflow-y: auto;
  }

  .group-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.75rem 1rem;
    border: none;
    border-bottom: 1px solid var(--border);
    background: transparent;
    cursor: pointer;
    text-align: left;
    transition: background 0.15s;
    color: var(--text);
    font-family: inherit;
    font-size: inherit;
  }

  .group-item:hover {
    background: var(--background);
  }

  .group-item:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: -2px;
    z-index: 1;
  }

  .group-item.selected {
    background: var(--primary);
    color: white;
  }

  .group-item.selected .group-meta {
    color: rgba(255, 255, 255, 0.8);
  }

  .group-icon {
    font-size: 1.5rem;
  }

  .group-info {
    flex: 1;
    min-width: 0;
  }

  .group-size {
    font-weight: 500;
  }

  .group-meta {
    display: flex;
    gap: 0.75rem;
    font-size: 0.8rem;
    color: var(--text-secondary);
  }

  .wasted {
    color: var(--warning);
  }

  .group-item.selected .wasted {
    color: rgba(255, 255, 255, 0.9);
  }

  .empty-state {
    padding: 2rem;
    text-align: center;
    color: var(--text-secondary);
  }
</style>
