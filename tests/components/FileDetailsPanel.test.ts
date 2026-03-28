import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import FileDetailsPanel from '$lib/components/FileDetailsPanel.svelte';
import type { DuplicateGroup } from '$lib/types';
import { makeGroup } from '../factories';

const photoPaths = [
  '/home/user/documents/photo.jpg',
  '/home/user/downloads/photo.jpg',
  '/home/user/backup/photo.jpg',
];
const group = makeGroup(1, 2048, 3, { paths: photoPaths });

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
    render(FileDetailsPanel, { props: { ...defaultProps, group } });
    expect(screen.getByText('3 Files')).toBeInTheDocument();
  });

  it('original file shows Original badge', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group } });
    expect(screen.getByText('Original')).toBeInTheDocument();
  });

  it('original file checkbox is disabled', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group } });
    const checkboxes = screen.getAllByRole('checkbox');
    // First file is original — its checkbox should be disabled
    expect(checkboxes[0]).toBeDisabled();
  });

  it('non-original file checkboxes are enabled', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group } });
    const checkboxes = screen.getAllByRole('checkbox');
    expect(checkboxes[1]).not.toBeDisabled();
    expect(checkboxes[2]).not.toBeDisabled();
  });

  it('toggling checkbox calls onToggleFile with file path', () => {
    const onToggleFile = vi.fn();
    render(FileDetailsPanel, {
      props: { ...defaultProps, group, onToggleFile },
    });
    const checkboxes = screen.getAllByRole('checkbox');
    checkboxes[1].click();
    expect(onToggleFile).toHaveBeenCalledWith('/home/user/downloads/photo.jpg');
  });

  it('displays both Created and Modified date labels', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group } });
    const createdLabels = screen.getAllByText('Created:');
    const modifiedLabels = screen.getAllByText('Modified:');
    expect(createdLabels.length).toBe(3); // one per file
    expect(modifiedLabels.length).toBe(3);
  });

  it('Select All Except Original button is present', () => {
    render(FileDetailsPanel, { props: { ...defaultProps, group } });
    expect(screen.getByText('Select All Except Original')).toBeInTheDocument();
  });

  it('Select All Except Original button calls callback', () => {
    const onSelectAllExceptOriginal = vi.fn();
    render(FileDetailsPanel, {
      props: { ...defaultProps, group, onSelectAllExceptOriginal },
    });
    screen.getByText('Select All Except Original').click();
    expect(onSelectAllExceptOriginal).toHaveBeenCalledOnce();
  });
});
