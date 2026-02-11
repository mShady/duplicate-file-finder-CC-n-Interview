import { describe, it, expect } from 'vitest';
import {
  selectAllExceptOldest,
  selectByLocation,
  selectByPathDepth,
  selectDeepestInGroup,
  clearSelection,
} from '../../src/lib/utils/selection';
import type { DuplicateGroup } from '../../src/lib/types';

function makeGroup(id: number, files: { path: string; created_at: number }[]): DuplicateGroup {
  return {
    id,
    hash: `hash_${id}`,
    file_size: 1000,
    files: files.map(f => ({
      path: f.path,
      size: 1000,
      created_at: f.created_at,
      modified_at: f.created_at,
      is_original: false,
    })),
    wasted_space: 1000 * (files.length - 1),
  };
}

describe('selectAllExceptOldest', () => {
  it('should select all files except the oldest in each group', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/file1.txt', created_at: 100 },
        { path: '/a/file2.txt', created_at: 200 },
        { path: '/a/file3.txt', created_at: 300 },
      ]),
    ];

    const selected = selectAllExceptOldest(groups);

    expect(selected.size).toBe(2);
    expect(selected.has('/a/file1.txt')).toBe(false); // oldest, kept
    expect(selected.has('/a/file2.txt')).toBe(true);
    expect(selected.has('/a/file3.txt')).toBe(true);
  });

  it('should handle multiple groups independently', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/old.txt', created_at: 100 },
        { path: '/a/new.txt', created_at: 200 },
      ]),
      makeGroup(2, [
        { path: '/b/old.txt', created_at: 50 },
        { path: '/b/new.txt', created_at: 150 },
      ]),
    ];

    const selected = selectAllExceptOldest(groups);

    expect(selected.size).toBe(2);
    expect(selected.has('/a/old.txt')).toBe(false);
    expect(selected.has('/a/new.txt')).toBe(true);
    expect(selected.has('/b/old.txt')).toBe(false);
    expect(selected.has('/b/new.txt')).toBe(true);
  });

  it('should handle group with only 2 files', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/old.txt', created_at: 100 },
        { path: '/a/new.txt', created_at: 200 },
      ]),
    ];

    const selected = selectAllExceptOldest(groups);

    expect(selected.size).toBe(1);
    expect(selected.has('/a/new.txt')).toBe(true);
  });

  it('should handle empty groups array', () => {
    const selected = selectAllExceptOldest([]);
    expect(selected.size).toBe(0);
  });
});

describe('selectByLocation', () => {
  it('should select files matching folder path prefix', () => {
    const groups = [
      makeGroup(1, [
        { path: '/Users/test/docs/file1.txt', created_at: 100 },
        { path: '/Users/test/photos/file1.txt', created_at: 200 },
        { path: '/Users/other/file1.txt', created_at: 300 },
      ]),
    ];

    const selected = selectByLocation(groups, '/Users/test/docs', new Set());

    expect(selected.size).toBe(1);
    expect(selected.has('/Users/test/docs/file1.txt')).toBe(true);
  });

  it('should add to existing selection', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/file1.txt', created_at: 100 },
        { path: '/b/file1.txt', created_at: 200 },
      ]),
    ];

    const existing = new Set(['/existing/file.txt']);
    const selected = selectByLocation(groups, '/b', existing);

    expect(selected.size).toBe(2);
    expect(selected.has('/existing/file.txt')).toBe(true);
    expect(selected.has('/b/file1.txt')).toBe(true);
  });

  it('should not select files in other locations', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/file.txt', created_at: 100 },
        { path: '/b/file.txt', created_at: 200 },
      ]),
    ];

    const selected = selectByLocation(groups, '/c', new Set());

    expect(selected.size).toBe(0);
  });

  it('should ensure at least one file per group is kept (safety guard)', () => {
    const groups = [
      makeGroup(1, [
        { path: '/same/dir/file1.txt', created_at: 100 },
        { path: '/same/dir/file2.txt', created_at: 200 },
      ]),
    ];

    // Both files are in /same/dir, so selectByLocation would select both
    const selected = selectByLocation(groups, '/same/dir', new Set());

    // The safety guard should ensure at least one is kept
    expect(selected.size).toBe(1);
  });
});

describe('selectByPathDepth', () => {
  it('should select files within depth range', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/file.txt', created_at: 100 },             // depth 2
        { path: '/a/b/c/file.txt', created_at: 200 },         // depth 4
        { path: '/a/b/c/d/e/file.txt', created_at: 300 },     // depth 6
      ]),
    ];

    const selected = selectByPathDepth(groups, 4, 6, new Set());

    expect(selected.has('/a/file.txt')).toBe(false);           // depth 2, out of range
    expect(selected.has('/a/b/c/file.txt')).toBe(true);        // depth 4
    expect(selected.has('/a/b/c/d/e/file.txt')).toBe(true);    // depth 6
  });

  it('should handle null maxDepth', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/file.txt', created_at: 100 },
        { path: '/a/b/c/d/e/file.txt', created_at: 200 },
      ]),
    ];

    const selected = selectByPathDepth(groups, 3, null, new Set());

    expect(selected.has('/a/file.txt')).toBe(false);
    expect(selected.has('/a/b/c/d/e/file.txt')).toBe(true);
  });

  it('should add to existing selection', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/file.txt', created_at: 100 },        // depth 2 - kept by group
        { path: '/a/b/c/file.txt', created_at: 200 },    // depth 4 - selected
      ]),
    ];

    const existing = new Set(['/existing.txt']);
    const selected = selectByPathDepth(groups, 3, null, existing);

    expect(selected.has('/existing.txt')).toBe(true);
    expect(selected.has('/a/b/c/file.txt')).toBe(true);
  });
});

describe('selectDeepestInGroup', () => {
  it('should select files at the deepest level', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/file.txt', created_at: 100 },             // depth 2
        { path: '/a/b/file.txt', created_at: 200 },           // depth 3
        { path: '/a/b/c/file.txt', created_at: 300 },         // depth 4 (deepest)
      ]),
    ];

    const selected = selectDeepestInGroup(groups);

    expect(selected.has('/a/file.txt')).toBe(false);
    expect(selected.has('/a/b/file.txt')).toBe(false);
    expect(selected.has('/a/b/c/file.txt')).toBe(true);
  });

  it('should keep oldest when all files at same depth', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/b/file1.txt', created_at: 100 },
        { path: '/c/d/file2.txt', created_at: 200 },
        { path: '/e/f/file3.txt', created_at: 300 },
      ]),
    ];

    const selected = selectDeepestInGroup(groups);

    // All at depth 3, keep oldest (file1.txt)
    expect(selected.size).toBe(2);
    expect(selected.has('/a/b/file1.txt')).toBe(false);  // oldest, kept
    expect(selected.has('/c/d/file2.txt')).toBe(true);
    expect(selected.has('/e/f/file3.txt')).toBe(true);
  });

  it('should handle multiple groups independently', () => {
    const groups = [
      makeGroup(1, [
        { path: '/a/file.txt', created_at: 100 },
        { path: '/a/b/file.txt', created_at: 200 },
      ]),
      makeGroup(2, [
        { path: '/x/file.txt', created_at: 100 },
        { path: '/x/y/z/file.txt', created_at: 200 },
      ]),
    ];

    const selected = selectDeepestInGroup(groups);

    expect(selected.has('/a/b/file.txt')).toBe(true);
    expect(selected.has('/x/y/z/file.txt')).toBe(true);
    expect(selected.has('/a/file.txt')).toBe(false);
    expect(selected.has('/x/file.txt')).toBe(false);
  });

  it('should handle empty groups array', () => {
    const selected = selectDeepestInGroup([]);
    expect(selected.size).toBe(0);
  });
});

describe('clearSelection', () => {
  it('should return an empty set', () => {
    const selected = clearSelection();
    expect(selected.size).toBe(0);
    expect(selected instanceof Set).toBe(true);
  });
});
