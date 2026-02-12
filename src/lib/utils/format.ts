const PATH_SEP = /[/\\]/;

/**
 * Format a byte count into a human-readable string (e.g. "1.5 MB").
 */
export function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  if (bytes < 0) return '-' + formatBytes(-bytes);
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.min(Math.floor(Math.log(bytes) / Math.log(k)), sizes.length - 1);
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

/**
 * Format a Unix timestamp (seconds) into a human-readable date string.
 */
export function formatDate(timestamp: number): string {
  if (!timestamp || timestamp < 0) {
    return 'Unknown';
  }

  try {
    return new Date(timestamp * 1000).toLocaleDateString(undefined, {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return 'Invalid date';
  }
}

/**
 * Extract the filename from a full file path.
 */
export function getFileName(path: string): string {
  if (!path) return '';

  const parts = path.split(PATH_SEP);
  const fileName = parts[parts.length - 1];
  return fileName || path;
}
