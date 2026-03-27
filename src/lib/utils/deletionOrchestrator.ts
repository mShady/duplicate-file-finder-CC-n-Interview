import type { DetectionResult, DeletionRequest } from '$lib/types';

export function buildDeletionRequests(
  detectionResult: DetectionResult,
  pendingFiles: string[]
): DeletionRequest[] {
  const fileMap = new Map<string, { hash: string; size: number }>();
  for (const group of detectionResult.groups) {
    for (const file of group.files) {
      fileMap.set(file.path, { hash: group.hash, size: file.size });
    }
  }

  return pendingFiles
    .filter((path) => fileMap.has(path))
    .map((path) => {
      const info = fileMap.get(path)!;
      return {
        path,
        expected_hash: info.hash,
        size: info.size,
      };
    });
}

export function buildKeptPathsAndGroupIds(
  detectionResult: DetectionResult,
  pendingFiles: string[]
): { keptPaths: Record<string, string>; groupIds: Record<string, number> } {
  const deletingSet = new Set(pendingFiles);
  const keptPaths: Record<string, string> = {};
  const groupIds: Record<string, number> = {};
  for (const group of detectionResult.groups) {
    const keptFile = group.files.find((f) => !deletingSet.has(f.path));
    for (const file of group.files) {
      if (deletingSet.has(file.path)) {
        if (keptFile) {
          keptPaths[file.path] = keptFile.path;
        }
        groupIds[file.path] = group.id;
      }
    }
  }
  return { keptPaths, groupIds };
}

export function updateResultsAfterDeletion(
  detectionResult: DetectionResult,
  deletedPaths: Set<string>
): DetectionResult {
  const updatedGroups = detectionResult.groups
    .map((group) => {
      const remainingFiles = group.files.filter((f) => !deletedPaths.has(f.path));
      if (remainingFiles.length <= 1) return null;
      const wastedSpace = group.file_size * (remainingFiles.length - 1);
      return {
        ...group,
        files: remainingFiles,
        wasted_space: wastedSpace,
      };
    })
    .filter((g): g is NonNullable<typeof g> => g !== null);

  const duplicateCount = updatedGroups.reduce((sum, g) => sum + g.files.length - 1, 0);
  const totalWastedSpace = updatedGroups.reduce((sum, g) => sum + g.wasted_space, 0);

  return {
    ...detectionResult,
    groups: updatedGroups,
    duplicate_count: duplicateCount,
    total_wasted_space: totalWastedSpace,
    unique_files: updatedGroups.length,
  };
}

export function computePendingDeletionSize(
  detectionResult: DetectionResult,
  pendingFiles: string[]
): number {
  const fileMap = new Map<string, number>();
  for (const group of detectionResult.groups) {
    for (const file of group.files) {
      fileMap.set(file.path, file.size);
    }
  }
  let total = 0;
  for (const path of pendingFiles) {
    total += fileMap.get(path) || 0;
  }
  return total;
}

export function isDeletingAllInGroup(
  detectionResult: DetectionResult,
  pendingFiles: string[]
): boolean {
  if (pendingFiles.length === 0) return false;
  const selectedSet = new Set(pendingFiles);
  return detectionResult.groups.some((group) =>
    group.files.every((file) => selectedSet.has(file.path))
  );
}
