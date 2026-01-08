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
- [ ] All E2E tests pass

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
