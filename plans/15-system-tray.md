# File 15: System Tray Integration

## Overview

This file covers implementing the optional "minimize to tray" feature, allowing users to keep the app running in the system tray/menu bar while minimizing the main window.

## Prerequisites

- Completed Files 01-14

---

## Phase 15.1: Add Tray Plugin

### Overview
Add the Tauri tray plugin for system tray functionality.

### Changes Required

#### 15.1.1 Install Tray Plugin

```bash
npm run tauri add tray-icon
```

#### 15.1.2 Update Capabilities

**File**: `src-tauri/capabilities/default.json`

Add to permissions array:
```json
"tray-icon:default",
"tray-icon:allow-set-icon",
"tray-icon:allow-set-menu",
"tray-icon:allow-set-tooltip"
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check` passes

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 15.2: Create Tray Icon Assets

### Overview
Create tray icon assets for both macOS and Windows.

### Changes Required

#### 15.2.1 Create Icon Files

Create tray icons in multiple sizes:

**Files to create**:
- `src-tauri/icons/tray-icon.png` (32x32)
- `src-tauri/icons/tray-icon@2x.png` (64x64, for Retina)
- `src-tauri/icons/tray-icon.ico` (Windows)

The icon should be:
- Simple and recognizable at small sizes
- Works well in both light and dark system themes
- Uses transparency appropriately for macOS menu bar

### Success Criteria

#### Manual Verification
- [ ] Icon files exist in correct locations
- [ ] Icons are visually clear at small sizes

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 15.3: Create Tray Settings

### Overview
Add setting for "minimize to tray" preference and persist it.

### Changes Required

#### 15.3.1 Add Setting to Database

The settings system already supports arbitrary key-value pairs. We'll use:
- Key: `minimize_to_tray`
- Value: `"true"` or `"false"`

#### 15.3.2 Update Settings Store

**File**: `src/lib/stores/settings.ts`

Add to AppSettings interface:
```typescript
export interface AppSettings {
  theme: 'system' | 'light' | 'dark';
  parallelism: 'light' | 'normal' | 'aggressive';
  lastScanPaths: string[];
  minimizeToTray: boolean;
}

const defaultSettings: AppSettings = {
  theme: 'system',
  parallelism: 'normal',
  lastScanPaths: [],
  minimizeToTray: false,
};
```

Update the load method to handle the new setting:
```typescript
async load() {
  try {
    const allSettings = await invoke<{ key: string; value: string }[]>('get_all_settings');
    for (const setting of allSettings) {
      // ... existing settings ...
      if (setting.key === 'minimize_to_tray') {
        this.settings.minimizeToTray = setting.value === 'true';
      }
    }
    // ...
  }
}
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 15.4: Implement Tray Service

### Overview
Create the Rust service for managing the system tray.

### Changes Required

#### 15.4.1 Create Tray Module

**File**: `src-tauri/src/tray.rs`

```rust
//! System tray integration

use tauri::{
    AppHandle, CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu,
    SystemTrayMenuItem,
};

/// Create the system tray with menu
pub fn create_tray() -> SystemTray {
    let show = CustomMenuItem::new("show".to_string(), "Show DupliFind");
    let quit = CustomMenuItem::new("quit".to_string(), "Quit");

    let tray_menu = SystemTrayMenu::new()
        .add_item(show)
        .add_native_item(SystemTrayMenuItem::Separator)
        .add_item(quit);

    SystemTray::new()
        .with_menu(tray_menu)
        .with_tooltip("DupliFind")
}

/// Handle system tray events
pub fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::LeftClick { .. } => {
            // Show the main window on left click
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            "show" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "quit" => {
                std::process::exit(0);
            }
            _ => {}
        },
        _ => {}
    }
}

/// Update tray tooltip with scan progress percentage
pub fn update_tray_for_scanning(app: &AppHandle, is_scanning: bool) {
    if let Some(tray) = app.tray_handle() {
        let tooltip = if is_scanning {
            "DupliFind - Scanning..."
        } else {
            "DupliFind"
        };
        let _ = tray.set_tooltip(tooltip);
    }
}

/// Update tray tooltip with detailed scan progress including percentage
/// Call this periodically during scans to keep tooltip updated
pub fn update_tray_progress(app: &AppHandle, progress: &ScanProgress) {
    if let Some(tray) = app.tray_handle() {
        let tooltip = format_scan_progress_tooltip(progress);
        let _ = tray.set_tooltip(&tooltip);
    }
}

/// Format scan progress for tray tooltip
fn format_scan_progress_tooltip(progress: &ScanProgress) -> String {
    let percent = if progress.total_files > 0 {
        (progress.files_processed as f64 / progress.total_files as f64 * 100.0) as u32
    } else {
        0
    };

    let phase_str = match progress.current_phase.as_str() {
        "scanning" => "Scanning files",
        "hashing" => "Computing hashes",
        "comparing" => "Finding duplicates",
        _ => "Processing",
    };

    let eta_str = if let Some(eta_seconds) = progress.eta_seconds {
        if eta_seconds < 60 {
            format!(" - {}s remaining", eta_seconds)
        } else if eta_seconds < 3600 {
            format!(" - {}m remaining", eta_seconds / 60)
        } else {
            format!(" - {}h {}m remaining", eta_seconds / 3600, (eta_seconds % 3600) / 60)
        }
    } else {
        String::new()
    };

    format!(
        "DupliFind - {} ({}%){}\n{} / {} files",
        phase_str,
        percent,
        eta_str,
        progress.files_processed,
        progress.total_files
    )
}

/// Scan progress info for tray updates
#[derive(Debug, Clone)]
pub struct ScanProgress {
    pub current_phase: String,
    pub files_processed: u64,
    pub total_files: u64,
    pub eta_seconds: Option<u64>,
}
```

#### 15.4.2 Update lib.rs to Initialize Tray

**File**: `src-tauri/src/lib.rs`

Add tray module and initialization:
```rust
mod tray;

// In the builder setup:
.system_tray(tray::create_tray())
.on_system_tray_event(tray::handle_tray_event)
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check` passes

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 15.5: Implement Window Minimize to Tray

### Overview
Intercept window close/minimize events and optionally minimize to tray instead.

### Changes Required

#### 15.5.1 Create Window Event Handler

**File**: `src-tauri/src/window_handler.rs`

```rust
//! Window event handling for minimize to tray

use crate::db::queries;
use crate::state::AppState;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, RunEvent, WindowEvent};

/// Check if minimize to tray is enabled
pub async fn is_minimize_to_tray_enabled(state: &Mutex<AppState>) -> bool {
    let state = match state.lock() {
        Ok(s) => s,
        Err(_) => return false,
    };

    let db = match state.database() {
        Some(db) => db,
        None => return false,
    };

    let db = db.blocking_lock();
    queries::settings::get(db.pool(), "minimize_to_tray")
        .await
        .ok()
        .flatten()
        .map(|s| s.value == "true")
        .unwrap_or(false)
}

/// Handle window close request - minimize to tray if enabled
pub fn handle_window_close(app: &AppHandle, window_label: &str) -> bool {
    if window_label != "main" {
        return false; // Allow other windows to close normally
    }

    let state = app.state::<Mutex<AppState>>();

    // Check setting synchronously
    let minimize_to_tray = tauri::async_runtime::block_on(async {
        is_minimize_to_tray_enabled(&state).await
    });

    if minimize_to_tray {
        // Hide window instead of closing
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        true // Prevent default close behavior
    } else {
        false // Allow normal close
    }
}
```

#### 15.5.2 Register Window Event Handler

Update `lib.rs` to handle window close events:

```rust
.on_window_event(|window, event| {
    if let WindowEvent::CloseRequested { api, .. } = event {
        if window_handler::handle_window_close(window.app_handle(), window.label()) {
            api.prevent_close();
        }
    }
})
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check` passes

#### Manual Verification
- [ ] With setting enabled, closing window hides to tray
- [ ] With setting disabled, closing window quits app

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 15.6: Add Tray Settings UI

### Overview
Add UI toggle for the minimize to tray setting.

### Changes Required

#### 15.6.1 Update Settings Panel

**File**: `src/lib/components/SettingsPanel.svelte`

Add after parallelism setting:

```svelte
<div class="setting-group">
  <label class="checkbox-label">
    <input
      type="checkbox"
      checked={settings.minimizeToTray}
      onchange={(e) => settingsStore.set('minimizeToTray', e.currentTarget.checked)}
    />
    <span>Minimize to system tray</span>
  </label>
  <p class="hint">
    When enabled, closing the window will minimize the app to the system tray
    instead of quitting. Click the tray icon to restore the window.
  </p>
</div>
```

Add styles:
```css
.checkbox-label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
}

.checkbox-label input[type="checkbox"] {
  width: 1.25rem;
  height: 1.25rem;
  cursor: pointer;
}

.checkbox-label span {
  font-weight: 500;
}
```

### Success Criteria

#### Automated Verification
- [ ] `npm run check` passes

#### Manual Verification
- [ ] Toggle appears in settings
- [ ] Setting persists across app restarts

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 15.7: Add Tray Commands

### Overview
Add Tauri commands for controlling tray behavior from frontend.

### Changes Required

#### 15.7.1 Create Tray Commands

**File**: `src-tauri/src/commands/tray.rs`

```rust
//! Tray-related Tauri commands

use tauri::{AppHandle, Manager};

/// Show the main window from tray
#[tauri::command]
pub async fn show_window(app_handle: AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Hide the main window to tray
#[tauri::command]
pub async fn hide_to_tray(app_handle: AppHandle) -> Result<(), String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        window.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Check if the app is currently hidden to tray
#[tauri::command]
pub async fn is_hidden(app_handle: AppHandle) -> Result<bool, String> {
    if let Some(window) = app_handle.get_webview_window("main") {
        Ok(!window.is_visible().unwrap_or(true))
    } else {
        Ok(false)
    }
}
```

#### 15.7.2 Register Commands

Add to commands module and register in lib.rs.

### Success Criteria

#### Automated Verification
- [ ] `cargo check` passes

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## Phase 15.8: Tests

### Overview
Add tests for tray functionality.

### Changes Required

Add tests for:
- Tray creation
- Menu item handling
- Window show/hide behavior
- Setting persistence

### Success Criteria

#### Automated Verification
- [ ] `cargo test tray` passes

### Code Review
Run code-review-fix-loop agent.

### Commit
Execute `/cl:commit`

---

## End of File 15

After completing all phases:
- System tray icon with menu
- "Show DupliFind" and "Quit" menu items
- Minimize to tray setting in UI
- Window hides to tray when setting enabled
- Click tray icon to restore window
- **Tray tooltip shows detailed scan progress including:**
  - Current phase (Scanning/Hashing/Finding duplicates)
  - Progress percentage (e.g., "45%")
  - Estimated time remaining
  - Files processed / total files

**Next**: This is the final feature file. Proceed to final testing and release preparation.
