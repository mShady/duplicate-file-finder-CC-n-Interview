# File 09: File Operations & Context Menu

## Overview

This file covers implementing file operations (open, reveal, copy path) and the context menu for files in the results view.

## Prerequisites

- Completed Files 01-08

---

## Phase 9.1: Add Shell Plugin

### Overview
Add the Tauri shell plugin for opening files and revealing in Finder/Explorer.

### Changes Required

```bash
npm run tauri add shell
```

Update capabilities:
```json
{
  "permissions": [
    "shell:allow-open"
  ]
}
```

### Commit
Execute `/cl:commit`

---

## Phase 9.2: Create File Operations Commands

### Overview
Create Tauri commands for file operations.

### Changes Required

**File**: `src-tauri/src/commands/files.rs`

```rust
use std::path::Path;
use tauri_plugin_shell::ShellExt;

#[tauri::command]
pub async fn open_file(path: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    app_handle
        .shell()
        .open(&path, None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn reveal_in_folder(path: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let path = Path::new(&path);
    let folder = path.parent().unwrap_or(path);

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(&path)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(folder)
            .spawn()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn open_folder(path: String, app_handle: tauri::AppHandle) -> Result<(), String> {
    let path = Path::new(&path);
    let folder = path.parent().unwrap_or(path);

    app_handle
        .shell()
        .open(folder.to_string_lossy().as_ref(), None)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_file_info(path: String) -> Result<FileInfoResponse, String> {
    let path = Path::new(&path);
    let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;

    Ok(FileInfoResponse {
        path: path.display().to_string(),
        name: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        size: metadata.len(),
        is_file: metadata.is_file(),
        is_dir: metadata.is_dir(),
        created: metadata.created().ok().map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
        modified: metadata.modified().ok().map(|t| t.duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()),
    })
}

#[derive(serde::Serialize)]
pub struct FileInfoResponse {
    path: String,
    name: String,
    size: u64,
    is_file: bool,
    is_dir: bool,
    created: Option<u64>,
    modified: Option<u64>,
}
```

### Commit
Execute `/cl:commit`

---

## Phase 9.3: Create Context Menu Component

### Overview
Create a reusable context menu component.

### Changes Required

**File**: `src/lib/components/ContextMenu.svelte`

```svelte
<script lang="ts">
  interface MenuItem {
    label: string;
    action: () => void;
    icon?: string;
    separator?: boolean;
    disabled?: boolean;
  }

  interface Props {
    items: MenuItem[];
    x: number;
    y: number;
    onClose: () => void;
  }

  let { items, x, y, onClose }: Props = $props();

  function handleClick(item: MenuItem) {
    if (!item.disabled && !item.separator) {
      item.action();
      onClose();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      onClose();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} onclick={onClose} />

<div
  class="context-menu"
  style="left: {x}px; top: {y}px"
  onclick={(e) => e.stopPropagation()}
  role="menu"
>
  {#each items as item}
    {#if item.separator}
      <div class="separator"></div>
    {:else}
      <button
        class="menu-item"
        class:disabled={item.disabled}
        onclick={() => handleClick(item)}
        disabled={item.disabled}
        role="menuitem"
      >
        {#if item.icon}
          <span class="icon">{item.icon}</span>
        {/if}
        {item.label}
      </button>
    {/if}
  {/each}
</div>

<style>
  .context-menu {
    position: fixed;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
    min-width: 180px;
    padding: 0.5rem 0;
    z-index: 1000;
  }

  .menu-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 1rem;
    border: none;
    background: transparent;
    color: var(--text);
    text-align: left;
    cursor: pointer;
    font-size: 0.9rem;
  }

  .menu-item:hover:not(.disabled) {
    background: var(--background);
  }

  .menu-item.disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .icon {
    width: 1.25rem;
    text-align: center;
  }

  .separator {
    height: 1px;
    background: var(--border);
    margin: 0.5rem 0;
  }
</style>
```

### Commit
Execute `/cl:commit`

---

## Phase 9.4: Create File Context Menu

### Overview
Create the specific context menu for files with all required actions.

### Changes Required

**File**: `src/lib/components/FileContextMenu.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';
  import ContextMenu from './ContextMenu.svelte';

  interface Props {
    filePath: string;
    x: number;
    y: number;
    isProtected: boolean;
    isSelected: boolean;
    onClose: () => void;
    onMarkForDeletion: () => void;
  }

  let { filePath, x, y, isProtected, isSelected, onClose, onMarkForDeletion }: Props = $props();

  async function openFile() {
    try {
      await invoke('open_file', { path: filePath });
    } catch (e) {
      console.error('Failed to open file:', e);
    }
  }

  async function revealInFinder() {
    try {
      await invoke('reveal_in_folder', { path: filePath });
    } catch (e) {
      console.error('Failed to reveal file:', e);
    }
  }

  async function openFolder() {
    try {
      await invoke('open_folder', { path: filePath });
    } catch (e) {
      console.error('Failed to open folder:', e);
    }
  }

  async function copyPath() {
    try {
      await writeText(filePath);
    } catch (e) {
      console.error('Failed to copy path:', e);
    }
  }

  const items = [
    { label: 'Open', action: openFile, icon: '📄' },
    { label: 'Reveal in Finder', action: revealInFinder, icon: '📁' },
    { label: 'Open Containing Folder', action: openFolder, icon: '📂' },
    { separator: true },
    { label: 'Copy Path', action: copyPath, icon: '📋' },
    { separator: true },
    {
      label: isSelected ? 'Unmark for Deletion' : 'Mark for Deletion',
      action: onMarkForDeletion,
      icon: '🗑️',
      disabled: isProtected,
    },
  ];
</script>

<ContextMenu {items} {x} {y} {onClose} />
```

### Commit
Execute `/cl:commit`

---

## Phase 9.5: Add Clipboard Plugin

### Overview
Add clipboard plugin for copy path functionality.

### Changes Required

```bash
npm run tauri add clipboard-manager
```

### Commit
Execute `/cl:commit`

---

## Phase 9.6: Integrate Context Menu in Results

### Overview
Add right-click context menu to file items in the results view.

### Changes Required

Update FileDetailsPanel to handle right-click and show FileContextMenu.

### Commit
Execute `/cl:commit`

---

## End of File 09

After completing all phases:
- Open file in default app
- Reveal in Finder/Explorer
- Open containing folder
- Copy file path to clipboard
- Context menu with all actions
- Mark for deletion from context menu

**Next**: Proceed to [10-filtering-search.md](./10-filtering-search.md)
