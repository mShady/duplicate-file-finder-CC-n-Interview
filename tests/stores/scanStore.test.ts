import { describe, it, expect, vi, beforeEach } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import { scanStore } from '$lib/stores/scanStore.svelte';
import type { DetectionResult } from '$lib/types';

// Helper to create mock event listeners that can be triggered
function createMockListeners() {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const listeners: Record<string, (event: any) => void> = {};

  vi.mocked(listen).mockImplementation((eventName, handler) => {
    listeners[eventName as string] = handler;
    return Promise.resolve(() => {
      delete listeners[eventName as string];
    });
  });

  return {
    emit(eventName: string, payload: unknown) {
      if (listeners[eventName]) {
        listeners[eventName]({ event: eventName, id: 0, payload });
      }
    },
    getRegistered() {
      return Object.keys(listeners);
    },
  };
}

describe('scanStore', () => {
  beforeEach(() => {
    scanStore.cleanup();
    scanStore.reset();
  });

  describe('initial state', () => {
    it('should have idle initial state', () => {
      expect(scanStore.isScanning).toBe(false);
      expect(scanStore.phase).toBe('idle');
      expect(scanStore.progress).toBeNull();
      expect(scanStore.scanResult).toBeNull();
      expect(scanStore.detectionResult).toBeNull();
      expect(scanStore.error).toBeNull();
    });
  });

  describe('init()', () => {
    it('should register event listeners', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      const registered = mockListeners.getRegistered();
      expect(registered).toContain('scan-progress');
      expect(registered).toContain('scan-phase');
      expect(registered).toContain('scan-results');
      expect(registered).toContain('scan-complete');
      expect(registered).toContain('scan-error');
    });

    it('should prevent double initialization', async () => {
      createMockListeners();
      await scanStore.init();

      // Second init should be a no-op (no extra listeners registered)
      const listenCallCount = vi.mocked(listen).mock.calls.length;
      await scanStore.init();
      expect(vi.mocked(listen).mock.calls.length).toBe(listenCallCount);
    });

    it('should accept navigation callbacks', async () => {
      const mockListeners = createMockListeners();
      const onComplete = vi.fn();
      const onError = vi.fn();
      await scanStore.init({ onComplete, onError });

      mockListeners.emit('scan-complete', {
        session_id: 1,
        total_files: 10,
        total_bytes: 1000,
        duplicate_groups: 1,
        duplicate_files: 2,
        wasted_space: 500,
        duration_ms: 100,
      });
      expect(onComplete).toHaveBeenCalledOnce();

      scanStore.cleanup();
      scanStore.reset();
      const mockListeners2 = createMockListeners();
      const onError2 = vi.fn();
      await scanStore.init({ onError: onError2 });

      mockListeners2.emit('scan-error', { session_id: 1, error: 'fail' });
      expect(onError2).toHaveBeenCalledOnce();
    });
  });

  describe('startScan()', () => {
    it('should set scanning state', () => {
      scanStore.startScan();

      expect(scanStore.isScanning).toBe(true);
      expect(scanStore.phase).toBe('collecting');
      expect(scanStore.progress).toBeNull();
      expect(scanStore.scanResult).toBeNull();
      expect(scanStore.detectionResult).toBeNull();
      expect(scanStore.error).toBeNull();
    });
  });

  describe('cancelledScan()', () => {
    it('should reset scanning flags', () => {
      scanStore.startScan();
      scanStore.cancelledScan();

      expect(scanStore.isScanning).toBe(false);
      expect(scanStore.phase).toBe('idle');
    });
  });

  describe('handleScanError()', () => {
    it('should set error and stop scanning', () => {
      scanStore.startScan();
      scanStore.handleScanError('Something went wrong');

      expect(scanStore.isScanning).toBe(false);
      expect(scanStore.phase).toBe('idle');
      expect(scanStore.error).toBe('Something went wrong');
    });
  });

  describe('reset()', () => {
    it('should reset to initial state', () => {
      scanStore.startScan();
      scanStore.reset();

      expect(scanStore.isScanning).toBe(false);
      expect(scanStore.phase).toBe('idle');
      expect(scanStore.progress).toBeNull();
      expect(scanStore.scanResult).toBeNull();
      expect(scanStore.detectionResult).toBeNull();
      expect(scanStore.error).toBeNull();
    });
  });

  describe('setters', () => {
    it('should allow setting detectionResult directly', () => {
      const result: DetectionResult = {
        groups: [],
        duplicate_count: 0,
        total_wasted_space: 0,
        unique_files: 5,
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

      scanStore.detectionResult = result;
      expect(scanStore.detectionResult).toEqual(result);
    });

    it('should allow setting error directly', () => {
      scanStore.error = 'test error';
      expect(scanStore.error).toBe('test error');

      scanStore.error = null;
      expect(scanStore.error).toBeNull();
    });
  });

  describe('event handling', () => {
    it('should update progress on scan-progress event', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      const progress = {
        total_files: 100,
        processed_files: 50,
        total_bytes: 5000,
        current_path: '/test/file.txt',
        skipped_files: 2,
        estimated_total: null,
      };

      mockListeners.emit('scan-progress', progress);
      expect(scanStore.progress).toEqual(progress);
    });

    it('should update phase on scan-phase event', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      mockListeners.emit('scan-phase', {
        phase: 'partial_hashing',
        message: 'Hashing files...',
      });

      expect(scanStore.phase).toBe('partial_hashing');
    });

    it('should set detectionResult on scan-results event', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      const result: DetectionResult = {
        groups: [],
        duplicate_count: 0,
        total_wasted_space: 0,
        unique_files: 5,
        stats: {
          size_groups: 1,
          size_candidates: 2,
          partial_hashes: 2,
          full_hashes: 2,
          size_grouping_ms: 10,
          partial_hashing_ms: 20,
          full_hashing_ms: 30,
        },
      };

      mockListeners.emit('scan-results', result);
      expect(scanStore.detectionResult).toEqual(result);
    });

    it('should handle scan completion', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();
      scanStore.startScan();

      const complete = {
        session_id: 1,
        total_files: 100,
        total_bytes: 50000,
        duplicate_groups: 5,
        duplicate_files: 10,
        wasted_space: 25000,
        duration_ms: 5000,
      };

      mockListeners.emit('scan-complete', complete);

      expect(scanStore.isScanning).toBe(false);
      expect(scanStore.phase).toBe('complete');
      expect(scanStore.scanResult).toEqual(complete);
    });

    it('should handle scan error', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();
      scanStore.startScan();

      mockListeners.emit('scan-error', {
        session_id: 1,
        error: 'Permission denied',
      });

      expect(scanStore.isScanning).toBe(false);
      expect(scanStore.phase).toBe('idle');
      expect(scanStore.error).toBe('Permission denied');
    });
  });

  describe('cleanup()', () => {
    it('should clean up listeners so events no longer update state', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      scanStore.cleanup();

      // After cleanup, emitting events should not update state
      scanStore.startScan();
      mockListeners.emit('scan-error', {
        session_id: 1,
        error: 'Should not be received',
      });

      // State should still be from startScan, not from the error event
      expect(scanStore.isScanning).toBe(true);
      expect(scanStore.error).toBeNull();
    });

    it('should allow reinit after cleanup', async () => {
      createMockListeners();
      await scanStore.init();
      scanStore.cleanup();

      // Should not throw and should register listeners again
      const mockListeners = createMockListeners();
      await scanStore.init();

      const registered = mockListeners.getRegistered();
      expect(registered).toContain('scan-progress');
    });
  });
});
