<script lang="ts">
  import type { DuplicateGroup } from '$lib/types';
  import { formatBytes, formatDate, getFileName } from '$lib/utils/format';

  interface Props {
    group: DuplicateGroup | null;
    selectedFiles: Set<string>;
    onToggleFile: (path: string) => void;
    onSelectAllExceptOriginal: () => void;
  }

  let { group, selectedFiles, onToggleFile, onSelectAllExceptOriginal }: Props = $props();

  const PATH_SEP = /[/\\]/;

  function getDirectory(path: string, maxLength: number = 50): string {
    if (!path) return '';

    const parts = path.split(PATH_SEP);
    if (parts.length <= 1) return '';

    parts.pop();
    const dir = parts.join('/');

    if (dir.length <= maxLength) {
      return dir;
    }

    const ellipsis = '/...';
    const availableLength = maxLength - ellipsis.length;

    if (availableLength < 10) {
      return ellipsis + dir.slice(-availableLength);
    }

    const startLength = Math.ceil(availableLength * 0.4);
    const endLength = Math.floor(availableLength * 0.6);

    const start = dir.slice(0, startLength);
    const end = dir.slice(-endLength);

    return `${start}${ellipsis}${end}`;
  }

  function getFileLabel(fileName: string, isOriginal: boolean): string {
    return isOriginal ? `${fileName} (Original, cannot be deleted)` : fileName;
  }
</script>

<div class="details-panel">
  {#if group}
    <div class="panel-header">
      <div class="header-info">
        <h2>{group.files.length} Files</h2>
        <span class="header-meta">
          {formatBytes(group.file_size)} each &bull; {formatBytes(group.wasted_space)} wasted
        </span>
      </div>
      <button 
        class="action-button" 
        onclick={onSelectAllExceptOriginal}
        aria-label="Select all duplicate files except the original for deletion"
      >
        Select All Except Original
      </button>
    </div>

    <div class="files-list">
      {#each group.files as file (file.path)}
        {@const fileName = getFileName(file.path)}
        <div class="file-item" class:original={file.is_original}>
          <label class="file-checkbox">
            <input
              type="checkbox"
              checked={selectedFiles.has(file.path)}
              disabled={file.is_original}
              onchange={() => onToggleFile(file.path)}
              aria-label={getFileLabel(fileName, file.is_original)}
            />
          </label>

          <div class="file-info">
            <div class="file-name">
              {#if file.is_original}
                <span class="original-badge" aria-label="Original file">Original</span>
              {/if}
              {fileName}
            </div>
            <div class="file-path" title={file.path}>
              {getDirectory(file.path)}
            </div>
            <div class="file-dates">
              <span class="date-label">Created:</span>
              <span class="date-value">{formatDate(file.created_at)}</span>
              <span class="date-separator" aria-hidden="true">|</span>
              <span class="date-label">Modified:</span>
              <span class="date-value">{formatDate(file.modified_at)}</span>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {:else}
    <div class="empty-state" role="status">
      <p>Select a duplicate group to view files</p>
    </div>
  {/if}
</div>

<style>
  .details-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: var(--background);
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem;
    border-bottom: 1px solid var(--border);
    background: var(--surface);
  }

  .header-info h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  .header-meta {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .action-button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    background: var(--primary);
    color: white;
    cursor: pointer;
    font-size: 0.875rem;
    transition: opacity 0.15s;
  }

  .action-button:hover {
    opacity: 0.9;
  }

  .action-button:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }

  .files-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .file-item {
    display: flex;
    gap: 0.75rem;
    padding: 0.75rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    margin-bottom: 0.5rem;
    background: var(--surface);
  }

  .file-item.original {
    border-color: var(--success);
    background: var(--success-bg);
  }

  .file-checkbox {
    display: flex;
    align-items: flex-start;
    padding-top: 0.25rem;
    cursor: pointer;
  }

  .file-checkbox input {
    width: 18px;
    height: 18px;
    cursor: pointer;
  }

  .file-checkbox input:disabled {
    cursor: not-allowed;
    opacity: 0.5;
  }

  .file-checkbox input:focus-visible {
    outline: 2px solid var(--primary);
    outline-offset: 2px;
  }

  .file-info {
    flex: 1;
    min-width: 0;
  }

  .file-name {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-weight: 500;
    word-break: break-word;
  }

  .original-badge {
    font-size: 0.7rem;
    padding: 0.1rem 0.4rem;
    background: var(--success);
    color: white;
    border-radius: 3px;
    flex-shrink: 0;
  }

  .file-path {
    font-size: 0.8rem;
    color: var(--text-secondary);
    font-family: var(--font-mono);
    margin-top: 0.25rem;
    word-break: break-word;
  }

  .file-dates {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin-top: 0.5rem;
    flex-wrap: wrap;
  }

  .file-dates .date-label {
    color: var(--text-secondary);
    opacity: 0.8;
  }

  .file-dates .date-value {
    color: var(--text);
  }

  .file-dates .date-separator {
    color: var(--border);
    margin: 0 0.25rem;
  }

  .empty-state {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
  }
</style>
