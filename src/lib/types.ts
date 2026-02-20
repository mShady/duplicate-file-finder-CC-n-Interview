// Shared TypeScript types for DupliFind

export interface DuplicateFile {
  path: string;
  size: number;
  created_at: number;
  modified_at: number;
  is_original: boolean;
}

export interface DuplicateGroup {
  id: number;
  hash: string;
  file_size: number;
  files: DuplicateFile[];
  wasted_space: number;
}

export interface DetectionResult {
  groups: DuplicateGroup[];
  duplicate_count: number;
  total_wasted_space: number;
  unique_files: number;
  stats: DetectionStats;
}

export interface DetectionStats {
  size_groups: number;
  size_candidates: number;
  partial_hashes: number;
  full_hashes: number;
  size_grouping_ms: number;
  partial_hashing_ms: number;
  full_hashing_ms: number;
}

export interface ScanProgress {
  total_files: number;
  processed_files: number;
  total_bytes: number;
  current_path: string | null;
  skipped_files: number;
  estimated_total: number | null;
  started_at_ms?: number;
  estimated_time_remaining_ms?: number;
}

export interface ScanComplete {
  session_id: number;
  total_files: number;
  total_bytes: number;
  duplicate_groups: number;
  duplicate_files: number;
  wasted_space: number;
  duration_ms: number;
}

export interface ScanPhaseEvent {
  phase: 'collecting' | 'partial_hashing' | 'full_hashing' | 'storing' | 'complete';
  message: string;
}

export interface DetectionProgressEvent {
  partial_hashes: number;
  full_hashes: number;
  groups_found: number;
}

export interface ScanErrorEvent {
  session_id: number;
  error: string;
}

export interface Setting {
  key: string;
  value: string;
}

export interface ProtectedFolder {
  id: number;
  path: string;
  added_at: number;
}

// Deletion types

export interface DeletionProgressEvent {
  current: number;
  total: number;
  current_path: string | null;
}

export interface DeletionRequest {
  path: string;
  expected_hash: string;
  size: number;
}

export interface DeletionResult {
  path: string;
  success: boolean;
  error: string | null;
  size: number;
}

export interface BatchDeletionResult {
  successful: DeletionResult[];
  failed: DeletionResult[];
  total_freed: number;
}

export interface DeleteFilesResponse {
  result: BatchDeletionResult;
  message: string;
}

export interface DeletionRecord {
  id: number;
  file_path: string;
  file_size: number;
  file_hash: string;
  deleted_at: number;
  group_id: number | null;
  kept_path: string | null;
}

export interface ScanRequest {
  paths: string[];
  parallelism?: string;
}

export type FileType = 'images' | 'videos' | 'documents' | 'audio' | 'other' | 'all';

export interface FilterState {
  fileType: FileType;
  minSize: number | null;
  maxSize: number | null;
  searchQuery: string;
}
