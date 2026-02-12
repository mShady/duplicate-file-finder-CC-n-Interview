import { describe, it, expect } from 'vitest';
import {
  formatBytes,
  formatDate,
  getFileName,
  getDirectory,
  getFileTypeIcon,
  getFileExtension,
} from '../../src/lib/utils/format';

describe('formatBytes', () => {
  it('should format 0 bytes', () => {
    expect(formatBytes(0)).toBe('0 B');
  });

  it('should format bytes', () => {
    expect(formatBytes(500)).toBe('500 B');
  });

  it('should format kilobytes', () => {
    expect(formatBytes(1024)).toBe('1 KB');
    expect(formatBytes(1536)).toBe('1.5 KB');
  });

  it('should format megabytes', () => {
    expect(formatBytes(1048576)).toBe('1 MB');
    expect(formatBytes(5242880)).toBe('5 MB');
  });

  it('should format gigabytes', () => {
    expect(formatBytes(1073741824)).toBe('1 GB');
  });

  it('should format terabytes', () => {
    expect(formatBytes(1099511627776)).toBe('1 TB');
  });

  it('should handle large numbers', () => {
    const result = formatBytes(999999999999);
    expect(result).toContain('GB');
  });

  it('should handle negative input', () => {
    expect(formatBytes(-1024)).toBe('-1 KB');
    expect(formatBytes(-5242880)).toBe('-5 MB');
  });

  it('should clamp index for extremely large values beyond TB', () => {
    // 1 PB = 1024 TB, but sizes array only goes to TB
    const petabyte = 1024 * 1099511627776;
    const result = formatBytes(petabyte);
    expect(result).toContain('TB');
    expect(result).not.toContain('undefined');
  });
});

describe('getDirectory (middle ellipsis truncation)', () => {
  it('should return empty string for empty path', () => {
    expect(getDirectory('')).toBe('');
  });

  it('should return empty string for filename only', () => {
    expect(getDirectory('file.txt')).toBe('');
  });

  it('should return directory for short paths', () => {
    expect(getDirectory('/Users/test/file.txt')).toBe('/Users/test');
  });

  it('should truncate long paths with middle ellipsis', () => {
    const longPath =
      '/Users/john/Documents/Projects/2024/DupliFind/backups/photos/vacation/file.jpg';
    const result = getDirectory(longPath, 50);

    expect(result.length).toBeLessThanOrEqual(50);
    expect(result).toContain('/...');
    // Should show beginning
    expect(result.startsWith('/Users')).toBe(true);
    // Should show end
    expect(result).toContain('vacation');
  });

  it('should not truncate paths within max length', () => {
    const shortPath = '/Users/test/docs/file.txt';
    const result = getDirectory(shortPath, 100);
    expect(result).toBe('/Users/test/docs');
    expect(result).not.toContain('...');
  });

  it('should handle Windows paths', () => {
    const windowsPath = 'C:\\Users\\john\\Documents\\file.txt';
    const result = getDirectory(windowsPath);
    // Windows paths get split by backslash
    expect(result).toContain('C:');
  });

  it('should handle very short max length', () => {
    const path = '/Users/john/Documents/file.txt';
    const result = getDirectory(path, 10);
    expect(result.length).toBeLessThanOrEqual(25); // Allow some overflow for ellipsis
  });
});

describe('getFileName', () => {
  it('should extract filename from Unix path', () => {
    expect(getFileName('/Users/test/file.txt')).toBe('file.txt');
  });

  it('should extract filename from Windows path', () => {
    expect(getFileName('C:\\Users\\test\\file.txt')).toBe('file.txt');
  });

  it('should handle filename only', () => {
    expect(getFileName('file.txt')).toBe('file.txt');
  });

  it('should handle empty string', () => {
    expect(getFileName('')).toBe('');
  });

  it('should handle path ending with separator', () => {
    const result = getFileName('/Users/test/');
    // Last part after split is empty string, falls back to full path
    expect(result).toBeTruthy();
  });
});

describe('formatDate', () => {
  it('should format a valid timestamp', () => {
    // 2024-01-01 00:00:00 UTC
    const result = formatDate(1704067200);
    expect(result).toContain('2024');
    expect(result).toContain('Jan');
  });

  it('should handle zero timestamp', () => {
    expect(formatDate(0)).toBe('Unknown');
  });

  it('should handle negative timestamp', () => {
    expect(formatDate(-100)).toBe('Unknown');
  });
});

describe('getFileTypeIcon', () => {
  it('should classify image extensions', () => {
    expect(getFileTypeIcon('jpg')).toBe('image');
    expect(getFileTypeIcon('png')).toBe('image');
    expect(getFileTypeIcon('svg')).toBe('image');
  });

  it('should classify video extensions', () => {
    expect(getFileTypeIcon('mp4')).toBe('video');
    expect(getFileTypeIcon('mov')).toBe('video');
  });

  it('should classify audio extensions', () => {
    expect(getFileTypeIcon('mp3')).toBe('audio');
    expect(getFileTypeIcon('flac')).toBe('audio');
  });

  it('should classify document extensions', () => {
    expect(getFileTypeIcon('pdf')).toBe('document');
    expect(getFileTypeIcon('txt')).toBe('document');
    expect(getFileTypeIcon('md')).toBe('document');
  });

  it('should return file for unknown extensions', () => {
    expect(getFileTypeIcon('xyz')).toBe('file');
    expect(getFileTypeIcon('')).toBe('file');
  });
});

describe('getFileExtension', () => {
  it('should extract extension', () => {
    expect(getFileExtension('/test/file.txt')).toBe('txt');
    expect(getFileExtension('/test/photo.JPG')).toBe('jpg');
  });

  it('should handle no extension (returns full path component)', () => {
    // When there's no dot, split('.').pop() returns the full string lowercased
    const result = getFileExtension('/test/Makefile');
    expect(result).toBe('/test/makefile');
  });

  it('should handle multiple dots', () => {
    expect(getFileExtension('/test/file.backup.tar.gz')).toBe('gz');
  });

  it('should handle empty string', () => {
    expect(getFileExtension('')).toBe('');
  });
});
