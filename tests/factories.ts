import type { DetectionResult, DuplicateGroup, DuplicateFile } from '$lib/types';

/**
 * Flexible factory for DuplicateGroup test fixtures.
 *
 * Supports multiple call patterns:
 * - makeGroup(id, fileSize, fileCount) — generates numbered file paths
 * - makeGroup(id, fileSize, fileCount, { hash, paths }) — custom hash/paths
 */
export function makeGroup(
  id: number,
  fileSize: number,
  fileCount: number,
  options?: { hash?: string; paths?: string[] }
): DuplicateGroup {
  const hash = options?.hash ?? `hash_${id}`;
  const paths = options?.paths;

  const files: DuplicateFile[] = Array.from({ length: fileCount }, (_, i) => ({
    path: paths ? paths[i] : `/files/group${id}/file${i}.txt`,
    size: fileSize,
    created_at: 1000 + i,
    modified_at: 2000 + i,
    is_original: i === 0,
  }));

  return {
    id,
    hash,
    file_size: fileSize,
    files,
    wasted_space: fileSize * (fileCount - 1),
  };
}

/**
 * Factory for DetectionResult test fixtures.
 * Derives duplicate_count, total_wasted_space, and unique_files from groups.
 *
 * Note on unique_files: set to groups.length (one unique hash per group).
 * The backend computes total_files - duplicate_count which equals this when
 * all files belong to duplicate groups (no standalone non-duplicate files).
 */
export function makeResult(groups: DuplicateGroup[]): DetectionResult {
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
