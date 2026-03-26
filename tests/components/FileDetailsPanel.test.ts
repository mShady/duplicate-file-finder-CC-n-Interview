import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import FileDetailsPanel from '$lib/components/FileDetailsPanel.svelte';
import type { DuplicateGroup } from '$lib/types';

function makeGroup(): DuplicateGroup {
  return {
    id: 1,
    hash: 'abc123',
    file_size: 2048,
    files: [
      {
        path: '/home/user/documents/photo.jpg',
        size: 2048,
        created_at: 1000,
        modified_at: 2000,
        is_original: true,
      },
      {
        path: '/home/user/downloads/photo.jpg',
        size: 2048,
        created_at: 2000,
        modified_at: 3000,
        is_original: false,
      },
      {
        path: '/home/user/backup/photo.jpg',
        size: 2048,
        created_at: 3000,
        modified_at: 4000,
        is_original: false,
      },
    ],
    wasted_space: 4096,
  };
}

const defaultProps = {
  group: null as DuplicateGroup | null,
  selectedFiles: new Set<string>(),
  onToggleFile: vi.fn(),
  onSelectAllExceptOriginal: vi.fn(),
};

describe('FileDetailsPanel', () => {
  it('renders empty state when no group selected', () => {
    render(FileDetailsPanel, { props: { ...defaultProps } });
    expect(screen.getByText('Select a duplicate group to view files')).toBeInTheDocument();
  });

  it('renders file list with correct count heading', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group: makeGroup() } });
    expect(screen.getByText('3 Files')).toBeInTheDocument();
  });

  it('original file shows Original badge', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group: makeGroup() } });
    expect(screen.getByText('Original')).toBeInTheDocument();
  });

  it('original file checkbox is disabled', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group: makeGroup() } });
    const checkboxes = screen.getAllByRole('checkbox');
    // First file is original — its checkbox should be disabled
    expect(checkboxes[0]).toBeDisabled();
  });

  it('non-original file checkboxes are enabled', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group: makeGroup() } });
    const checkboxes = screen.getAllByRole('checkbox');
    expect(checkboxes[1]).not.toBeDisabled();
    expect(checkboxes[2]).not.toBeDisabled();
  });

  it('toggling checkbox calls onToggleFile with file path', () => {
    const onToggleFile = vi.fn();
    render(FileDetailsPanel, {
      props: { ...defaultProps, group: makeGroup(), onToggleFile },
    });
    const checkboxes = screen.getAllByRole('checkbox');
    checkboxes[1].click();
    expect(onToggleFile).toHaveBeenCalledWith('/home/user/downloads/photo.jpg');
  });

  it('displays both Created and Modified date labels', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group: makeGroup() } });
    const createdLabels = screen.getAllByText('Created:');
    const modifiedLabels = screen.getAllByText('Modified:');
    expect(createdLabels.length).toBe(3); // one per file
    expect(modifiedLabels.length).toBe(3);
  });

  it('Select All Except Original button is present', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group: makeGroup() } });
    expect(screen.getByText('Select All Except Original')).toBeInTheDocument();
  });

  it('Select All Except Original button calls callback', () => {
    const onSelectAllExceptOriginal = vi.fn();
    render(FileDetailsPanel, {
      props: { ...defaultProps, group: makeGroup(), onSelectAllExceptOriginal },
    });
    screen.getByText('Select All Except Original').click();
    expect(onSelectAllExceptOriginal).toHaveBeenCalledOnce();
  });
});
