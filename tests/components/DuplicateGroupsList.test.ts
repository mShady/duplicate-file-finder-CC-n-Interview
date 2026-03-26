import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import DuplicateGroupsList from '$lib/components/DuplicateGroupsList.svelte';
import type { DuplicateGroup } from '$lib/types';

function makeGroup(id: number, fileSize: number, fileCount: number): DuplicateGroup {
  return {
    id,
    hash: `hash_${id}`,
    file_size: fileSize,
    files: Array.from({ length: fileCount }, (_, i) => ({
      path: `/files/file${i}.txt`,
      size: fileSize,
      created_at: 1000 + i,
      modified_at: 2000 + i,
      is_original: i === 0,
    })),
    wasted_space: fileSize * (fileCount - 1),
  };
}

describe('DuplicateGroupsList', () => {
  it('renders empty state when no groups', () => {
    render(DuplicateGroupsList, {
      props: { groups: [], selectedGroupId: null, onSelect: vi.fn() },
    });
    expect(screen.getByText('No duplicate groups found')).toBeInTheDocument();
  });

  it('renders group items with file count and wasted space', () => {
    const groups = [makeGroup(1, 1024, 3)];
    render(DuplicateGroupsList, {
      props: { groups, selectedGroupId: null, onSelect: vi.fn() },
    });
    expect(screen.getByText('3 files')).toBeInTheDocument();
    expect(screen.getByText(/wasted/)).toBeInTheDocument();
  });

  it('shows group count in header', () => {
    const groups = [makeGroup(1, 1024, 2), makeGroup(2, 2048, 3)];
    render(DuplicateGroupsList, {
      props: { groups, selectedGroupId: null, onSelect: vi.fn() },
    });
    expect(screen.getByText('2')).toBeInTheDocument();
  });

  it('selected group has aria-selected true', () => {
    const groups = [makeGroup(1, 1024, 2)];
    render(DuplicateGroupsList, {
      props: { groups, selectedGroupId: 1, onSelect: vi.fn() },
    });
    const option = screen.getByRole('option');
    expect(option).toHaveAttribute('aria-selected', 'true');
  });

  it('clicking group item calls onSelect', () => {
    const groups = [makeGroup(1, 1024, 2)];
    const onSelect = vi.fn();
    render(DuplicateGroupsList, {
      props: { groups, selectedGroupId: null, onSelect },
    });
    screen.getByRole('option').click();
    expect(onSelect).toHaveBeenCalledWith(groups[0]);
  });
});
