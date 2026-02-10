# File 10: Filtering, Search & Thumbnails

## Overview

This file covers implementing file type filters, full-text search, size range filters, and image thumbnail generation and caching.

## Prerequisites

- Completed Files 01-09

---

## Phase 10.1: Create Filter Types

### Overview
Define filter types and filtering logic.

### Changes Required

**File**: `src/lib/stores/filters.ts`

```typescript
import type { DuplicateGroup, FileType, FilterState } from '$lib/types';

export const fileTypeExtensions: Record<FileType, string[]> = {
  images: ['jpg', 'jpeg', 'png', 'gif', 'webp', 'bmp', 'svg', 'ico', 'tiff', 'heic'],
  videos: ['mp4', 'mov', 'avi', 'mkv', 'webm', 'wmv', 'flv', 'm4v'],
  documents: ['pdf', 'doc', 'docx', 'txt', 'rtf', 'md', 'xls', 'xlsx', 'ppt', 'pptx'],
  audio: ['mp3', 'wav', 'flac', 'aac', 'm4a', 'ogg', 'wma'],
  other: [],
  all: [],
};

export function filterGroups(groups: DuplicateGroup[], filter: FilterState): DuplicateGroup[] {
  return groups.filter((group) => {
    // File type filter
    if (filter.fileType !== 'all') {
      const ext = getExtension(group.files[0]?.path || '');
      const validExts = fileTypeExtensions[filter.fileType];
      if (filter.fileType === 'other') {
        const allKnownExts = [
          ...fileTypeExtensions.images,
          ...fileTypeExtensions.videos,
          ...fileTypeExtensions.documents,
          ...fileTypeExtensions.audio,
        ];
        if (allKnownExts.includes(ext)) return false;
      } else {
        if (!validExts.includes(ext)) return false;
      }
    }

    // Size filter
    if (filter.minSize !== null && group.file_size < filter.minSize) return false;
    if (filter.maxSize !== null && group.file_size > filter.maxSize) return false;

    // Search filter
    if (filter.searchQuery) {
      const query = filter.searchQuery.toLowerCase();
      const matchesPath = group.files.some((f) => f.path.toLowerCase().includes(query));
      if (!matchesPath) return false;
    }

    return true;
  });
}

function getExtension(path: string): string {
  return path.split('.').pop()?.toLowerCase() || '';
}
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 10.2: Create Filter Bar Component

### Overview
Create the filter bar UI with file type buttons and search.

### Changes Required

**File**: `src/lib/components/FilterBar.svelte`

```svelte
<script lang="ts">
  import type { FileType, FilterState } from '$lib/types';

  interface Props {
    filter: FilterState;
    onFilterChange: (filter: FilterState) => void;
    groupCounts: Record<FileType, number>;
  }

  let { filter, onFilterChange, groupCounts }: Props = $props();

  const fileTypes: { type: FileType; label: string; icon: string }[] = [
    { type: 'all', label: 'All', icon: '📁' },
    { type: 'images', label: 'Images', icon: '🖼️' },
    { type: 'videos', label: 'Videos', icon: '🎬' },
    { type: 'documents', label: 'Documents', icon: '📄' },
    { type: 'audio', label: 'Audio', icon: '🎵' },
    { type: 'other', label: 'Other', icon: '📦' },
  ];

  function setFileType(type: FileType) {
    onFilterChange({ ...filter, fileType: type });
  }

  function setSearch(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    onFilterChange({ ...filter, searchQuery: value });
  }

  function clearFilters() {
    onFilterChange({
      fileType: 'all',
      minSize: null,
      maxSize: null,
      searchQuery: '',
    });
  }

  let hasActiveFilters = $derived(
    filter.fileType !== 'all' ||
      filter.minSize !== null ||
      filter.maxSize !== null ||
      filter.searchQuery !== ''
  );
</script>

<div class="filter-bar">
  <div class="file-type-filters">
    {#each fileTypes as ft}
      <button
        class="type-btn"
        class:active={filter.fileType === ft.type}
        onclick={() => setFileType(ft.type)}
      >
        <span class="icon">{ft.icon}</span>
        <span class="label">{ft.label}</span>
        <span class="count">{groupCounts[ft.type] || 0}</span>
      </button>
    {/each}
  </div>

  <div class="search-box">
    <input
      type="search"
      placeholder="Search files..."
      value={filter.searchQuery}
      oninput={setSearch}
    />
  </div>

  {#if hasActiveFilters}
    <button class="clear-btn" onclick={clearFilters}>Clear Filters</button>
  {/if}
</div>

<style>
  .filter-bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem;
    background: var(--surface);
    border-bottom: 1px solid var(--border);
  }

  .file-type-filters {
    display: flex;
    gap: 0.25rem;
  }

  .type-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .type-btn:hover {
    background: var(--background);
  }

  .type-btn.active {
    background: var(--primary);
    color: white;
    border-color: var(--primary);
  }

  .type-btn .icon {
    font-size: 1rem;
  }

  .type-btn .count {
    font-size: 0.75rem;
    opacity: 0.7;
  }

  .search-box {
    flex: 1;
    max-width: 300px;
  }

  .search-box input {
    width: 100%;
    padding: 0.5rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--background);
    color: var(--text);
  }

  .clear-btn {
    padding: 0.5rem 1rem;
    border: none;
    background: transparent;
    color: var(--primary);
    cursor: pointer;
    font-size: 0.85rem;
  }
</style>
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 10.3: Add Thumbnail Generation Backend

### Overview
Add Rust code for generating image thumbnails.

### Changes Required

**File**: `src-tauri/Cargo.toml`

Add:
```toml
image = { version = "0.25", default-features = false, features = ["jpeg", "png", "gif", "webp"] }
```

**File**: `src-tauri/src/services/thumbnails.rs`

```rust
use image::GenericImageView;
use std::path::Path;

const THUMBNAIL_SIZE: u32 = 100;

pub fn generate_thumbnail(path: &Path) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let img = image::open(path)?;

    // Calculate thumbnail dimensions maintaining aspect ratio
    let (width, height) = img.dimensions();
    let ratio = width as f64 / height as f64;

    let (thumb_w, thumb_h) = if ratio > 1.0 {
        (THUMBNAIL_SIZE, (THUMBNAIL_SIZE as f64 / ratio) as u32)
    } else {
        ((THUMBNAIL_SIZE as f64 * ratio) as u32, THUMBNAIL_SIZE)
    };

    let thumbnail = img.thumbnail(thumb_w, thumb_h);

    let mut bytes: Vec<u8> = Vec::new();
    thumbnail.write_to(&mut std::io::Cursor::new(&mut bytes), image::ImageFormat::Jpeg)?;

    Ok(bytes)
}

pub fn is_image_file(path: &Path) -> bool {
    let ext = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "gif" | "webp" | "bmp")
}
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 10.4: Create Thumbnail Commands

### Overview
Create Tauri commands for thumbnail generation and caching.

### Changes Required

**File**: `src-tauri/src/commands/thumbnails.rs`

```rust
use crate::services::thumbnails;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use std::path::Path;

#[tauri::command]
pub async fn get_thumbnail(path: String) -> Result<Option<String>, String> {
    let path = Path::new(&path);

    if !thumbnails::is_image_file(path) {
        return Ok(None);
    }

    match thumbnails::generate_thumbnail(path) {
        Ok(bytes) => {
            let base64 = BASE64.encode(&bytes);
            Ok(Some(format!("data:image/jpeg;base64,{}", base64)))
        }
        Err(e) => {
            log::debug!("Failed to generate thumbnail for {}: {}", path.display(), e);
            Ok(None)
        }
    }
}
```

Add to Cargo.toml:
```toml
base64 = "0.22"
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 10.5: Create Thumbnail Component

### Overview
Create a component that displays thumbnails with lazy loading.

### Changes Required

**File**: `src/lib/components/FileThumbnail.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  interface Props {
    path: string;
    size?: number;
  }

  let { path, size = 48 }: Props = $props();

  let thumbnail = $state<string | null>(null);
  let loading = $state(true);
  let error = $state(false);

  async function loadThumbnail() {
    try {
      const result = await invoke<string | null>('get_thumbnail', { path });
      thumbnail = result;
    } catch (e) {
      error = true;
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    loadThumbnail();
  });

  function getFileIcon(path: string): string {
    const ext = path.split('.').pop()?.toLowerCase() || '';
    const icons: Record<string, string> = {
      jpg: '🖼️', jpeg: '🖼️', png: '🖼️', gif: '🖼️', webp: '🖼️',
      mp4: '🎬', mov: '🎬', avi: '🎬', mkv: '🎬',
      mp3: '🎵', wav: '🎵', flac: '🎵',
      pdf: '📄', doc: '📄', docx: '📄', txt: '📄',
    };
    return icons[ext] || '📁';
  }
</script>

<div class="thumbnail" style="width: {size}px; height: {size}px">
  {#if loading}
    <div class="placeholder">...</div>
  {:else if thumbnail}
    <img src={thumbnail} alt="" />
  {:else}
    <span class="icon">{getFileIcon(path)}</span>
  {/if}
</div>

<style>
  .thumbnail {
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--background);
    border-radius: 4px;
    overflow: hidden;
    flex-shrink: 0;
  }

  img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .placeholder {
    color: var(--text-secondary);
  }

  .icon {
    font-size: 1.5rem;
  }
</style>
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 10.6: Integrate Filters in Results View

### Overview
Add filter bar and filtering logic to the results view.

### Changes Required

Update ResultsView.svelte to include FilterBar and apply filtering to groups.

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 10.7: Add Thumbnail Caching

### Overview
Cache thumbnails to avoid regenerating on every view.

### Changes Required

Add thumbnail caching table and logic.

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 10.8: Tests

Add tests for filtering and thumbnail generation.

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## End of File 10

After completing all phases:
- File type filters (Images, Videos, Documents, Audio, Other)
- Full-text search across file paths
- Size range filters
- Image thumbnail generation
- Thumbnail caching
- Filter counts per type

**Next**: Proceed to [11-keyboard-nav.md](./11-keyboard-nav.md)
