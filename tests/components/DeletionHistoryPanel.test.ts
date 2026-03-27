import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import DeletionHistoryPanel from '$lib/components/DeletionHistoryPanel.svelte';
import type { DeletionRecord } from '$lib/types';

/** Mock invoke by dispatching on the Tauri command name, not call order. */
function mockInvoke(handlers: Record<string, unknown>) {
  vi.mocked(invoke).mockImplementation((cmd: string) => {
    if (cmd in handlers) {
      const val = handlers[cmd];
      return val instanceof Error ? Promise.reject(val) : Promise.resolve(val);
    }
    return Promise.resolve(undefined);
  });
}

const sampleRecord: DeletionRecord = {
  id: 1,
  file_path: '/home/user/docs/report.pdf',
  file_size: 2048,
  file_hash: 'abc123',
  deleted_at: 1700000000,
  group_id: 1,
  kept_path: '/home/user/backup/report.pdf',
};

describe('DeletionHistoryPanel', () => {
  it('shows loading state initially', () => {
    // Mock invoke to never resolve (stays loading)
    vi.mocked(invoke).mockReturnValue(new Promise(() => {}));
    render(DeletionHistoryPanel, { props: { onClose: vi.fn() } });
    expect(screen.getByText('Loading history...')).toBeInTheDocument();
  });

  it('shows empty state when no history', async () => {
    mockInvoke({
      get_deletion_history_summary: [0, 0],
      get_deletion_history: [],
    });
    render(DeletionHistoryPanel, { props: { onClose: vi.fn() } });
    await waitFor(() => {
      expect(screen.getByText('No deletion history yet')).toBeInTheDocument();
    });
  });

  it('renders records with file name and size', async () => {
    mockInvoke({
      get_deletion_history_summary: [1, 2048],
      get_deletion_history: [sampleRecord],
    });
    render(DeletionHistoryPanel, { props: { onClose: vi.fn() } });
    await waitFor(() => {
      expect(screen.getByText('report.pdf')).toBeInTheDocument();
    });
  });

  it('summary header shows total count and freed space', async () => {
    mockInvoke({
      get_deletion_history_summary: [5, 10240],
      get_deletion_history: [sampleRecord],
    });
    render(DeletionHistoryPanel, { props: { onClose: vi.fn() } });
    await waitFor(() => {
      expect(screen.getByText(/5 files/)).toBeInTheDocument();
      expect(screen.getByText(/10 KB/)).toBeInTheDocument();
    });
  });

  it('shows error state when API rejects', async () => {
    mockInvoke({
      get_deletion_history_summary: [0, 0],
      get_deletion_history: new Error('Network error'),
    });
    render(DeletionHistoryPanel, { props: { onClose: vi.fn() } });
    await waitFor(() => {
      expect(screen.getByText(/Network error/)).toBeInTheDocument();
    });
  });

  it('shows Load More button when a full page of records is returned', async () => {
    // pageSize is 50 in the component — returning exactly 50 records means hasMore = true
    const fullPage = Array.from({ length: 50 }, (_, i) => ({
      ...sampleRecord,
      id: i + 1,
      file_path: `/files/file${i}.txt`,
    }));
    mockInvoke({
      get_deletion_history_summary: [100, 50000],
      get_deletion_history: fullPage,
    });
    render(DeletionHistoryPanel, { props: { onClose: vi.fn() } });
    await waitFor(() => {
      expect(screen.getByText('Load More')).toBeInTheDocument();
    });
  });

  it('Close button calls onClose', () => {
    mockInvoke({
      get_deletion_history_summary: [0, 0],
      get_deletion_history: [],
    });
    const onClose = vi.fn();
    render(DeletionHistoryPanel, { props: { onClose } });
    screen.getByText('Close').click();
    expect(onClose).toHaveBeenCalledOnce();
  });
});
