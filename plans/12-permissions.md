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

---

## Phase 12.3: Add Permission State Persistence

### Overview
Remember if user has granted permissions or skipped.

### Changes Required

Store permission state in settings and check on app startup.

### Commit
Execute `/cl:commit`

---

## Phase 12.4: Create Windows Permission Guide

### Overview
Create Windows-specific permission guidance (if needed).

### Changes Required

Add Windows-specific steps for folder access permissions.

### Commit
Execute `/cl:commit`

---

## Phase 12.5: Integrate Wizard in App Startup

### Overview
Show permission wizard on first launch or when access is lost.

### Changes Required

Update App.svelte to check permissions on mount and show wizard if needed.

### Commit
Execute `/cl:commit`

---

## Phase 12.6: Tests

Add tests for permission checking.

### Commit
Execute `/cl:commit`

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
