import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import { invoke } from '@tauri-apps/api/core';
import DeletionHistoryPanel from '$lib/components/DeletionHistoryPanel.svelte';
import type { DeletionRecord } from '$lib/types';

function mockEmptyHistory() {
  // First call: getDeletionHistorySummary -> [0, 0]
  // Second call: getDeletionHistory -> []
  vi.mocked(invoke).mockResolvedValueOnce([0, 0]).mockResolvedValueOnce([]);
}

function mockHistoryWithRecords(records: DeletionRecord[], totalCount: number, totalFreed: number) {
  vi.mocked(invoke).mockResolvedValueOnce([totalCount, totalFreed]).mockResolvedValueOnce(records);
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
    mockEmptyHistory();
    render(DeletionHistoryPanel, { props: { onClose: vi.fn() } });
    await waitFor(() => {
      expect(screen.getByText('No deletion history yet')).toBeInTheDocument();
    });
  });

  it('renders records with file name and size', async () => {
    mockHistoryWithRecords([sampleRecord], 1, 2048);
    render(DeletionHistoryPanel, { props: { onClose: vi.fn() } });
    await waitFor(() => {
      expect(screen.getByText('report.pdf')).toBeInTheDocument();
    });
  });

  it('summary header shows total count and freed space', async () => {
    mockHistoryWithRecords([sampleRecord], 5, 10240);
    render(DeletionHistoryPanel, { props: { onClose: vi.fn() } });
    await waitFor(() => {
      expect(screen.getByText(/5 files/)).toBeInTheDocument();
      expect(screen.getByText(/freed/)).toBeInTheDocument();
    });
  });

  it('Close button calls onClose', () => {
    mockEmptyHistory();
    const onClose = vi.fn();
    render(DeletionHistoryPanel, { props: { onClose } });
    screen.getByText('Close').click();
    expect(onClose).toHaveBeenCalledOnce();
  });
});
