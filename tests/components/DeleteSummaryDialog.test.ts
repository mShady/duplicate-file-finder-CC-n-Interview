import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import DeleteSummaryDialog from '$lib/components/DeleteSummaryDialog.svelte';
import type { BatchDeletionResult } from '$lib/types';

function makeBatchResult(
  successCount: number,
  failCount: number,
  totalFreed: number
): BatchDeletionResult {
  return {
    successful: Array.from({ length: successCount }, (_, i) => ({
      path: `/file${i}.txt`,
      success: true,
      error: null,
      size: 1024,
    })),
    failed: Array.from({ length: failCount }, (_, i) => ({
      path: `/fail${i}.txt`,
      success: false,
      error: 'Permission denied',
      size: 512,
    })),
    total_freed: totalFreed,
  };
}

describe('DeleteSummaryDialog', () => {
  it('shows successful deletion count', () => {
    render(DeleteSummaryDialog, {
      props: { result: makeBatchResult(5, 0, 5120), onClose: vi.fn() },
    });
    expect(screen.getByText('5')).toBeInTheDocument();
    expect(screen.getByText('Files deleted')).toBeInTheDocument();
  });

  it('shows failed deletion count when there are failures', () => {
    render(DeleteSummaryDialog, {
      props: { result: makeBatchResult(3, 2, 3072), onClose: vi.fn() },
    });
    expect(screen.getByText('2')).toBeInTheDocument();
    expect(screen.getByText('Failed')).toBeInTheDocument();
  });

  it('Done button calls onClose', () => {
    const onClose = vi.fn();
    render(DeleteSummaryDialog, {
      props: { result: makeBatchResult(1, 0, 1024), onClose },
    });
    screen.getByText('Done').click();
    expect(onClose).toHaveBeenCalledOnce();
  });
});
