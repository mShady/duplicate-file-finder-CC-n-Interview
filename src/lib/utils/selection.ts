import type { DuplicateGroup } from '$lib/types';

/**
 * Count directory levels in a file path.
 * Uses '/' or '\\' as separator depending on which appears in the path.
 */
function getPathDepth(path: string): number {
  const separator = path.includes('/') ? '/' : '\\';
  return path.split(separator).filter(Boolean).length;
}

/**
 * Safety helper: given a group and a candidate set of paths to select,
 * ensure at least one file in the group remains unselected.
 * When all files would be selected, the oldest (by created_at) is excluded.
 */
function ensureOneKept(
  group: DuplicateGroup,
  selected: Set<string>
): void {
  const allSelected = group.files.every(f => selected.has(f.path));
  if (allSelected && group.files.length > 0) {
    // Remove the oldest file from selection to keep at least one copy
    const oldest = [...group.files].sort((a, b) => a.created_at - b.created_at)[0];
    selected.delete(oldest.path);
  }
}

/**
 * Select all files except the oldest (original) in each group
 */
export function selectAllExceptOldest(groups: DuplicateGroup[]): Set<string> {
  const selected = new Set<string>();

  for (const group of groups) {
    // Sort by creation date, oldest first
    const sorted = [...group.files].sort((a, b) => a.created_at - b.created_at);

    // Select all except the first (oldest)
    for (let i = 1; i < sorted.length; i++) {
      selected.add(sorted[i].path);
    }
  }

  return selected;
}

/**
 * Select all files in a specific folder path.
 * Safety: ensures at least one copy per group is always kept.
 */
export function selectByLocation(
  groups: DuplicateGroup[],
  folderPath: string,
  currentSelection: Set<string>
): Set<string> {
  const selected = new Set(currentSelection);

  for (const group of groups) {
    for (const file of group.files) {
      if (file.path.startsWith(folderPath)) {
        selected.add(file.path);
      }
    }
    ensureOneKept(group, selected);
  }

  return selected;
}

/**
 * Select files by path depth (number of directory levels).
 * Useful for selecting files in deeper nested directories.
 * Safety: ensures at least one copy per group is always kept.
 */
export function selectByPathDepth(
  groups: DuplicateGroup[],
  minDepth: number,
  maxDepth: number | null,
  currentSelection: Set<string>
): Set<string> {
  const selected = new Set(currentSelection);

  for (const group of groups) {
    for (const file of group.files) {
      const depth = getPathDepth(file.path);
      if (depth >= minDepth && (maxDepth === null || depth <= maxDepth)) {
        selected.add(file.path);
      }
    }
    ensureOneKept(group, selected);
  }

  return selected;
}

/**
 * Select deepest files in each group (files with longest path depth).
 * When all files share the same depth, keeps the oldest and selects the rest.
 */
export function selectDeepestInGroup(groups: DuplicateGroup[]): Set<string> {
  const selected = new Set<string>();

  for (const group of groups) {
    // Find max depth in this group
    let maxDepth = 0;
    for (const file of group.files) {
      const depth = getPathDepth(file.path);
      if (depth > maxDepth) maxDepth = depth;
    }

    // Select all files at max depth (except keep at least one)
    const deepestFiles = group.files.filter(f => getPathDepth(f.path) === maxDepth);
    const otherFiles = group.files.filter(f => getPathDepth(f.path) < maxDepth);

    // If all files are at the same depth, keep the oldest
    if (otherFiles.length === 0) {
      const sorted = [...deepestFiles].sort((a, b) => a.created_at - b.created_at);
      for (let i = 1; i < sorted.length; i++) {
        selected.add(sorted[i].path);
      }
    } else {
      // Select all deepest files
      for (const file of deepestFiles) {
        selected.add(file.path);
      }
    }
  }

  return selected;
}

/**
 * Clear all selections
 */
export function clearSelection(): Set<string> {
  return new Set();
}
