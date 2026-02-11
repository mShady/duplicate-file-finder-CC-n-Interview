import { describe, it, expect } from 'vitest';
import { formatBytes } from '../../src/lib/utils/format';

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
