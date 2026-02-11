import { describe, it, expect } from 'vitest';
import type {
  DuplicateFile,
  DuplicateGroup,
  DetectionResult,
  DetectionStats,
  ScanProgress,
  ScanComplete,
  ScanPhaseEvent,
  DetectionProgressEvent,
  ScanErrorEvent,
  FilterState,
  FileType,
} from '$lib/types';

describe('TypeScript types', () => {
  describe('DuplicateFile', () => {
    it('should match the expected structure', () => {
      const file: DuplicateFile = {
        path: '/test/file.txt',
        size: 1024,
        created_at: 1700000000,
        modified_at: 1700001000,
        is_original: true,
      };

      expect(file.path).toBe('/test/file.txt');
      expect(file.size).toBe(1024);
      expect(file.is_original).toBe(true);
    });
  });

  describe('DuplicateGroup', () => {
    it('should match the expected structure', () => {
      const group: DuplicateGroup = {
        id: 1,
        hash: 'abc123def456',
        file_size: 2048,
        files: [
          {
            path: '/test/a.txt',
            size: 2048,
            created_at: 1700000000,
            modified_at: 1700001000,
            is_original: true,
          },
          {
            path: '/test/b.txt',
            size: 2048,
            created_at: 1700002000,
            modified_at: 1700003000,
            is_original: false,
          },
        ],
        wasted_space: 2048,
      };

      expect(group.files).toHaveLength(2);
      expect(group.wasted_space).toBe(2048);
    });
  });

  describe('DetectionResult', () => {
    it('should match the expected structure with stats', () => {
      const stats: DetectionStats = {
        size_groups: 10,
        size_candidates: 20,
        partial_hashes: 15,
        full_hashes: 8,
        size_grouping_ms: 100,
        partial_hashing_ms: 200,
        full_hashing_ms: 300,
      };

      const result: DetectionResult = {
        groups: [],
        duplicate_count: 5,
        total_wasted_space: 10240,
        unique_files: 50,
        stats,
      };

      expect(result.stats.size_groups).toBe(10);
      expect(result.total_wasted_space).toBe(10240);
    });
  });

  describe('ScanProgress', () => {
    it('should support optional fields', () => {
      const progress: ScanProgress = {
        total_files: 100,
        processed_files: 50,
        total_bytes: 5000000,
        current_path: '/Users/test/Documents',
        skipped_files: 3,
        estimated_total: 200,
      };

      expect(progress.estimated_total).toBe(200);
      expect(progress.started_at_ms).toBeUndefined();
      expect(progress.estimated_time_remaining_ms).toBeUndefined();
    });

    it('should allow null current_path', () => {
      const progress: ScanProgress = {
        total_files: 0,
        processed_files: 0,
        total_bytes: 0,
        current_path: null,
        skipped_files: 0,
        estimated_total: null,
      };

      expect(progress.current_path).toBeNull();
    });
  });

  describe('ScanComplete', () => {
    it('should match the Rust ScanComplete struct', () => {
      const complete: ScanComplete = {
        session_id: 1,
        total_files: 1000,
        total_bytes: 5000000000,
        duplicate_groups: 50,
        duplicate_files: 200,
        wasted_space: 2000000000,
        duration_ms: 30000,
      };

      expect(complete.session_id).toBe(1);
      expect(complete.duration_ms).toBe(30000);
    });
  });

  describe('Event types', () => {
    it('ScanPhaseEvent should support all phases', () => {
      const phases: ScanPhaseEvent['phase'][] = [
        'collecting',
        'partial_hashing',
        'full_hashing',
        'storing',
        'complete',
      ];

      phases.forEach((phase) => {
        const event: ScanPhaseEvent = { phase, message: `Phase: ${phase}` };
        expect(event.phase).toBe(phase);
      });
    });

    it('DetectionProgressEvent should have all fields', () => {
      const progress: DetectionProgressEvent = {
        partial_hashes: 50,
        full_hashes: 20,
        groups_found: 5,
      };

      expect(progress.groups_found).toBe(5);
    });

    it('ScanErrorEvent should have session_id and error', () => {
      const error: ScanErrorEvent = {
        session_id: 1,
        error: 'Permission denied',
      };

      expect(error.error).toBe('Permission denied');
    });
  });

  describe('FilterState', () => {
    it('should support all file types', () => {
      const types: FileType[] = ['images', 'videos', 'documents', 'audio', 'other', 'all'];

      types.forEach((type) => {
        const filter: FilterState = {
          fileType: type,
          minSize: null,
          maxSize: null,
          searchQuery: '',
        };
        expect(filter.fileType).toBe(type);
      });
    });

    it('should support size filters', () => {
      const filter: FilterState = {
        fileType: 'all',
        minSize: 1024,
        maxSize: 1048576,
        searchQuery: 'photo',
      };

      expect(filter.minSize).toBe(1024);
      expect(filter.maxSize).toBe(1048576);
    });
  });
});
