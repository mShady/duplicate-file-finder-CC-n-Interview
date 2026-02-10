# File 12: Permissions & Onboarding

## Overview

This file covers implementing the blocking permission wizard, Full Disk Access guide for macOS, and equivalent Windows permission handling.

## Prerequisites

- Completed Files 01-11

---

## Phase 12.1: Create Permission Check Commands

### Overview
Create Rust commands to check for necessary file system permissions.

### Changes Required

**File**: `src-tauri/src/commands/permissions.rs`

```rust
use std::path::Path;

/// Check if we have read access to a directory
#[tauri::command]
pub async fn check_directory_access(path: String) -> Result<bool, String> {
    let path = Path::new(&path);

    if !path.exists() {
        return Ok(false);
    }

    // Try to read the directory
    match std::fs::read_dir(path) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

/// Check if Full Disk Access is granted (macOS specific)
#[tauri::command]
pub async fn check_full_disk_access() -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        // Check access to a protected directory that requires FDA
        let protected_paths = [
            "/Library/Application Support/com.apple.TCC",
            dirs::home_dir().map(|h| h.join("Library/Safari")),
        ];

        for path in protected_paths.iter().filter_map(|p| p.as_ref()) {
            if let Ok(entries) = std::fs::read_dir(path) {
                // If we can read entries, we likely have FDA
                if entries.count() > 0 {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    #[cfg(not(target_os = "macos"))]
    {
        // On Windows, we don't need FDA
        Ok(true)
    }
}

/// Get system information for permission guidance
#[tauri::command]
pub async fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        os: std::env::consts::OS.to_string(),
        os_version: os_version().unwrap_or_default(),
        arch: std::env::consts::ARCH.to_string(),
    })
}

#[derive(serde::Serialize)]
pub struct SystemInfo {
    os: String,
    os_version: String,
    arch: String,
}

fn os_version() -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let output = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    #[cfg(target_os = "windows")]
    {
        // Windows version detection
        Some("10+".to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}
```

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 12.2: Create Permission Wizard Component

### Overview
Create the blocking permission wizard UI.

### Changes Required

**File**: `src/lib/components/PermissionWizard.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';
  import { onMount } from 'svelte';

  interface Props {
    onComplete: () => void;
  }

  let { onComplete }: Props = $props();

  let checking = $state(true);
  let hasAccess = $state(false);
  let platform = $state<'macos' | 'windows' | 'linux'>('macos');
  let currentStep = $state(0);

  onMount(async () => {
    const info = await invoke<{ os: string }>('get_system_info');
    platform = info.os === 'macos' ? 'macos' : info.os === 'windows' ? 'windows' : 'linux';

    await checkPermissions();
  });

  async function checkPermissions() {
    checking = true;
    try {
      hasAccess = await invoke<boolean>('check_full_disk_access');
      if (hasAccess) {
        onComplete();
      }
    } catch (e) {
      console.error('Permission check failed:', e);
    } finally {
      checking = false;
    }
  }

  function openSystemPreferences() {
    invoke('open_file', {
      path: 'x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles',
    });
  }

  const macSteps = [
    {
      title: 'Open System Settings',
      description: 'Click the button below to open Privacy & Security settings.',
      action: openSystemPreferences,
      actionLabel: 'Open System Settings',
    },
    {
      title: 'Navigate to Full Disk Access',
      description: 'In the sidebar, click "Full Disk Access" under Privacy.',
    },
    {
      title: 'Add DupliFind',
      description: 'Click the + button and add DupliFind from your Applications folder.',
    },
    {
      title: 'Restart DupliFind',
      description: 'Close and reopen DupliFind for the changes to take effect.',
    },
  ];
</script>

<div class="wizard">
  {#if checking}
    <div class="checking">
      <div class="spinner"></div>
      <p>Checking permissions...</p>
    </div>
  {:else if !hasAccess}
    <div class="wizard-content">
      <h1>Permission Required</h1>
      <p class="intro">
        DupliFind needs Full Disk Access to scan all your files for duplicates.
        Without this permission, some files may be inaccessible.
      </p>

      {#if platform === 'macos'}
        <div class="steps">
          {#each macSteps as step, i}
            <div class="step" class:active={currentStep === i} class:completed={currentStep > i}>
              <div class="step-number">{i + 1}</div>
              <div class="step-content">
                <h3>{step.title}</h3>
                <p>{step.description}</p>
                {#if step.action && currentStep === i}
                  <button class="step-action" onclick={step.action}>
                    {step.actionLabel}
                  </button>
                {/if}
              </div>
            </div>
          {/each}
        </div>

        <div class="navigation">
          {#if currentStep > 0}
            <button class="nav-btn" onclick={() => (currentStep -= 1)}>Back</button>
          {/if}
          {#if currentStep < macSteps.length - 1}
            <button class="nav-btn primary" onclick={() => (currentStep += 1)}>Next</button>
          {:else}
            <button class="nav-btn primary" onclick={checkPermissions}>Check Again</button>
          {/if}
        </div>
      {:else}
        <p>Please ensure DupliFind has access to the folders you want to scan.</p>
        <button class="nav-btn primary" onclick={onComplete}>Continue</button>
      {/if}

      <button class="skip-btn" onclick={onComplete}>
        Skip (Some files may be inaccessible)
      </button>
    </div>
  {/if}
</div>

<style>
  .wizard {
    min-height: 100vh;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
  }

  .checking {
    text-align: center;
  }

  .spinner {
    width: 48px;
    height: 48px;
    border: 4px solid var(--border);
    border-top-color: var(--primary);
    border-radius: 50%;
    margin: 0 auto 1rem;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .wizard-content {
    max-width: 600px;
    text-align: center;
  }

  h1 {
    margin-bottom: 1rem;
  }

  .intro {
    color: var(--text-secondary);
    margin-bottom: 2rem;
  }

  .steps {
    text-align: left;
    margin-bottom: 2rem;
  }

  .step {
    display: flex;
    gap: 1rem;
    padding: 1rem;
    border-radius: 8px;
    margin-bottom: 0.5rem;
    background: var(--surface);
    opacity: 0.6;
  }

  .step.active {
    opacity: 1;
    border: 2px solid var(--primary);
  }

  .step.completed {
    opacity: 0.8;
  }

  .step-number {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    background: var(--primary);
    color: white;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
    flex-shrink: 0;
  }

  .step-content h3 {
    margin: 0 0 0.5rem;
    font-size: 1rem;
  }

  .step-content p {
    margin: 0;
    font-size: 0.9rem;
    color: var(--text-secondary);
  }

  .step-action {
    margin-top: 0.75rem;
    padding: 0.5rem 1rem;
    background: var(--primary);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }

  .navigation {
    display: flex;
    gap: 0.5rem;
    justify-content: center;
    margin-bottom: 1rem;
  }

  .nav-btn {
    padding: 0.75rem 1.5rem;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
  }

  .nav-btn.primary {
    background: var(--primary);
    color: white;
    border-color: var(--primary);
  }

  .skip-btn {
    padding: 0.5rem;
    background: transparent;
    border: none;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.85rem;
  }

  .skip-btn:hover {
    text-decoration: underline;
  }
</style>
```

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 12.3: Add Permission State Persistence

### Overview
Remember if user has granted permissions or skipped.

### Changes Required

Store permission state in settings and check on app startup.

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 12.4: Create Windows Permission Wizard

### Overview
Create Windows-specific permission wizard with step-by-step guidance for folder access permissions, particularly for protected system folders and user directories.

### Windows Permission Scenarios

1. **Controlled Folder Access (Windows Defender)** - May block app from accessing protected folders
2. **User Account Control (UAC)** - May prompt for elevation
3. **NTFS Permissions** - Folder-level access restrictions
4. **Ransomware Protection** - Windows Security feature that blocks unauthorized apps

### Changes Required

#### 12.4.1 Update Permission Check for Windows

**File**: `src-tauri/src/commands/permissions.rs`

Add Windows-specific checks:

```rust
/// Check if we have access to protected folders (Windows specific)
#[tauri::command]
pub async fn check_windows_folder_access() -> Result<WindowsAccessStatus, String> {
    #[cfg(target_os = "windows")]
    {
        use std::path::Path;

        let test_paths = vec![
            dirs::document_dir(),
            dirs::download_dir(),
            dirs::desktop_dir(),
        ];

        let mut blocked_paths = Vec::new();

        for path in test_paths.into_iter().flatten() {
            match std::fs::read_dir(&path) {
                Ok(_) => {}
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::PermissionDenied {
                        blocked_paths.push(path.display().to_string());
                    }
                }
            }
        }

        Ok(WindowsAccessStatus {
            has_full_access: blocked_paths.is_empty(),
            blocked_paths,
            controlled_folder_access_enabled: check_controlled_folder_access(),
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Ok(WindowsAccessStatus {
            has_full_access: true,
            blocked_paths: vec![],
            controlled_folder_access_enabled: false,
        })
    }
}

#[derive(serde::Serialize)]
pub struct WindowsAccessStatus {
    pub has_full_access: bool,
    pub blocked_paths: Vec<String>,
    pub controlled_folder_access_enabled: bool,
}

#[cfg(target_os = "windows")]
fn check_controlled_folder_access() -> bool {
    // Check registry for Controlled Folder Access setting
    use winreg::enums::*;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    if let Ok(key) = hklm.open_subkey(
        "SOFTWARE\\Microsoft\\Windows Defender\\Windows Defender Exploit Guard\\Controlled Folder Access"
    ) {
        if let Ok(enabled) = key.get_value::<u32, _>("EnableControlledFolderAccess") {
            return enabled == 1;
        }
    }
    false
}
```

#### 12.4.2 Update Permission Wizard for Windows

**File**: `src/lib/components/PermissionWizard.svelte`

Add Windows-specific wizard steps:

```svelte
<script lang="ts">
  // ... existing imports ...

  // Windows-specific state
  let windowsStatus = $state<{
    has_full_access: boolean;
    blocked_paths: string[];
    controlled_folder_access_enabled: boolean;
  } | null>(null);

  // ... existing code ...

  const windowsSteps = [
    {
      title: 'Check Windows Security Settings',
      description: 'Open Windows Security to manage Controlled Folder Access, which may be blocking DupliFind.',
      action: () => invoke('open_file', { path: 'windowsdefender://threatsettings' }),
      actionLabel: 'Open Windows Security',
    },
    {
      title: 'Allow DupliFind Through Controlled Folder Access',
      description: 'In "Ransomware protection", click "Allow an app through Controlled folder access" and add DupliFind.',
    },
    {
      title: 'Check Blocked Folders',
      description: windowsStatus?.blocked_paths.length
        ? `The following folders are blocked: ${windowsStatus.blocked_paths.join(', ')}`
        : 'No folders are currently blocked.',
    },
    {
      title: 'Restart DupliFind',
      description: 'Close and reopen DupliFind for the changes to take effect.',
    },
  ];
</script>

<!-- In the template, add Windows support: -->
{#if platform === 'windows'}
  <div class="steps">
    {#each windowsSteps as step, i}
      <div class="step" class:active={currentStep === i} class:completed={currentStep > i}>
        <div class="step-number">{i + 1}</div>
        <div class="step-content">
          <h3>{step.title}</h3>
          <p>{step.description}</p>
          {#if step.action && currentStep === i}
            <button class="step-action" onclick={step.action}>
              {step.actionLabel}
            </button>
          {/if}
        </div>
      </div>
    {/each}
  </div>

  {#if windowsStatus?.controlled_folder_access_enabled}
    <div class="info-banner">
      <strong>Controlled Folder Access is enabled</strong>
      <p>This Windows Security feature protects your folders from unauthorized changes.
         You need to add DupliFind to the allowed apps list.</p>
    </div>
  {/if}

  {#if windowsStatus?.blocked_paths.length}
    <div class="warning-banner">
      <strong>Some folders are blocked:</strong>
      <ul>
        {#each windowsStatus.blocked_paths as path}
          <li>{path}</li>
        {/each}
      </ul>
    </div>
  {/if}

  <div class="navigation">
    {#if currentStep > 0}
      <button class="nav-btn" onclick={() => (currentStep -= 1)}>Back</button>
    {/if}
    {#if currentStep < windowsSteps.length - 1}
      <button class="nav-btn primary" onclick={() => (currentStep += 1)}>Next</button>
    {:else}
      <button class="nav-btn primary" onclick={checkPermissions}>Check Again</button>
    {/if}
  </div>
{/if}
```

Add styles for the info and warning banners:

```css
.info-banner {
  background: var(--primary-bg);
  border: 1px solid var(--primary);
  padding: 1rem;
  border-radius: 6px;
  margin-bottom: 1rem;
}

.info-banner strong {
  display: block;
  margin-bottom: 0.25rem;
  color: var(--primary);
}

.info-banner p {
  margin: 0;
  font-size: 0.9rem;
}

.warning-banner {
  background: var(--warning-bg);
  border: 1px solid var(--warning);
  padding: 1rem;
  border-radius: 6px;
  margin-bottom: 1rem;
}

.warning-banner strong {
  display: block;
  margin-bottom: 0.5rem;
  color: var(--warning);
}

.warning-banner ul {
  margin: 0;
  padding-left: 1.5rem;
  font-size: 0.85rem;
  font-family: var(--font-mono);
}
```

#### 12.4.3 Add Windows Registry Dependency

**File**: `src-tauri/Cargo.toml`

```toml
[target.'cfg(windows)'.dependencies]
winreg = "0.52"
```

### Success Criteria

#### Automated Verification
- [ ] `cargo check` passes on Windows
- [ ] `npm run check` passes

#### Manual Verification
- [ ] Windows wizard shows Controlled Folder Access status
- [ ] Blocked paths are listed when access is denied
- [ ] "Open Windows Security" button opens correct settings page
- [ ] Wizard guides user through adding DupliFind to allowed apps
- [ ] Access check updates after adding app to allowed list

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent on Windows permission code.

---

## Phase 12.5: Integrate Wizard in App Startup

### Overview
Show permission wizard on first launch or when access is lost.

### Changes Required

Update App.svelte to check permissions on mount and show wizard if needed.

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## Phase 12.6: Tests

Add tests for permission checking.

### Commit
Execute `/cl:commit`

### Code Review
Run code-review-fix-loop agent.

---

## End of File 12

After completing all phases:
- Permission check on startup
- Blocking permission wizard
- Step-by-step macOS Full Disk Access guide
- Windows permission handling
- Skip option with warning
- Permission state persistence

**Next**: Proceed to [13-error-handling.md](./13-error-handling.md)
