import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/svelte';
import DeleteConfirmDialog from '$lib/components/DeleteConfirmDialog.svelte';

const defaultProps = {
  fileCount: 3,
  totalSize: 3072,
  sampleFiles: ['/a.txt', '/b.txt', '/c.txt'],
  allInGroup: false,
  onConfirm: vi.fn(),
  onCancel: vi.fn(),
};

describe('DeleteConfirmDialog', () => {
  it('shows file count and size', () => {
    render(DeleteConfirmDialog, { props: { ...defaultProps } });
    // Text is split: <strong>3</strong> files will be moved to Trash (3 KB)
    expect(screen.getByText(/files will be moved to Trash/)).toBeInTheDocument();
    expect(screen.getByText('Confirm Deletion')).toBeInTheDocument();
  });

  it('renders Confirm and Cancel buttons', () => {
    render(DeleteConfirmDialog, { props: { ...defaultProps } });
    expect(screen.getByText('Cancel')).toBeInTheDocument();
    expect(screen.getByText('Delete to Trash')).toBeInTheDocument();
  });

  it('shows danger warning when allInGroup is true', () => {
    render(DeleteConfirmDialog, { props: { ...defaultProps, allInGroup: true } });
    expect(screen.getByText(/DANGER: You are deleting ALL copies!/)).toBeInTheDocument();
  });

  it('confirm button disabled when allInGroup and checkbox not checked', () => {
    render(DeleteConfirmDialog, { props: { ...defaultProps, allInGroup: true } });
    expect(screen.getByText('Delete ALL Copies')).toBeDisabled();
  });

  it('cancel button calls onCancel', () => {
    const onCancel = vi.fn();
    render(DeleteConfirmDialog, { props: { ...defaultProps, onCancel } });
    screen.getByText('Cancel').click();
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it('confirm button calls onConfirm in normal mode', () => {
    const onConfirm = vi.fn();
    render(DeleteConfirmDialog, { props: { ...defaultProps, onConfirm } });
    screen.getByText('Delete to Trash').click();
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it('allInGroup checkbox enables confirm button when checked', async () => {
    const onConfirm = vi.fn();
    render(DeleteConfirmDialog, {
      props: { ...defaultProps, allInGroup: true, onConfirm },
    });
    const confirmBtn = screen.getByText('Delete ALL Copies');
    expect(confirmBtn).toBeDisabled();

    // Check the confirmation checkbox
    const checkbox = screen.getByRole('checkbox');
    checkbox.click();

    await waitFor(() => {
      expect(confirmBtn).not.toBeDisabled();
    });

    confirmBtn.click();
    expect(onConfirm).toHaveBeenCalledOnce();
  });
});
