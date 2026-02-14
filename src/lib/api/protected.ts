import { invoke } from '@tauri-apps/api/core';
import type { ProtectedFolder } from '$lib/types';

export async function addProtectedFolder(path: string): Promise<number> {
  return invoke<number>('add_protected_folder', { path });
}

export async function removeProtectedFolder(id: number): Promise<boolean> {
  return invoke<boolean>('remove_protected_folder', { id });
}

export async function getProtectedFolders(): Promise<ProtectedFolder[]> {
  return invoke<ProtectedFolder[]>('get_protected_folders');
}

export async function isPathProtected(path: string): Promise<boolean> {
  return invoke<boolean>('is_path_protected', { path });
}
