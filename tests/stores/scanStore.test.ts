import { describe, it, expect, vi, beforeEach } from 'vitest';
import { listen } from '@tauri-apps/api/event';
import { get } from 'svelte/store';
import {
  scanStore,
  isScanning,
  currentPhase,
  duplicateGroups,
  totalWastedSpace,
} from '$lib/stores/scanStore';
import type { DuplicateGroup, DetectionResult } from '$lib/types';

// Helper to create mock event listeners that can be triggered
function createMockListeners() {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const listeners: Record<string, (event: any) => void> = {};

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  vi.mocked(listen).mockImplementation(async (eventName: any, handler: any) => {
    listeners[eventName] = handler;
    return () => {
      delete listeners[eventName];
    };
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

function createMockGroup(overrides: Partial<DuplicateGroup> = {}): DuplicateGroup {
  return {
    id: 1,
    hash: 'abc123',
    file_size: 1024,
    files: [
      {
        path: '/test/file1.txt',
        size: 1024,
        created_at: 1000000,
        modified_at: 1000100,
        is_original: true,
      },
      {
        path: '/test/file2.txt',
        size: 1024,
        created_at: 1000200,
        modified_at: 1000300,
        is_original: false,
      },
    ],
    wasted_space: 1024,
    ...overrides,
  };
}

describe('scanStore', () => {
  beforeEach(() => {
    scanStore.cleanup();
    scanStore.reset();
  });

  describe('initial state', () => {
    it('should have idle initial state', () => {
      const state = get(scanStore);
      expect(state.isScanning).toBe(false);
      expect(state.phase).toBe('idle');
      expect(state.phaseMessage).toBe('');
      expect(state.progress).toBeNull();
      expect(state.detectionProgress).toBeNull();
      expect(state.liveGroups).toEqual([]);
      expect(state.finalResult).toBeNull();
      expect(state.scanComplete).toBeNull();
      expect(state.error).toBeNull();
    });
  });

  describe('init()', () => {
    it('should register event listeners', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      const registered = mockListeners.getRegistered();
      expect(registered).toContain('scan-progress');
      expect(registered).toContain('scan-phase');
      expect(registered).toContain('duplicate-found');
      expect(registered).toContain('detection-progress');
      expect(registered).toContain('scan-results');
      expect(registered).toContain('scan-complete');
      expect(registered).toContain('scan-error');
    });

    it('should prevent double initialization', async () => {
      createMockListeners();
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});

      await scanStore.init();
      await scanStore.init();

      expect(warnSpy).toHaveBeenCalledWith('scanStore already initialized');
      warnSpy.mockRestore();
    });
  });

  describe('startScan()', () => {
    it('should set scanning state', () => {
      scanStore.startScan();

      const state = get(scanStore);
      expect(state.isScanning).toBe(true);
      expect(state.phase).toBe('collecting');
      expect(state.phaseMessage).toBe('Starting scan...');
      expect(state.progress).toBeNull();
      expect(state.liveGroups).toEqual([]);
      expect(state.finalResult).toBeNull();
      expect(state.error).toBeNull();
    });

    it('should clear previous results', () => {
      // Set some state first
      scanStore.startScan();
      const state = get(scanStore);
      expect(state.finalResult).toBeNull();
      expect(state.scanComplete).toBeNull();
      expect(state.liveGroups).toEqual([]);
    });
  });

  describe('reset()', () => {
    it('should reset to initial state', () => {
      scanStore.startScan();
      scanStore.reset();

      const state = get(scanStore);
      expect(state.isScanning).toBe(false);
      expect(state.phase).toBe('idle');
      expect(state.liveGroups).toEqual([]);
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

      const state = get(scanStore);
      expect(state.progress).toEqual(progress);
    });

    it('should update detection progress on detection-progress event', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      const detectionProgress = {
        partial_hashes: 50,
        full_hashes: 20,
        groups_found: 5,
      };

      mockListeners.emit('detection-progress', detectionProgress);

      const state = get(scanStore);
      expect(state.detectionProgress).toEqual(detectionProgress);
    });

    it('should update phase on scan-phase event', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      mockListeners.emit('scan-phase', {
        phase: 'partial_hashing',
        message: 'Hashing files...',
      });

      const state = get(scanStore);
      expect(state.phase).toBe('partial_hashing');
      expect(state.phaseMessage).toBe('Hashing files...');
    });

    it('should add live groups on duplicate-found event', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      const group = createMockGroup({ id: 1, wasted_space: 500 });
      mockListeners.emit('duplicate-found', group);

      const state = get(scanStore);
      expect(state.liveGroups).toHaveLength(1);
      expect(state.liveGroups[0].id).toBe(1);
    });

    it('should sort live groups by wasted space descending', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      const smallGroup = createMockGroup({ id: 1, wasted_space: 100 });
      const largeGroup = createMockGroup({ id: 2, wasted_space: 1000 });

      mockListeners.emit('duplicate-found', smallGroup);
      mockListeners.emit('duplicate-found', largeGroup);

      const state = get(scanStore);
      expect(state.liveGroups).toHaveLength(2);
      expect(state.liveGroups[0].wasted_space).toBe(1000);
      expect(state.liveGroups[1].wasted_space).toBe(100);
    });

    it('should replace live groups with final results', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      // Add a live group first
      mockListeners.emit('duplicate-found', createMockGroup({ id: 1 }));

      // Then emit final results
      const result: DetectionResult = {
        groups: [createMockGroup({ id: 10 }), createMockGroup({ id: 20 })],
        duplicate_count: 2,
        total_wasted_space: 2048,
        unique_files: 5,
        stats: {
          size_groups: 3,
          size_candidates: 4,
          partial_hashes: 4,
          full_hashes: 2,
          size_grouping_ms: 10,
          partial_hashing_ms: 20,
          full_hashing_ms: 30,
        },
      };

      mockListeners.emit('scan-results', result);

      const state = get(scanStore);
      expect(state.finalResult).toEqual(result);
      expect(state.liveGroups).toHaveLength(2);
      expect(state.liveGroups[0].id).toBe(10);
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

      const state = get(scanStore);
      expect(state.isScanning).toBe(false);
      expect(state.phase).toBe('complete');
      expect(state.scanComplete).toEqual(complete);
    });

    it('should handle scan error', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();
      scanStore.startScan();

      mockListeners.emit('scan-error', {
        session_id: 1,
        error: 'Permission denied',
      });

      const state = get(scanStore);
      expect(state.isScanning).toBe(false);
      expect(state.phase).toBe('error');
      expect(state.error).toBe('Permission denied');
    });
  });

  describe('cleanup()', () => {
    it('should clean up listeners', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      scanStore.cleanup();

      // After cleanup, emitting events should not update state
      scanStore.startScan();
      mockListeners.emit('scan-error', {
        session_id: 1,
        error: 'Should not be received',
      });

      const state = get(scanStore);
      // State should still be from startScan, not from the error event
      expect(state.isScanning).toBe(true);
      expect(state.error).toBeNull();
    });

    it('should allow reinit after cleanup', async () => {
      createMockListeners();
      await scanStore.init();
      scanStore.cleanup();

      // Should not warn about double init
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {});
      await scanStore.init();
      expect(warnSpy).not.toHaveBeenCalled();
      warnSpy.mockRestore();
    });
  });
});

describe('derived stores', () => {
  beforeEach(() => {
    scanStore.cleanup();
    scanStore.reset();
  });

  describe('isScanning', () => {
    it('should reflect scanning state', () => {
      expect(get(isScanning)).toBe(false);

      scanStore.startScan();
      expect(get(isScanning)).toBe(true);

      scanStore.reset();
      expect(get(isScanning)).toBe(false);
    });
  });

  describe('currentPhase', () => {
    it('should reflect current phase', () => {
      expect(get(currentPhase)).toBe('idle');

      scanStore.startScan();
      expect(get(currentPhase)).toBe('collecting');
    });
  });

  describe('duplicateGroups', () => {
    it('should return empty array initially', () => {
      expect(get(duplicateGroups)).toEqual([]);
    });

    it('should return live groups when no final result', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      mockListeners.emit('duplicate-found', createMockGroup({ id: 1 }));

      expect(get(duplicateGroups)).toHaveLength(1);
    });

    it('should prefer final results over live groups', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      // Add live group
      mockListeners.emit('duplicate-found', createMockGroup({ id: 1 }));

      // Add final results
      const result: DetectionResult = {
        groups: [createMockGroup({ id: 10 }), createMockGroup({ id: 20 })],
        duplicate_count: 2,
        total_wasted_space: 2048,
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

      const groups = get(duplicateGroups);
      expect(groups).toHaveLength(2);
      expect(groups[0].id).toBe(10);
    });
  });

  describe('totalWastedSpace', () => {
    it('should return 0 initially', () => {
      expect(get(totalWastedSpace)).toBe(0);
    });

    it('should sum live group wasted space', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      mockListeners.emit('duplicate-found', createMockGroup({ id: 1, wasted_space: 100 }));
      mockListeners.emit('duplicate-found', createMockGroup({ id: 2, wasted_space: 200 }));

      expect(get(totalWastedSpace)).toBe(300);
    });

    it('should use final result total when available', async () => {
      const mockListeners = createMockListeners();
      await scanStore.init();

      const result: DetectionResult = {
        groups: [],
        duplicate_count: 0,
        total_wasted_space: 5000,
        unique_files: 10,
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

      mockListeners.emit('scan-results', result);

      expect(get(totalWastedSpace)).toBe(5000);
    });
  });
});
