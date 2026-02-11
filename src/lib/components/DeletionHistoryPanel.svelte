<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';
  import type { DeletionRecord } from '$lib/types';
  import { formatBytes } from '$lib/utils/format';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  let history = $state<DeletionRecord[]>([]);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let page = $state(0);
  let hasMore = $state(true);
  const pageSize = 50;
  const PATH_SEP = /[\/\\]/;

  onMount(() => {
    loadHistory();
  });

  async function loadHistory(reset: boolean = false) {
    if (reset) {
      page = 0;
      history = [];
      hasMore = true;
    }

    loading = true;
    error = null;

    try {
      const records = await invoke<DeletionRecord[]>('get_deletion_history', {
        limit: pageSize,
        offset: page * pageSize,
      });

      if (records.length < pageSize) {
        hasMore = false;
      }

      history = reset ? records : [...history, ...records];
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function loadMore() {
    page += 1;
    loadHistory();
  }

  function formatDate(timestamp: number): string {
    if (!timestamp || timestamp < 0) {
      return 'Unknown';
    }

    try {
      return new Date(timestamp * 1000).toLocaleDateString(undefined, {
        year: 'numeric',
        month: 'short',
        day: 'numeric',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return 'Invalid date';
    }
  }

  function getFileName(path: string): string {
    if (!path) return '';

    const parts = path.split(PATH_SEP);
    const fileName = parts[parts.length - 1];
    return fileName || path;
  }

  let totalFreed = $derived(history.reduce((sum, r) => sum + r.file_size, 0));
</script>

<div class="history-panel">
  <div class="header">
    <div class="header-info">
      <h2>Deletion History</h2>
      <span class="summary">{history.length} files &bull; {formatBytes(totalFreed)} freed</span>
    </div>
    <button class="close-btn" onclick={onClose}>Close</button>
  </div>

  {#if error}
    <div class="error-message">{error}</div>
  {/if}

  {#if loading && history.length === 0}
    <div class="loading">Loading history...</div>
  {:else if history.length === 0}
    <div class="empty-state">
      <p>No deletion history yet</p>
      <p class="hint">Deleted files will appear here</p>
    </div>
  {:else}
    <div class="history-list">
      {#each history as record (record.id)}
        <div class="history-item">
          <div class="item-main">
            <span class="file-name">{getFileName(record.file_path)}</span>
            <span class="file-size">{formatBytes(record.file_size)}</span>
          </div>
          <div class="item-details">
            <span class="file-path" title={record.file_path}>{record.file_path}</span>
            <span class="delete-time">Deleted: {formatDate(record.deleted_at)}</span>
          </div>
          {#if record.kept_path}
            <div class="kept-info">
              <span class="kept-label">Kept:</span>
              <span class="kept-path" title={record.kept_path}>{record.kept_path}</span>
            </div>
          {/if}
        </div>
      {/each}

      {#if hasMore}
        <button class="load-more-btn" onclick={loadMore} disabled={loading}>
          {loading ? 'Loading...' : 'Load More'}
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .history-panel {
    background: var(--surface);
    border-radius: 8px;
    padding: 1rem;
    max-height: 500px;
    display: flex;
    flex-direction: column;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    margin-bottom: 1rem;
    padding-bottom: 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .header-info h2 {
    margin: 0;
    font-size: 1.1rem;
  }

  .header-info .summary {
    font-size: 0.85rem;
    color: var(--text-secondary);
  }

  .close-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text);
    padding: 0.5rem 1rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .error-message {
    background: var(--error-bg);
    color: var(--error);
    padding: 0.75rem;
    border-radius: 6px;
    margin-bottom: 1rem;
  }

  .loading, .empty-state {
    text-align: center;
    padding: 2rem;
    color: var(--text-secondary);
  }

  .empty-state .hint {
    font-size: 0.85rem;
    margin-top: 0.5rem;
  }

  .history-list {
    flex: 1;
    overflow-y: auto;
  }

  .history-item {
    padding: 0.75rem;
    background: var(--background);
    border-radius: 6px;
    margin-bottom: 0.5rem;
  }

  .item-main {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.25rem;
  }

  .file-name {
    font-weight: 500;
    word-break: break-all;
  }

  .file-size {
    flex-shrink: 0;
    color: var(--text-secondary);
    font-size: 0.85rem;
    margin-left: 1rem;
  }

  .item-details {
    display: flex;
    justify-content: space-between;
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .file-path {
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 70%;
  }

  .kept-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 0.25rem;
    font-size: 0.75rem;
    color: var(--success);
  }

  .kept-label {
    flex-shrink: 0;
    font-weight: 500;
  }

  .kept-path {
    font-family: var(--font-mono);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-secondary);
  }

  .load-more-btn {
    width: 100%;
    padding: 0.75rem;
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    cursor: pointer;
    margin-top: 0.5rem;
  }

  .load-more-btn:hover:not(:disabled) {
    background: var(--background);
  }

  .load-more-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
