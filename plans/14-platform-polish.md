# File 14: Platform Polish & Final Testing

## Overview

This file covers achieving native look and feel on each platform, final styling polish, comprehensive E2E testing, and preparing for distribution.

## Prerequisites

- Completed Files 01-13

---

## Phase 14.1: macOS Native Styling

### Overview
Apply macOS-specific styling for native look and feel.

### Changes Required

**File**: `src/lib/styles/macos.css`

```css
/* macOS-specific styling */
[data-platform="macos"] {
  /* Native macOS font stack */
  --font-sans: -apple-system, BlinkMacSystemFont, 'SF Pro Text', 'SF Pro Icons',
               'Helvetica Neue', Helvetica, Arial, sans-serif;
  --font-mono: 'SF Mono', SFMono-Regular, Menlo, Monaco, Consolas, monospace;

  /* macOS-style colors */
  --primary: #007aff;
  --primary-hover: #0056b3;

  /* Vibrancy/translucency effects */
  --surface-opacity: 0.85;
}

/* macOS window controls spacing */
[data-platform="macos"] .app-header {
  padding-left: 80px; /* Space for traffic lights */
}

/* macOS-style buttons */
[data-platform="macos"] button {
  border-radius: 6px;
}

/* macOS-style inputs */
[data-platform="macos"] input,
[data-platform="macos"] select {
  border-radius: 6px;
  border-width: 1px;
}

/* macOS-style scrollbars */
[data-platform="macos"] ::-webkit-scrollbar {
  width: 8px;
}

[data-platform="macos"] ::-webkit-scrollbar-thumb {
  background: rgba(0, 0, 0, 0.3);
  border-radius: 4px;
}

[data-platform="macos"] ::-webkit-scrollbar-thumb:hover {
  background: rgba(0, 0, 0, 0.5);
}
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 14.2: Windows Native Styling

### Overview
Apply Windows-specific styling for native look and feel.

### Changes Required

**File**: `src/lib/styles/windows.css`

```css
/* Windows-specific styling */
[data-platform="windows"] {
  /* Windows font stack */
  --font-sans: 'Segoe UI', 'Segoe UI Variable', -apple-system, sans-serif;
  --font-mono: 'Cascadia Code', 'Cascadia Mono', Consolas, monospace;

  /* Windows accent colors */
  --primary: #0078d4;
  --primary-hover: #106ebe;
}

/* Windows-style buttons */
[data-platform="windows"] button {
  border-radius: 4px;
}

/* Windows-style inputs */
[data-platform="windows"] input,
[data-platform="windows"] select {
  border-radius: 4px;
  border-width: 1px;
}

/* Windows-style focus */
[data-platform="windows"] :focus-visible {
  outline: 2px solid var(--primary);
  outline-offset: 1px;
}

/* Windows-style scrollbars */
[data-platform="windows"] ::-webkit-scrollbar {
  width: 12px;
}

[data-platform="windows"] ::-webkit-scrollbar-track {
  background: var(--background);
}

[data-platform="windows"] ::-webkit-scrollbar-thumb {
  background: var(--border);
  border: 3px solid var(--background);
}

[data-platform="windows"] ::-webkit-scrollbar-thumb:hover {
  background: var(--text-secondary);
}
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 14.3: Platform Detection and Style Loading

### Overview
Detect platform and apply appropriate styles.

### Changes Required

**File**: `src/lib/utils/platform.ts`

```typescript
import { platform } from '@tauri-apps/plugin-os';

export type Platform = 'macos' | 'windows' | 'linux';

let detectedPlatform: Platform | null = null;

export async function detectPlatform(): Promise<Platform> {
  if (detectedPlatform) return detectedPlatform;

  const os = await platform();
  detectedPlatform = os === 'darwin' ? 'macos' : os === 'windows' ? 'windows' : 'linux';

  // Set platform attribute on document
  document.documentElement.setAttribute('data-platform', detectedPlatform);

  return detectedPlatform;
}

export function getPlatform(): Platform | null {
  return detectedPlatform;
}
```

Update main.ts:
```typescript
import { detectPlatform } from '$lib/utils/platform';

detectPlatform().then(() => {
  // Platform styles are now applied
});
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 14.4: Final UI Polish

### Overview
Review and polish all UI components for consistency.

### Changes Required

- Consistent spacing throughout
- Proper loading states
- Smooth transitions
- Error state styling
- Empty state designs
- Hover effects
- Focus indicators

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 14.5: Create E2E Test Suite

### Overview
Set up WebdriverIO E2E tests for the complete application.

### Changes Required

**File**: `e2e/wdio.conf.js`

```javascript
import { spawn } from 'child_process';

let tauriDriver;

export const config = {
  specs: ['./specs/**/*.spec.js'],
  maxInstances: 1,
  capabilities: [
    {
      maxInstances: 1,
      'tauri:options': {
        application: '../src-tauri/target/debug/duplifind',
      },
    },
  ],
  logLevel: 'info',
  bail: 0,
  waitforTimeout: 10000,
  connectionRetryTimeout: 120000,
  connectionRetryCount: 3,
  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: {
    ui: 'bdd',
    timeout: 60000,
  },

  beforeSession: () => {
    tauriDriver = spawn('tauri-driver', [], {
      stdio: [null, process.stdout, process.stderr],
    });
  },

  afterSession: () => {
    tauriDriver.kill();
  },
};
```

**File**: `e2e/specs/app.spec.js`

```javascript
describe('DupliFind App', () => {
  it('should launch successfully', async () => {
    const title = await browser.getTitle();
    expect(title).toBe('DupliFind');
  });

  it('should show home view on start', async () => {
    const heading = await $('h2');
    const text = await heading.getText();
    expect(text).toBe('Find Duplicate Files');
  });

  it('should have start scan button', async () => {
    const button = await $('button*=Start Scan');
    expect(await button.isDisplayed()).toBe(true);
  });
});

describe('Scan Flow', () => {
  it('should start scanning when button clicked', async () => {
    const button = await $('button*=Start Scan');
    await button.click();

    // Wait for scanning view
    await browser.waitUntil(
      async () => {
        const spinner = await $('.spinner');
        return spinner.isDisplayed();
      },
      { timeout: 5000 }
    );
  });

  it('should show progress during scan', async () => {
    const progress = await $('.progress-display');
    expect(await progress.isDisplayed()).toBe(true);
  });

  it('should allow cancelling scan', async () => {
    const cancelBtn = await $('button*=Cancel');
    await cancelBtn.click();

    // Should return to home
    await browser.waitUntil(
      async () => {
        const homeBtn = await $('button*=Start Scan');
        return homeBtn.isDisplayed();
      },
      { timeout: 5000 }
    );
  });
});

describe('Settings', () => {
  it('should open settings view', async () => {
    const settingsBtn = await $('button*=Settings');
    if (await settingsBtn.isExisting()) {
      await settingsBtn.click();

      const themeSelect = await $('select#theme');
      expect(await themeSelect.isDisplayed()).toBe(true);
    }
  });

  it('should change theme', async () => {
    const themeSelect = await $('select#theme');
    if (await themeSelect.isExisting()) {
      await themeSelect.selectByVisibleText('Dark');
      const theme = await browser.execute(() => {
        return document.documentElement.getAttribute('data-theme');
      });
      expect(theme).toBe('dark');
    }
  });
});
```

**File**: `e2e/specs/duplicate-detection.spec.js`

```javascript
import * as fs from 'fs';
import * as path from 'path';
import * as os from 'os';

/**
 * E2E tests for actual duplicate detection logic.
 * These tests create real duplicate files and verify the app correctly identifies them.
 */
describe('Duplicate Detection Logic', () => {
  let testDir;

  beforeAll(async () => {
    // Create a temporary directory with test files
    testDir = path.join(os.tmpdir(), `duplifind-e2e-${Date.now()}`);
    fs.mkdirSync(testDir, { recursive: true });

    // Create test files: some duplicates, some unique
    const content1 = 'This is duplicate content that appears multiple times';
    const content2 = 'This is unique content only appearing once';
    const content3 = 'Another duplicate content string for testing';

    // Create duplicates of content1
    fs.writeFileSync(path.join(testDir, 'file1-original.txt'), content1);
    fs.writeFileSync(path.join(testDir, 'file1-copy1.txt'), content1);
    fs.writeFileSync(path.join(testDir, 'file1-copy2.txt'), content1);

    // Create unique file
    fs.writeFileSync(path.join(testDir, 'unique-file.txt'), content2);

    // Create another set of duplicates
    fs.writeFileSync(path.join(testDir, 'file2-original.txt'), content3);
    fs.writeFileSync(path.join(testDir, 'subdir/file2-copy.txt'), content3);
  });

  afterAll(async () => {
    // Cleanup test directory
    if (testDir && fs.existsSync(testDir)) {
      fs.rmSync(testDir, { recursive: true, force: true });
    }
  });

  it('should correctly identify duplicate files', async () => {
    // Select the test directory for scanning
    const addFolderBtn = await $('button*=Add Folder');
    await addFolderBtn.click();

    // Use the native dialog or directly invoke the command
    // For E2E, we may need to use invoke directly
    await browser.execute(async (dir) => {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_setting', { key: 'last_scan_paths', value: JSON.stringify([dir]) });
    }, testDir);

    // Refresh the app to load the test directory
    await browser.refresh();
    await browser.pause(1000);

    // Start the scan
    const scanBtn = await $('button*=Start Scan');
    await scanBtn.click();

    // Wait for scan to complete (look for results)
    await browser.waitUntil(
      async () => {
        const results = await $('.duplicate-groups');
        return results.isExisting();
      },
      { timeout: 30000, timeoutMsg: 'Scan did not complete in time' }
    );

    // Verify duplicate groups are found
    const groups = await $$('.duplicate-group');
    expect(groups.length).toBe(2); // Should find 2 duplicate groups

    // Verify correct number of files in each group
    const group1Files = await groups[0].$$('.file-item');
    const group2Files = await groups[1].$$('.file-item');

    // One group should have 3 files (file1 duplicates), other should have 2 (file2 duplicates)
    const fileCounts = [group1Files.length, group2Files.length].sort();
    expect(fileCounts).toEqual([2, 3]);
  });

  it('should not flag unique files as duplicates', async () => {
    // After the previous scan, verify unique file is not in any group
    const allFilesInGroups = await $$('.file-item .file-name');
    const fileNames = await Promise.all(allFilesInGroups.map(el => el.getText()));

    expect(fileNames).not.toContain('unique-file.txt');
  });

  it('should correctly calculate file sizes', async () => {
    // Get the size shown in the UI for a known file
    const firstGroup = await $('.duplicate-group');
    const sizeElement = await firstGroup.$('.group-size');
    const sizeText = await sizeElement.getText();

    // File content is ~54 bytes, group of 3 = ~162 bytes
    // Verify size is shown and reasonable
    expect(sizeText).toMatch(/\d+\s*(B|KB)/);
  });

  it('should correctly identify files across subdirectories', async () => {
    // Verify file2-copy.txt in subdir was found as duplicate
    const allFilePaths = await $$('.file-item .file-path');
    const paths = await Promise.all(allFilePaths.map(el => el.getAttribute('title')));

    const hasSubdirFile = paths.some(p => p && p.includes('subdir'));
    expect(hasSubdirFile).toBe(true);
  });

  it('should mark oldest file as original', async () => {
    // The oldest file in each group should have the "Original" badge
    const groups = await $$('.duplicate-group');

    for (const group of groups) {
      const originalBadge = await group.$('.original-badge');
      expect(await originalBadge.isExisting()).toBe(true);

      // Should only have one original per group
      const allBadges = await group.$$('.original-badge');
      expect(allBadges.length).toBe(1);
    }
  });

  it('should allow selecting duplicates for deletion', async () => {
    // Select all except oldest in first group
    const selectBtn = await $('button*=Select All Except Oldest');
    await selectBtn.click();

    // Verify checkboxes are selected
    const selectedCheckboxes = await $$('.file-item input[type="checkbox"]:checked');
    expect(selectedCheckboxes.length).toBeGreaterThan(0);

    // The original files should NOT be selected
    const groups = await $$('.duplicate-group');
    for (const group of groups) {
      const originalItem = await group.$('.file-item:has(.original-badge)');
      const checkbox = await originalItem.$('input[type="checkbox"]');
      expect(await checkbox.isSelected()).toBe(false);
    }
  });
});

describe('Duplicate Detection Edge Cases', () => {
  let edgeCaseDir;

  beforeAll(async () => {
    edgeCaseDir = path.join(os.tmpdir(), `duplifind-edge-${Date.now()}`);
    fs.mkdirSync(edgeCaseDir, { recursive: true });

    // Create edge case files
    // Empty files (should be grouped together)
    fs.writeFileSync(path.join(edgeCaseDir, 'empty1.txt'), '');
    fs.writeFileSync(path.join(edgeCaseDir, 'empty2.txt'), '');

    // Files with same size but different content
    fs.writeFileSync(path.join(edgeCaseDir, 'same-size-1.txt'), 'AAAA');
    fs.writeFileSync(path.join(edgeCaseDir, 'same-size-2.txt'), 'BBBB');

    // Large identical files (tests partial hashing)
    const largeContent = 'x'.repeat(100000);
    fs.writeFileSync(path.join(edgeCaseDir, 'large1.bin'), largeContent);
    fs.writeFileSync(path.join(edgeCaseDir, 'large2.bin'), largeContent);
  });

  afterAll(async () => {
    if (edgeCaseDir && fs.existsSync(edgeCaseDir)) {
      fs.rmSync(edgeCaseDir, { recursive: true, force: true });
    }
  });

  it('should correctly group empty files as duplicates', async () => {
    // Set up and scan the edge case directory
    await browser.execute(async (dir) => {
      const { invoke } = await import('@tauri-apps/api/core');
      await invoke('set_setting', { key: 'last_scan_paths', value: JSON.stringify([dir]) });
    }, edgeCaseDir);

    await browser.refresh();
    const scanBtn = await $('button*=Start Scan');
    await scanBtn.click();

    await browser.waitUntil(
      async () => (await $('.duplicate-groups')).isExisting(),
      { timeout: 30000 }
    );

    // Find the group containing empty files
    const allFiles = await $$('.file-item .file-name');
    const fileNames = await Promise.all(allFiles.map(el => el.getText()));

    const hasEmpty1 = fileNames.includes('empty1.txt');
    const hasEmpty2 = fileNames.includes('empty2.txt');

    // Both empty files should appear in results (as duplicates)
    expect(hasEmpty1 && hasEmpty2).toBe(true);
  });

  it('should NOT group same-size files with different content', async () => {
    // same-size-1.txt and same-size-2.txt should NOT be in the same group
    const groups = await $$('.duplicate-group');

    for (const group of groups) {
      const filesInGroup = await group.$$('.file-item .file-name');
      const names = await Promise.all(filesInGroup.map(el => el.getText()));

      const hasSameSize1 = names.includes('same-size-1.txt');
      const hasSameSize2 = names.includes('same-size-2.txt');

      // They should never both be in the same group
      expect(hasSameSize1 && hasSameSize2).toBe(false);
    }
  });

  it('should correctly detect large file duplicates', async () => {
    // large1.bin and large2.bin should be grouped
    const groups = await $$('.duplicate-group');

    let foundLargeGroup = false;
    for (const group of groups) {
      const filesInGroup = await group.$$('.file-item .file-name');
      const names = await Promise.all(filesInGroup.map(el => el.getText()));

      if (names.includes('large1.bin') && names.includes('large2.bin')) {
        foundLargeGroup = true;
        break;
      }
    }

    expect(foundLargeGroup).toBe(true);
  });
});
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 14.6: Run E2E Tests

### Overview
Run full E2E test suite and fix any issues.

### Changes Required

1. Build debug version of app
2. Run E2E tests
3. Fix any failures
4. Ensure all tests pass

### Success Criteria

#### Automated Verification
- [ ] `npm run build` succeeds
- [ ] `npm run tauri build -- --debug` succeeds
- [ ] All E2E tests pass including:
  - [ ] Basic app launch and navigation tests
  - [ ] Scan flow tests
  - [ ] Settings tests
  - [ ] **Duplicate detection logic tests:**
    - [ ] Correctly identifies duplicate files
    - [ ] Does not flag unique files as duplicates
    - [ ] Correctly calculates file sizes
    - [ ] Finds duplicates across subdirectories
    - [ ] Marks oldest file as original
    - [ ] Selection logic works correctly
  - [ ] **Edge case tests:**
    - [ ] Empty files grouped as duplicates
    - [ ] Same-size different-content files NOT grouped
    - [ ] Large file duplicate detection works

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 14.7: Build Scripts

### Overview
Create build scripts for distribution.

### Changes Required

**File**: `scripts/build-mac.sh`

```bash
#!/bin/bash
set -e

echo "Building DupliFind for macOS..."

# Build frontend
npm run build

# Build Tauri app
npm run tauri build

echo "Build complete! App bundle at:"
echo "  src-tauri/target/release/bundle/macos/DupliFind.app"
echo "  src-tauri/target/release/bundle/dmg/DupliFind_*.dmg"
```

**File**: `scripts/build-windows.ps1`

```powershell
Write-Host "Building DupliFind for Windows..." -ForegroundColor Cyan

# Build frontend
npm run build

# Build Tauri app
npm run tauri build

Write-Host "Build complete! Installer at:" -ForegroundColor Green
Write-Host "  src-tauri\target\release\bundle\msi\DupliFind_*.msi"
Write-Host "  src-tauri\target\release\bundle\nsis\DupliFind_*-setup.exe"
```

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 14.8: Final Documentation and Cleanup

### Overview
Update documentation and clean up code.

### Changes Required

1. Update README with final instructions
2. Add CHANGELOG.md
3. Clean up any TODO comments
4. Ensure all code is properly documented
5. Final code review

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes
- [ ] `cargo clippy` shows no warnings
- [ ] `npm run test` passes
- [ ] `cargo test` passes

#### Manual Verification
- [ ] App builds successfully on macOS
- [ ] App builds successfully on Windows
- [ ] All features work as specified
- [ ] UI is polished and consistent

### Code Review
Run final comprehensive code review across all files.

### Commit
Execute `/cl:commit`

---

## End of File 14

After completing all phases:
- Native macOS styling
- Native Windows styling
- Platform detection
- Complete E2E test suite
- Build scripts for distribution
- Final documentation

**Next**: Proceed to [15-system-tray.md](./15-system-tray.md) for optional system tray integration.

---

# Implementation Complete!

After completing File 15, you will have completed the full implementation of DupliFind.

## Summary of Deliverables

1. **Project Foundation** (File 01): Tauri + Svelte project setup
2. **Database** (File 02): SQLite with complete schema
3. **File Scanning** (File 03): Directory traversal, metadata collection, folder picker UI
4. **Duplicate Detection** (File 04): BLAKE3 hashing with three-stage algorithm
5. **Results UI** (File 05): Master-detail layout with groups and files
6. **Selection & Deletion** (File 06): Batch deletion with verification, path depth selection
7. **Progress & Controls** (File 07): Pause/resume with persistence, estimated time remaining
8. **Settings** (File 08): Theme, parallelism, protected folders
9. **File Operations** (File 09): Open, reveal, copy path, view file info, context menu
10. **Filtering & Thumbnails** (File 10): Type filters, search, image previews
11. **Keyboard Navigation** (File 11): Full keyboard support and shortcuts
12. **Permissions** (File 12): macOS Full Disk Access wizard
13. **Error Handling** (File 13): Skip/retry, disk full handling, throttling, incremental scan
14. **Platform Polish** (File 14): Native styling and E2E tests
15. **System Tray** (File 15): Minimize to tray feature

## Next Steps

1. Run all tests to ensure everything works
2. Build for your target platform
3. Test thoroughly on real data
4. Distribute to users
