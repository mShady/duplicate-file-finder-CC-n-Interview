# File 11: Keyboard Navigation & Accessibility

## Overview

This file covers implementing full keyboard navigation, shortcuts, focus management, and accessibility features.

## Prerequisites

- Completed Files 01-10

---

## Phase 11.1: Create Keyboard Shortcuts Manager

### Overview

Create a centralized keyboard shortcuts manager.

### Changes Required

**File**: `src/lib/stores/shortcuts.ts`

```typescript
type ShortcutHandler = () => void;

interface Shortcut {
  key: string;
  ctrl?: boolean;
  meta?: boolean;
  shift?: boolean;
  alt?: boolean;
  handler: ShortcutHandler;
  description: string;
}

class ShortcutsManager {
  private shortcuts: Shortcut[] = [];
  private enabled = true;

  register(shortcut: Shortcut) {
    this.shortcuts.push(shortcut);
    return () => this.unregister(shortcut);
  }

  unregister(shortcut: Shortcut) {
    const index = this.shortcuts.indexOf(shortcut);
    if (index > -1) {
      this.shortcuts.splice(index, 1);
    }
  }

  handleKeydown(e: KeyboardEvent) {
    if (!this.enabled) return;

    // Don't trigger shortcuts when typing in inputs
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) {
      return;
    }

    for (const shortcut of this.shortcuts) {
      const ctrlMatch = shortcut.ctrl ? e.ctrlKey || e.metaKey : !e.ctrlKey;
      const metaMatch = shortcut.meta ? e.metaKey : true;
      const shiftMatch = shortcut.shift ? e.shiftKey : !e.shiftKey;
      const altMatch = shortcut.alt ? e.altKey : !e.altKey;
      const keyMatch = e.key.toLowerCase() === shortcut.key.toLowerCase();

      if (ctrlMatch && metaMatch && shiftMatch && altMatch && keyMatch) {
        e.preventDefault();
        shortcut.handler();
        return;
      }
    }
  }

  setEnabled(enabled: boolean) {
    this.enabled = enabled;
  }

  getAll(): Shortcut[] {
    return [...this.shortcuts];
  }
}

export const shortcuts = new ShortcutsManager();

// Default shortcuts
export const defaultShortcuts = {
  startScan: { key: 's', ctrl: true, description: 'Start scan' },
  pauseScan: { key: 'p', ctrl: true, description: 'Pause/Resume scan' },
  cancelScan: { key: 'Escape', description: 'Cancel scan' },
  selectAll: { key: 'a', ctrl: true, description: 'Select all (except original)' },
  deselectAll: { key: 'd', ctrl: true, description: 'Deselect all' },
  deleteSelected: { key: 'Delete', description: 'Delete selected files' },
  nextGroup: { key: 'ArrowDown', description: 'Next group' },
  prevGroup: { key: 'ArrowUp', description: 'Previous group' },
  openFile: { key: 'Enter', description: 'Open selected file' },
  toggleSelection: { key: ' ', description: 'Toggle file selection' },
  search: { key: 'f', ctrl: true, description: 'Focus search' },
  settings: { key: ',', ctrl: true, description: 'Open settings' },
};
```

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 11.2: Create Focus Management Hook

### Overview

Create a hook for managing focus in lists.

### Changes Required

**File**: `src/lib/utils/focus.ts`

```typescript
export function createFocusManager<T>(items: () => T[], onSelect: (item: T) => void) {
  let currentIndex = $state(-1);

  function moveFocus(delta: number) {
    const list = items();
    if (list.length === 0) return;

    const newIndex = Math.max(0, Math.min(list.length - 1, currentIndex + delta));
    if (newIndex !== currentIndex) {
      currentIndex = newIndex;
      onSelect(list[newIndex]);
    }
  }

  function setFocus(index: number) {
    const list = items();
    if (index >= 0 && index < list.length) {
      currentIndex = index;
      onSelect(list[index]);
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    switch (e.key) {
      case 'ArrowDown':
        e.preventDefault();
        moveFocus(1);
        break;
      case 'ArrowUp':
        e.preventDefault();
        moveFocus(-1);
        break;
      case 'Home':
        e.preventDefault();
        setFocus(0);
        break;
      case 'End':
        e.preventDefault();
        setFocus(items().length - 1);
        break;
    }
  }

  return {
    get currentIndex() {
      return currentIndex;
    },
    setFocus,
    handleKeydown,
  };
}
```

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 11.3: Add Keyboard Navigation to Groups List

### Overview

Add keyboard navigation to the duplicate groups list.

### Changes Required

Update DuplicateGroupsList.svelte:

- Add tabindex for focusability
- Handle arrow key navigation
- Add visual focus indicator

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 11.4: Add Keyboard Navigation to Files List

### Overview

Add keyboard navigation to the file details panel.

### Changes Required

Update FileDetailsPanel.svelte:

- Space to toggle selection
- Enter to open file
- Arrow keys to navigate
- Delete to mark for deletion

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 11.5: Create Keyboard Shortcuts Help Dialog

### Overview

Create a dialog showing all available keyboard shortcuts.

### Changes Required

**File**: `src/lib/components/ShortcutsHelp.svelte`

```svelte
<script lang="ts">
  import { defaultShortcuts } from '$lib/stores/shortcuts';

  interface Props {
    onClose: () => void;
  }

  let { onClose }: Props = $props();

  function formatShortcut(s: typeof defaultShortcuts.startScan): string {
    const parts: string[] = [];
    if (s.ctrl) parts.push('Ctrl');
    if (s.shift) parts.push('Shift');
    if (s.alt) parts.push('Alt');
    parts.push(s.key === ' ' ? 'Space' : s.key);
    return parts.join(' + ');
  }
</script>

<div class="overlay" onclick={onClose}>
  <div class="dialog" onclick={(e) => e.stopPropagation()}>
    <h2>Keyboard Shortcuts</h2>

    <div class="shortcuts-list">
      <div class="section">
        <h3>Navigation</h3>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.nextGroup)}</span>
          <span class="desc">{defaultShortcuts.nextGroup.description}</span>
        </div>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.prevGroup)}</span>
          <span class="desc">{defaultShortcuts.prevGroup.description}</span>
        </div>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.search)}</span>
          <span class="desc">{defaultShortcuts.search.description}</span>
        </div>
      </div>

      <div class="section">
        <h3>Selection</h3>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.selectAll)}</span>
          <span class="desc">{defaultShortcuts.selectAll.description}</span>
        </div>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.deselectAll)}</span>
          <span class="desc">{defaultShortcuts.deselectAll.description}</span>
        </div>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.toggleSelection)}</span>
          <span class="desc">{defaultShortcuts.toggleSelection.description}</span>
        </div>
      </div>

      <div class="section">
        <h3>Actions</h3>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.openFile)}</span>
          <span class="desc">{defaultShortcuts.openFile.description}</span>
        </div>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.deleteSelected)}</span>
          <span class="desc">{defaultShortcuts.deleteSelected.description}</span>
        </div>
      </div>

      <div class="section">
        <h3>Scanning</h3>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.startScan)}</span>
          <span class="desc">{defaultShortcuts.startScan.description}</span>
        </div>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.pauseScan)}</span>
          <span class="desc">{defaultShortcuts.pauseScan.description}</span>
        </div>
        <div class="shortcut">
          <span class="keys">{formatShortcut(defaultShortcuts.cancelScan)}</span>
          <span class="desc">{defaultShortcuts.cancelScan.description}</span>
        </div>
      </div>
    </div>

    <button class="close-btn" onclick={onClose}>Close</button>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background: var(--surface);
    border-radius: 12px;
    padding: 1.5rem;
    max-width: 500px;
    width: 90%;
    max-height: 80vh;
    overflow-y: auto;
  }

  h2 {
    margin: 0 0 1.5rem;
  }

  .section {
    margin-bottom: 1.5rem;
  }

  h3 {
    font-size: 0.85rem;
    color: var(--text-secondary);
    margin: 0 0 0.5rem;
    text-transform: uppercase;
  }

  .shortcut {
    display: flex;
    justify-content: space-between;
    padding: 0.5rem 0;
    border-bottom: 1px solid var(--border);
  }

  .keys {
    font-family: var(--font-mono);
    background: var(--background);
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    font-size: 0.85rem;
  }

  .desc {
    color: var(--text-secondary);
  }

  .close-btn {
    width: 100%;
    padding: 0.75rem;
    margin-top: 1rem;
    border: none;
    border-radius: 6px;
    background: var(--primary);
    color: white;
    cursor: pointer;
  }
</style>
```

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## Phase 11.6: Add ARIA Labels and Accessibility

### Overview

Add proper ARIA labels and roles for screen readers.

### Changes Required

Update all interactive components with:

- Proper role attributes
- aria-label descriptions
- aria-selected for selections
- aria-expanded for collapsible sections
- Focus visible styles

### Commit

Execute `/cl:commit`

### Code Review

Run code-review-fix-loop agent.

---

## End of File 11

After completing all phases:

- Full keyboard navigation
- Shortcuts for all major actions
- Focus management in lists
- Keyboard shortcuts help dialog
- ARIA labels for accessibility
- Screen reader support

**Next**: Proceed to [12-permissions.md](./12-permissions.md)
