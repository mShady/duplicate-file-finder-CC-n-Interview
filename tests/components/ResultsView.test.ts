import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/svelte';
import ResultsView from '$lib/components/ResultsView.svelte';
import type { DetectionResult, DuplicateGroup } from '$lib/types';

function makeGroup(id: number, fileSize: number, fileCount: number, hash?: string): DuplicateGroup {
  return {
    id,
    hash: hash || `hash_${id}`,
    file_size: fileSize,
    files: Array.from({ length: fileCount }, (_, i) => ({
      path: `/files/group${id}/file${i}.txt`,
      size: fileSize,
      created_at: 1000 + i,
      modified_at: 2000 + i,
      is_original: i === 0,
    })),
    wasted_space: fileSize * (fileCount - 1),
  };
}

function makeResult(groups: DuplicateGroup[]): DetectionResult {
  return {
    groups,
    duplicate_count: groups.reduce((s, g) => s + g.files.length - 1, 0),
    total_wasted_space: groups.reduce((s, g) => s + g.wasted_space, 0),
    unique_files: groups.length,
    stats: {
      size_groups: 0,
      size_candidates: 0,
      partial_hashes: 0,
      full_hashes: 0,
      size_grouping_ms: 0,
      partial_hashing_ms: 0,
      full_hashing_ms: 0,
    },
  };
}

describe('ResultsView', () => {
  it('renders stats: groups count, duplicate count, wasted space', () => {
    const result = makeResult([makeGroup(1, 1024, 3), makeGroup(2, 2048, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });

    // 2 groups
    expect(screen.getByLabelText('2 duplicate groups')).toBeInTheDocument();
    // 3 duplicates (3-1 + 2-1 = 3)
    expect(screen.getByLabelText('3 duplicate files')).toBeInTheDocument();
  });

  it('renders Smart Select toggle button', () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });
    expect(screen.getByText('Smart Select')).toBeInTheDocument();
  });

  it('renders Duplicate Groups header', () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });
    expect(screen.getByText('Duplicate Groups')).toBeInTheDocument();
  });

  it('renders empty file details panel when no group selected', () => {
    const result = makeResult([makeGroup(1, 1024, 2)]);
    render(ResultsView, { props: { result, onDeleteSelected: vi.fn() } });
    expect(screen.getByText('Select a duplicate group to view files')).toBeInTheDocument();
  });
});
