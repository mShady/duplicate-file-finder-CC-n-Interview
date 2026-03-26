import { describe, it, expect } from 'vitest';
import type { DetectionResult, DuplicateGroup } from '$lib/types';
import {
  buildDeletionRequests,
  buildKeptPathsAndGroupIds,
  updateResultsAfterDeletion,
  computePendingDeletionSize,
  isDeletingAllInGroup,
} from '$lib/utils/deletionOrchestrator';

function makeGroup(id: number, hash: string, fileSize: number, paths: string[]): DuplicateGroup {
  return {
    id,
    hash,
    file_size: fileSize,
    files: paths.map((p, i) => ({
      path: p,
      size: fileSize,
      created_at: 1000 + i,
      modified_at: 2000 + i,
      is_original: i === 0,
    })),
    wasted_space: fileSize * (paths.length - 1),
  };
}

function makeResult(groups: DuplicateGroup[]): DetectionResult {
  const duplicateCount = groups.reduce((s, g) => s + g.files.length - 1, 0);
  const totalWasted = groups.reduce((s, g) => s + g.wasted_space, 0);
  return {
    groups,
    duplicate_count: duplicateCount,
    total_wasted_space: totalWasted,
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

describe('buildDeletionRequests', () => {
  it('maps pending files to requests with hash and size from groups', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    const requests = buildDeletionRequests(result, ['/b.txt']);
    expect(requests).toEqual([{ path: '/b.txt', expected_hash: 'abc', size: 1024 }]);
  });

  it('filters out paths not found in any group', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    const requests = buildDeletionRequests(result, ['/unknown.txt', '/b.txt']);
    expect(requests).toHaveLength(1);
    expect(requests[0].path).toBe('/b.txt');
  });

  it('returns empty array for empty pending list', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    expect(buildDeletionRequests(result, [])).toEqual([]);
  });
});

describe('buildKeptPathsAndGroupIds', () => {
  it('maps deleted files to kept paths and group IDs', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt', '/c.txt'])]);
    const { keptPaths, groupIds } = buildKeptPathsAndGroupIds(result, ['/b.txt', '/c.txt']);
    expect(keptPaths['/b.txt']).toBe('/a.txt');
    expect(keptPaths['/c.txt']).toBe('/a.txt');
    expect(groupIds['/b.txt']).toBe(1);
    expect(groupIds['/c.txt']).toBe(1);
  });

  it('has no kept path when all files in group are deleted', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    const { keptPaths, groupIds } = buildKeptPathsAndGroupIds(result, ['/a.txt', '/b.txt']);
    expect(keptPaths['/a.txt']).toBeUndefined();
    expect(keptPaths['/b.txt']).toBeUndefined();
    expect(groupIds['/a.txt']).toBe(1);
    expect(groupIds['/b.txt']).toBe(1);
  });

  it('returns empty maps for empty pending list', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    const { keptPaths, groupIds } = buildKeptPathsAndGroupIds(result, []);
    expect(Object.keys(keptPaths)).toHaveLength(0);
    expect(Object.keys(groupIds)).toHaveLength(0);
  });
});

describe('updateResultsAfterDeletion', () => {
  it('removes deleted files from groups', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt', '/c.txt'])]);
    const updated = updateResultsAfterDeletion(result, new Set(['/c.txt']));
    expect(updated.groups[0].files).toHaveLength(2);
    expect(updated.groups[0].files.map((f) => f.path)).toEqual(['/a.txt', '/b.txt']);
  });

  it('drops groups with 1 or fewer files remaining', () => {
    const result = makeResult([
      makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt']),
      makeGroup(2, 'def', 2048, ['/d.txt', '/e.txt', '/f.txt']),
    ]);
    const updated = updateResultsAfterDeletion(result, new Set(['/b.txt']));
    expect(updated.groups).toHaveLength(1);
    expect(updated.groups[0].hash).toBe('def');
  });

  it('recalculates wasted space and duplicate count', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt', '/c.txt'])]);
    const updated = updateResultsAfterDeletion(result, new Set(['/c.txt']));
    expect(updated.groups[0].wasted_space).toBe(1024); // 1024 * (2 - 1)
    expect(updated.duplicate_count).toBe(1);
    expect(updated.total_wasted_space).toBe(1024);
  });

  it('returns empty groups when all files deleted', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    const updated = updateResultsAfterDeletion(result, new Set(['/a.txt', '/b.txt']));
    expect(updated.groups).toHaveLength(0);
    expect(updated.duplicate_count).toBe(0);
    expect(updated.total_wasted_space).toBe(0);
  });
});

describe('computePendingDeletionSize', () => {
  it('sums sizes of pending files', () => {
    const result = makeResult([
      makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt']),
      makeGroup(2, 'def', 2048, ['/c.txt', '/d.txt']),
    ]);
    expect(computePendingDeletionSize(result, ['/b.txt', '/d.txt'])).toBe(1024 + 2048);
  });

  it('returns 0 for empty pending list', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    expect(computePendingDeletionSize(result, [])).toBe(0);
  });

  it('returns 0 for unknown paths', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    expect(computePendingDeletionSize(result, ['/unknown.txt'])).toBe(0);
  });
});

describe('isDeletingAllInGroup', () => {
  it('returns true when all files in a group are selected', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    expect(isDeletingAllInGroup(result, ['/a.txt', '/b.txt'])).toBe(true);
  });

  it('returns false for partial selection', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    expect(isDeletingAllInGroup(result, ['/b.txt'])).toBe(false);
  });

  it('returns false for empty pending list', () => {
    const result = makeResult([makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt'])]);
    expect(isDeletingAllInGroup(result, [])).toBe(false);
  });

  it('returns true when any one group has all files selected', () => {
    const result = makeResult([
      makeGroup(1, 'abc', 1024, ['/a.txt', '/b.txt', '/c.txt']),
      makeGroup(2, 'def', 2048, ['/d.txt', '/e.txt']),
    ]);
    expect(isDeletingAllInGroup(result, ['/d.txt', '/e.txt'])).toBe(true);
  });
});
