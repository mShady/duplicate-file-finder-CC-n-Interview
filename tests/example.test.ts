import { describe, it, expect, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

describe('Tauri API Mock', () => {
  it('should mock invoke correctly', async () => {
    const mockInvoke = vi.mocked(invoke);
    mockInvoke.mockResolvedValue('Hello, Test! Welcome to DupliFind.');

    const result = await invoke('greet', { name: 'Test' });

    expect(result).toBe('Hello, Test! Welcome to DupliFind.');
    expect(mockInvoke).toHaveBeenCalledWith('greet', { name: 'Test' });
  });
});

describe('Basic Tests', () => {
  it('should pass a simple test', () => {
    expect(1 + 1).toBe(2);
  });

  it('should handle string operations', () => {
    const str = 'DupliFind';
    expect(str.toLowerCase()).toBe('duplifind');
  });
});
