import { invoke } from '@tauri-apps/api/core';
import type { DetectionResult, ScanProgress, ScanRequest } from '$lib/types';

interface ScanResponse {
  session_id: number;
  message: string;
}

export async function startScan(request: ScanRequest): Promise<ScanResponse> {
  return invoke<ScanResponse>('start_scan', { request });
}

export async function cancelScan(): Promise<void> {
  return invoke<void>('cancel_scan');
}

export async function getScanProgress(): Promise<ScanProgress | null> {
  return invoke<ScanProgress | null>('get_scan_progress');
}

export async function isScanning(): Promise<boolean> {
  return invoke<boolean>('is_scanning');
}

export async function getScanResults(): Promise<DetectionResult | null> {
  return invoke<DetectionResult | null>('get_scan_results');
}
