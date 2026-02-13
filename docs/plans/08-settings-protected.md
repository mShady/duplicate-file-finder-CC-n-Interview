# File 08: Settings & Protected Folders

## Overview

This file covers implementing the settings UI, theme switching, parallelism configuration, and protected folders management.

## Prerequisites

- Completed Files 01-07

---

## Phase 8.1: Create Settings Store

### Overview

Create a frontend store for settings management with **automatic restoration on app launch**.

### Key Behavior: App Launch Restoration

The settings store MUST:

1. **Load all settings on app initialization** - Before the main UI renders
2. **Restore last scan paths** - So users see their previous scan locations immediately
3. **Apply theme preference** - Match user's saved theme choice
4. **Restore parallelism setting** - For consistent scan performance

This is critical for UX: users should see their last scan settings immediately when opening the app.

### Changes Required

**File**: `src/lib/stores/settings.ts`

```typescript
import { invoke } from '@tauri-apps/api/core';

export interface AppSettings {
  theme: 'system' | 'light' | 'dark';
  parallelism: 'light' | 'normal' | 'aggressive';
  lastScanPaths: string[];
}

const defaultSettings: AppSettings = {
  theme: 'system',
  parallelism: 'normal',
  lastScanPaths: [],
};

class SettingsStore {
  private settings = $state<AppSettings>(defaultSettings);
  private loaded = $state(false);

  async load() {
    try {
      const allSettings = await invoke<{ key: string; value: string }[]>('get_all_settings');
      for (const setting of allSettings) {
        if (setting.key === 'theme') {
          this.settings.theme = setting.value as AppSettings['theme'];
        } else if (setting.key === 'parallelism') {
          this.settings.parallelism = setting.value as AppSettings['parallelism'];
        } else if (setting.key === 'last_scan_paths') {
          this.settings.lastScanPaths = JSON.parse(setting.value);
        }
      }
      this.loaded = true;
      this.applyTheme();
    } catch (e) {
      console.error('Failed to load settings:', e);
    }
  }

  async set<K extends keyof AppSettings>(key: K, value: AppSettings[K]) {
    this.settings[key] = value;
    const dbKey = key === 'lastScanPaths' ? 'last_scan_paths' : key;
    const dbValue = typeof value === 'object' ? JSON.stringify(value) : String(value);
    await invoke('set_setting', { key: dbKey, value: dbValue });

    if (key === 'theme') {
      this.applyTheme();
    }
  }

  get current() {
    return this.settings;
  }

  private applyTheme() {
    const theme = this.settings.theme;
    document.documentElement.setAttribute('data-theme', theme);
  }
}

export const settingsStore = new SettingsStore();
```

### Success Criteria

#### Automated Verification

- [ ] `npm run check` passes

#### Manual Verification

- [ ] Settings load correctly on app startup
- [ ] **Last scan paths are restored** when app opens (verify in FolderPicker component)
- [ ] Theme is applied immediately on app launch
- [ ] Parallelism setting persists across app restarts
- [ ] Setting changes are saved immediately to database

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 8.2: Create Settings Panel

### Overview

Create the settings UI panel.

### Changes Required

**File**: `src/lib/components/SettingsPanel.svelte`

```svelte
<script lang="ts">
  import { settingsStore } from '$lib/stores/settings';

  let settings = $derived(settingsStore.current);

  function handleThemeChange(e: Event) {
    const value = (e.target as HTMLSelectElement).value as 'system' | 'light' | 'dark';
    settingsStore.set('theme', value);
  }

  function handleParallelismChange(e: Event) {
    const value = (e.target as HTMLSelectElement).value as 'light' | 'normal' | 'aggressive';
    settingsStore.set('parallelism', value);
  }
</script>

<div class="settings-panel">
  <h2>Settings</h2>

  <div class="setting-group">
    <label for="theme">Theme</label>
    <select id="theme" value={settings.theme} onchange={handleThemeChange}>
      <option value="system">System Default</option>
      <option value="light">Light</option>
      <option value="dark">Dark</option>
    </select>
    <p class="hint">Follow your system's dark/light mode setting</p>
  </div>

  <div class="setting-group">
    <label for="parallelism">CPU Usage</label>
    <select id="parallelism" value={settings.parallelism} onchange={handleParallelismChange}>
      <option value="light">Light (1-2 threads)</option>
      <option value="normal">Normal (~75% CPU)</option>
      <option value="aggressive">Aggressive (All cores)</option>
    </select>
    <p class="hint">Higher settings scan faster but use more system resources</p>
  </div>
</div>

<style>
  .settings-panel {
    padding: 1.5rem;
    max-width: 500px;
  }

  h2 {
    margin: 0 0 1.5rem;
  }

  .setting-group {
    margin-bottom: 1.5rem;
  }

  label {
    display: block;
    font-weight: 500;
    margin-bottom: 0.5rem;
  }

  select {
    width: 100%;
    padding: 0.5rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--background);
    color: var(--text);
  }

  .hint {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-top: 0.25rem;
  }
</style>
```

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 8.3: Create Protected Folders Manager

### Overview

Create UI for managing protected folders.

### Changes Required

**File**: `src/lib/components/ProtectedFolders.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import type { ProtectedFolder } from '$lib/types';

  let folders = $state<ProtectedFolder[]>([]);
  let loading = $state(true);

  async function loadFolders() {
    try {
      folders = await invoke<ProtectedFolder[]>('get_protected_folders');
    } catch (e) {
      console.error('Failed to load protected folders:', e);
    } finally {
      loading = false;
    }
  }

  async function addFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: 'Select folder to protect',
      });

      if (selected) {
        await invoke('add_protected_folder', { path: selected });
        await loadFolders();
      }
    } catch (e) {
      console.error('Failed to add protected folder:', e);
    }
  }

  async function removeFolder(id: number) {
    try {
      await invoke('remove_protected_folder', { id });
      await loadFolders();
    } catch (e) {
      console.error('Failed to remove protected folder:', e);
    }
  }

  loadFolders();
</script>

<div class="protected-folders">
  <div class="header">
    <h3>Protected Folders</h3>
    <button class="add-btn" onclick={addFolder}>Add Folder</button>
  </div>

  <p class="description">Files in protected folders cannot be selected for deletion.</p>

  {#if loading}
    <div class="loading">Loading...</div>
  {:else if folders.length === 0}
    <div class="empty">No protected folders configured</div>
  {:else}
    <ul class="folder-list">
      {#each folders as folder (folder.id)}
        <li>
          <span class="path">{folder.path}</span>
          <button class="remove-btn" onclick={() => removeFolder(folder.id)}>Remove</button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .protected-folders {
    padding: 1rem;
    background: var(--surface);
    border-radius: 8px;
  }

  .header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  h3 {
    margin: 0;
  }

  .add-btn {
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .description {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin-bottom: 1rem;
  }

  .folder-list {
    list-style: none;
    margin: 0;
    padding: 0;
  }

  .folder-list li {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    background: var(--background);
    border-radius: 4px;
    margin-bottom: 0.5rem;
  }

  .path {
    font-family: var(--font-mono);
    font-size: 0.85rem;
  }

  .remove-btn {
    padding: 0.25rem 0.5rem;
    background: transparent;
    border: 1px solid var(--error);
    color: var(--error);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.8rem;
  }

  .empty,
  .loading {
    text-align: center;
    color: var(--text-secondary);
    padding: 2rem;
  }
</style>
```

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 8.4: Add Dialog Plugin

### Overview

Add Tauri dialog plugin for folder selection.

### Changes Required

```bash
npm run tauri add dialog
```

Update capabilities to include dialog permissions.

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 8.5: Create Full Settings View

### Overview

Combine settings and protected folders into settings view.

### Changes Required

**File**: `src/lib/components/SettingsView.svelte`

Combine SettingsPanel and ProtectedFolders.

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 8.6: Tests

Add tests for settings and protected folders.

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## End of File 08

After completing all phases:

- Theme switching (system/light/dark)
- Parallelism configuration
- Protected folders management
- Settings persistence

**Next**: Proceed to [09-file-operations.md](./09-file-operations.md)
