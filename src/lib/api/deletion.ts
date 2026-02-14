import { invoke } from '@tauri-apps/api/core';
import type { DeletionRequest, DeleteFilesResponse, DeletionRecord } from '$lib/types';

interface DeleteFilesRequestPayload {
  files: DeletionRequest[];
  kept_paths: Record<string, string>;
  group_ids: Record<string, number>;
}

export async function deleteFiles(
  request: DeleteFilesRequestPayload
): Promise<DeleteFilesResponse> {
  return invoke<DeleteFilesResponse>('delete_files', { request });
}

export async function getDeletionHistorySummary(): Promise<[number, number]> {
  return invoke<[number, number]>('get_deletion_history_summary');
}

export async function getDeletionHistory(limit: number, offset: number): Promise<DeletionRecord[]> {
  return invoke<DeletionRecord[]>('get_deletion_history', { limit, offset });
}
