# File 01: Project Foundation & Setup

## Overview

This file covers the initial project scaffolding, developer tooling, and basic application shell. By the end of this file, you'll have a working Tauri + Svelte application that can be built and run on both Mac and Windows.

## Prerequisites

Before starting, ensure you have:
- Node.js 20+ installed
- Rust toolchain installed (rustup)
- Platform-specific dependencies (covered in Phase 1.1)

---

## Phase 1.1: Create Setup Scripts

### Overview
Create developer setup scripts that check for and install prerequisites on Mac and Windows.

### Changes Required

#### 1.1.1 Create Mac Setup Script

**File**: `scripts/setup-mac.sh`

```bash
#!/bin/bash
set -e

echo "🔧 DupliFind - Mac Development Setup"
echo "======================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if a command exists
check_command() {
    if command -v "$1" &> /dev/null; then
        echo -e "${GREEN}✓${NC} $1 is installed"
        return 0
    else
        echo -e "${RED}✗${NC} $1 is not installed"
        return 1
    fi
}

# Check Xcode Command Line Tools
echo ""
echo "Checking Xcode Command Line Tools..."
if xcode-select -p &> /dev/null; then
    echo -e "${GREEN}✓${NC} Xcode Command Line Tools installed"
else
    echo -e "${YELLOW}Installing Xcode Command Line Tools...${NC}"
    xcode-select --install
    echo "Please complete the installation dialog, then run this script again."
    exit 1
fi

# Check Homebrew
echo ""
echo "Checking Homebrew..."
if ! check_command brew; then
    echo -e "${YELLOW}Installing Homebrew...${NC}"
    /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

    # Add to path for Apple Silicon
    if [[ $(uname -m) == 'arm64' ]]; then
        echo 'eval "$(/opt/homebrew/bin/brew shellenv)"' >> ~/.zprofile
        eval "$(/opt/homebrew/bin/brew shellenv)"
    fi
fi

# Check Node.js
echo ""
echo "Checking Node.js..."
if check_command node; then
    NODE_VERSION=$(node -v | cut -d'v' -f2 | cut -d'.' -f1)
    if [ "$NODE_VERSION" -ge 20 ]; then
        echo -e "${GREEN}✓${NC} Node.js version is 20+"
    else
        echo -e "${YELLOW}Node.js version is below 20. Installing latest LTS...${NC}"
        brew install node@20
    fi
else
    echo -e "${YELLOW}Installing Node.js...${NC}"
    brew install node@20
fi

# Check Rust
echo ""
echo "Checking Rust..."
if check_command rustc; then
    RUST_VERSION=$(rustc --version)
    echo -e "${GREEN}✓${NC} Rust: $RUST_VERSION"
else
    echo -e "${YELLOW}Installing Rust...${NC}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi

# Update Rust to latest stable
echo ""
echo "Updating Rust to latest stable..."
rustup update stable
rustup default stable

# Check for required Rust targets (for cross-compilation if needed)
echo ""
echo "Adding required Rust targets..."
rustup target add aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null || true

# Install Tauri CLI
echo ""
echo "Checking Tauri CLI..."
if cargo install --list | grep -q "tauri-cli"; then
    echo -e "${GREEN}✓${NC} Tauri CLI is installed"
else
    echo -e "${YELLOW}Installing Tauri CLI...${NC}"
    cargo install tauri-cli
fi

# Verify all installations
echo ""
echo "======================================"
echo "Verification"
echo "======================================"
echo ""

ALL_GOOD=true

check_command node || ALL_GOOD=false
check_command npm || ALL_GOOD=false
check_command rustc || ALL_GOOD=false
check_command cargo || ALL_GOOD=false

if [ "$ALL_GOOD" = true ]; then
    echo ""
    echo -e "${GREEN}✓ All prerequisites are installed!${NC}"
    echo ""
    echo "Next steps:"
    echo "  1. Run 'npm install' to install Node dependencies"
    echo "  2. Run 'npm run tauri dev' to start development"
    echo ""
else
    echo ""
    echo -e "${RED}Some prerequisites are missing. Please install them and run this script again.${NC}"
    exit 1
fi
```

#### 1.1.2 Create Windows Setup Script

**File**: `scripts/setup-windows.ps1`

```powershell
# DupliFind - Windows Development Setup
# Run this script in PowerShell as Administrator

Write-Host "DupliFind - Windows Development Setup" -ForegroundColor Cyan
Write-Host "======================================" -ForegroundColor Cyan

$ErrorActionPreference = "Stop"

function Test-Command {
    param([string]$Command)
    try {
        if (Get-Command $Command -ErrorAction SilentlyContinue) {
            return $true
        }
    } catch {
        return $false
    }
    return $false
}

function Write-Status {
    param([string]$Message, [bool]$Success)
    if ($Success) {
        Write-Host "[OK] $Message" -ForegroundColor Green
    } else {
        Write-Host "[MISSING] $Message" -ForegroundColor Red
    }
}

# Check for winget (Windows Package Manager)
Write-Host "`nChecking Windows Package Manager..." -ForegroundColor Yellow
if (!(Test-Command "winget")) {
    Write-Host "winget is not available. Please install App Installer from Microsoft Store." -ForegroundColor Red
    Write-Host "https://www.microsoft.com/store/productId/9NBLGGH4NNS1" -ForegroundColor Yellow
    exit 1
}
Write-Status "winget is installed" $true

# Check/Install Visual Studio Build Tools
Write-Host "`nChecking Visual Studio Build Tools..." -ForegroundColor Yellow
$vsWhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasBuildTools = $false

if (Test-Path $vsWhere) {
    $vsInstalls = & $vsWhere -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if ($vsInstalls) {
        $hasBuildTools = $true
    }
}

if ($hasBuildTools) {
    Write-Status "Visual Studio Build Tools are installed" $true
} else {
    Write-Host "Installing Visual Studio Build Tools..." -ForegroundColor Yellow
    Write-Host "This may take several minutes..." -ForegroundColor Gray
    winget install Microsoft.VisualStudio.2022.BuildTools --override "--quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    Write-Host "Please restart your terminal after installation completes." -ForegroundColor Yellow
}

# Check/Install WebView2
Write-Host "`nChecking WebView2 Runtime..." -ForegroundColor Yellow
$webview2 = Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue

if ($webview2) {
    Write-Status "WebView2 Runtime is installed" $true
} else {
    Write-Host "Installing WebView2 Runtime..." -ForegroundColor Yellow
    winget install Microsoft.EdgeWebView2Runtime
}

# Check/Install Node.js
Write-Host "`nChecking Node.js..." -ForegroundColor Yellow
if (Test-Command "node") {
    $nodeVersion = (node -v).Replace("v", "").Split(".")[0]
    if ([int]$nodeVersion -ge 20) {
        Write-Status "Node.js v$nodeVersion is installed (20+ required)" $true
    } else {
        Write-Host "Node.js version is below 20. Installing latest LTS..." -ForegroundColor Yellow
        winget install OpenJS.NodeJS.LTS
    }
} else {
    Write-Host "Installing Node.js LTS..." -ForegroundColor Yellow
    winget install OpenJS.NodeJS.LTS
}

# Check/Install Rust
Write-Host "`nChecking Rust..." -ForegroundColor Yellow
if (Test-Command "rustc") {
    $rustVersion = rustc --version
    Write-Status "Rust is installed: $rustVersion" $true

    # Update to latest stable
    Write-Host "Updating Rust to latest stable..." -ForegroundColor Gray
    rustup update stable
} else {
    Write-Host "Installing Rust..." -ForegroundColor Yellow
    winget install Rustlang.Rustup

    # Initialize rustup
    Write-Host "Please restart your terminal and run this script again to complete Rust setup." -ForegroundColor Yellow
    exit 0
}

# Install Tauri CLI
Write-Host "`nChecking Tauri CLI..." -ForegroundColor Yellow
$tauriInstalled = cargo install --list 2>$null | Select-String "tauri-cli"
if ($tauriInstalled) {
    Write-Status "Tauri CLI is installed" $true
} else {
    Write-Host "Installing Tauri CLI..." -ForegroundColor Yellow
    cargo install tauri-cli
}

# Final verification
Write-Host "`n======================================" -ForegroundColor Cyan
Write-Host "Verification" -ForegroundColor Cyan
Write-Host "======================================`n" -ForegroundColor Cyan

$allGood = $true

if (Test-Command "node") {
    Write-Status "Node.js" $true
} else {
    Write-Status "Node.js" $false
    $allGood = $false
}

if (Test-Command "npm") {
    Write-Status "npm" $true
} else {
    Write-Status "npm" $false
    $allGood = $false
}

if (Test-Command "rustc") {
    Write-Status "Rust (rustc)" $true
} else {
    Write-Status "Rust (rustc)" $false
    $allGood = $false
}

if (Test-Command "cargo") {
    Write-Status "Cargo" $true
} else {
    Write-Status "Cargo" $false
    $allGood = $false
}

if ($allGood) {
    Write-Host "`nAll prerequisites are installed!" -ForegroundColor Green
    Write-Host "`nNext steps:" -ForegroundColor Cyan
    Write-Host "  1. Run 'npm install' to install Node dependencies"
    Write-Host "  2. Run 'npm run tauri dev' to start development"
} else {
    Write-Host "`nSome prerequisites are missing." -ForegroundColor Red
    Write-Host "Please restart your terminal and run this script again." -ForegroundColor Yellow
    exit 1
}
```

### Success Criteria

#### Automated Verification
- [x] `ls scripts/setup-mac.sh scripts/setup-windows.ps1` shows both files exist
- [x] `bash -n scripts/setup-mac.sh` passes syntax check (no output = success)

#### Manual Verification
- [x] On Mac: `chmod +x scripts/setup-mac.sh && ./scripts/setup-mac.sh` runs without errors
- [ ] On Windows: PowerShell script runs and shows all prerequisites

### Code Review
Run background code-reviewer agent on `scripts/setup-mac.sh` and `scripts/setup-windows.ps1`. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 1.2: Initialize Tauri Project

### Overview
Create the base Tauri + Svelte project structure using the official Tauri create tool, then customize for our needs.

### Changes Required

#### 1.2.1 Create Tauri Project

Run the following command to scaffold the project:

```bash
npm create tauri-app@latest . -- --template svelte-ts --manager npm
```

This will create:
- `package.json` with Svelte and Tauri dependencies
- `vite.config.ts` for Vite bundler
- `svelte.config.js` for Svelte configuration
- `tsconfig.json` for TypeScript
- `src/` directory with frontend code
- `src-tauri/` directory with Rust backend

#### 1.2.2 Update package.json

**File**: `package.json`

After scaffolding, update the package.json to include all required dependencies and scripts:

```json
{
  "name": "duplifind",
  "version": "0.1.0",
  "description": "Cross-platform duplicate file finder",
  "type": "module",
  "scripts": {
    "dev": "vite dev",
    "build": "vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "check": "svelte-check --tsconfig ./tsconfig.json",
    "lint": "eslint . --ext .ts,.svelte",
    "lint:fix": "eslint . --ext .ts,.svelte --fix",
    "format": "prettier --write .",
    "test": "vitest run",
    "test:watch": "vitest",
    "test:coverage": "vitest run --coverage"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.9.0",
    "@tauri-apps/plugin-shell": "^2.2.0"
  },
  "devDependencies": {
    "@sveltejs/vite-plugin-svelte": "^5.0.0",
    "@tauri-apps/cli": "^2.9.0",
    "@types/node": "^22.0.0",
    "svelte": "^5.0.0",
    "svelte-check": "^4.0.0",
    "typescript": "^5.6.0",
    "vite": "^6.0.0",
    "vitest": "^2.0.0",
    "@vitest/browser": "^2.0.0",
    "playwright": "^1.48.0",
    "eslint": "^9.0.0",
    "prettier": "^3.4.0",
    "prettier-plugin-svelte": "^3.3.0"
  }
}
```

### Success Criteria

#### Automated Verification
- [x] `ls package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json` shows all files exist
- [x] `npm install` completes without errors
- [x] `cargo check --manifest-path src-tauri/Cargo.toml` passes

#### Manual Verification
- [x] Project structure matches expected layout
- [x] `npm run dev` starts the Vite dev server (Ctrl+C to exit)

### Code Review
Run background code-reviewer agent on `package.json`. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 1.3: Configure Tauri Application

### Overview
Configure the Tauri application settings, window properties, and application metadata.

### Changes Required

#### 1.3.1 Update Tauri Configuration

**File**: `src-tauri/tauri.conf.json`

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "DupliFind",
  "version": "0.1.0",
  "identifier": "com.duplifind.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:5173",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "DupliFind",
        "width": 1200,
        "height": 800,
        "minWidth": 900,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false,
        "center": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; style-src 'self' 'unsafe-inline'"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "macOS": {
      "minimumSystemVersion": "10.15"
    },
    "windows": {
      "webviewInstallMode": {
        "type": "embedBootstrapper"
      }
    }
  }
}
```

**Note on CSP**: The `style-src 'unsafe-inline'` directive is intentionally required because Svelte component scoped styles inject inline `<style>` tags at runtime. Without this directive, the styles would be blocked by the Content Security Policy.

#### 1.3.2 Create Capabilities Configuration

**File**: `src-tauri/capabilities/default.json`

```json
{
  "$schema": "https://schema.tauri.app/config/2/capability",
  "identifier": "default",
  "description": "Default capabilities for the main window",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open"
  ]
}
```

#### 1.3.3 Update Cargo.toml

**File**: `src-tauri/Cargo.toml`

```toml
[package]
name = "duplifind"
version = "0.1.0"
description = "Cross-platform duplicate file finder"
authors = ["DupliFind Team"]
edition = "2021"
license = "MIT"

[lib]
name = "duplifind_lib"
crate-type = ["staticlib", "cdylib", "lib"]

[[bin]]
name = "duplifind"
path = "src/main.rs"

[build-dependencies]
tauri-build = { version = "2.0", features = [] }

[dependencies]
tauri = { version = "2.0", features = [] }
tauri-plugin-shell = "2.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

[profile.dev]
incremental = true

[profile.release]
panic = "abort"
codegen-units = 1
lto = true
opt-level = "s"
strip = true
```

### Success Criteria

#### Automated Verification
- [x] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [x] `cat src-tauri/tauri.conf.json | jq .productName` outputs "DupliFind"
- [x] `ls src-tauri/capabilities/default.json` confirms capabilities file exists

#### Manual Verification
- [x] Configuration values match specification requirements

### Code Review
Run background code-reviewer agent on `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, and `src-tauri/capabilities/default.json`. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 1.4: Setup Rust Project Structure

### Overview
Create the modular Rust backend structure with proper separation of concerns.

### Changes Required

#### 1.4.1 Create Module Structure

**File**: `src-tauri/src/lib.rs`

```rust
// DupliFind - Main library entry point

mod commands;
mod state;

use state::AppState;
use std::sync::Mutex;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Initialize application state
            let state = AppState::new();
            app.manage(Mutex::new(state));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::greet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

#### 1.4.2 Create State Module

**File**: `src-tauri/src/state.rs`

```rust
//! Application state management

/// Global application state
pub struct AppState {
    /// Flag indicating if a scan is currently running
    pub is_scanning: bool,
}

impl AppState {
    /// Create a new application state
    pub fn new() -> Self {
        Self {
            is_scanning: false,
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_new() {
        let state = AppState::new();
        assert!(!state.is_scanning);
    }

    #[test]
    fn test_app_state_default() {
        let state = AppState::default();
        assert!(!state.is_scanning);
    }
}
```

#### 1.4.3 Create Commands Module

**File**: `src-tauri/src/commands/mod.rs`

```rust
//! Tauri command handlers

/// Simple greet command for testing
#[tauri::command]
pub fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to DupliFind.", name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let result = greet("World");
        assert_eq!(result, "Hello, World! Welcome to DupliFind.");
    }

    #[test]
    fn test_greet_empty() {
        let result = greet("");
        assert_eq!(result, "Hello, ! Welcome to DupliFind.");
    }
}
```

#### 1.4.4 Update main.rs

**File**: `src-tauri/src/main.rs`

```rust
// Prevents additional console window on Windows in release
#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

fn main() {
    duplifind_lib::run();
}
```

### Success Criteria

#### Automated Verification
- [x] `cargo check --manifest-path src-tauri/Cargo.toml` passes
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` passes (all tests pass)
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml` shows no warnings

#### Manual Verification
- [x] Module structure is clean and follows Rust conventions

### Code Review
Run background code-reviewer agent on all new Rust files. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 1.5: Setup Frontend Structure

### Overview
Create the Svelte frontend structure with TypeScript configuration and basic components.

### Changes Required

#### 1.5.1 Update Vite Configuration

**File**: `vite.config.ts`

```typescript
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [svelte()],

  // Vite options tailored for Tauri development
  clearScreen: false,

  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },

  // Environment variables that start with TAURI_ will be exposed
  envPrefix: ['VITE_', 'TAURI_'],

  build: {
    // Tauri uses Chromium on Windows and WebKit on macOS and Linux
    target: process.env.TAURI_PLATFORM === 'windows' ? 'chrome105' : 'safari15',
    // Don't minify for debug builds
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    // Produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
```

#### 1.5.2 Update TypeScript Configuration

**File**: `tsconfig.json`

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "moduleResolution": "bundler",
    "strict": true,
    "noEmit": true,
    "isolatedModules": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "allowImportingTsExtensions": true,
    "verbatimModuleSyntax": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true,
    "types": ["vite/client"],
    "paths": {
      "$lib/*": ["./src/lib/*"]
    }
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"],
  "references": [{ "path": "./tsconfig.node.json" }]
}
```

#### 1.5.3 Create tsconfig.node.json

**File**: `tsconfig.node.json`

```json
{
  "compilerOptions": {
    "composite": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "allowSyntheticDefaultImports": true,
    "strict": true,
    "noEmit": true
  },
  "include": ["vite.config.ts"]
}
```

#### 1.5.4 Create Main HTML

**File**: `index.html`

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>DupliFind</title>
    <link rel="stylesheet" href="/src/app.css" />
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/src/main.ts"></script>
  </body>
</html>
```

### Success Criteria

#### Automated Verification
- [x] `npm run check` passes (svelte-check)
- [x] `ls vite.config.ts tsconfig.json tsconfig.node.json index.html` shows all files

#### Manual Verification
- [x] TypeScript configuration is strict and appropriate for the project

### Code Review
Run background code-reviewer agent on configuration files. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 1.6: Create Base Svelte Application

### Overview
Create the main Svelte application entry point and base styles.

### Changes Required

#### 1.6.1 Create Main Entry Point

**File**: `src/main.ts`

```typescript
import App from './App.svelte';
import { mount } from 'svelte';

const app = mount(App, {
  target: document.getElementById('app')!,
});

export default app;
```

#### 1.6.2 Create Base App Component

**File**: `src/App.svelte`

```svelte
<script lang="ts">
  import { invoke } from '@tauri-apps/api/core';

  let name = $state('');
  let greeting = $state('');

  async function greet() {
    greeting = await invoke('greet', { name });
  }
</script>

<main>
  <h1>DupliFind</h1>
  <p class="subtitle">Find and remove duplicate files</p>

  <div class="test-section">
    <h2>Connection Test</h2>
    <form onsubmit={(e) => { e.preventDefault(); greet(); }}>
      <input
        type="text"
        bind:value={name}
        placeholder="Enter your name"
      />
      <button type="submit">Test Backend</button>
    </form>
    {#if greeting}
      <p class="greeting">{greeting}</p>
    {/if}
  </div>

  <div class="info">
    <p>This is a placeholder UI. The full interface will be built in subsequent phases.</p>
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    min-height: 100vh;
    padding: 2rem;
  }

  h1 {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
  }

  .subtitle {
    color: var(--text-secondary);
    margin-bottom: 2rem;
  }

  .test-section {
    background: var(--surface);
    padding: 2rem;
    border-radius: 8px;
    margin-bottom: 2rem;
    width: 100%;
    max-width: 400px;
  }

  .test-section h2 {
    font-size: 1.2rem;
    margin-bottom: 1rem;
  }

  form {
    display: flex;
    gap: 0.5rem;
  }

  input {
    flex: 1;
    padding: 0.5rem 1rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--background);
    color: var(--text);
  }

  button {
    padding: 0.5rem 1rem;
    border: none;
    border-radius: 4px;
    background: var(--primary);
    color: white;
    cursor: pointer;
    font-weight: 500;
  }

  button:hover {
    opacity: 0.9;
  }

  .greeting {
    margin-top: 1rem;
    padding: 1rem;
    background: var(--success-bg);
    border-radius: 4px;
    color: var(--success);
  }

  .info {
    color: var(--text-secondary);
    font-size: 0.875rem;
    text-align: center;
  }
</style>
```

#### 1.6.3 Create Base CSS

**File**: `src/app.css`

```css
/* DupliFind - Base Styles */

/* CSS Custom Properties (Light Theme) */
:root {
  --background: #ffffff;
  --surface: #f5f5f5;
  --text: #1a1a1a;
  --text-secondary: #666666;
  --border: #e0e0e0;
  --primary: #2563eb;
  --primary-hover: #1d4ed8;
  --success: #16a34a;
  --success-bg: #dcfce7;
  --warning: #ca8a04;
  --warning-bg: #fef9c3;
  --error: #dc2626;
  --error-bg: #fee2e2;

  /* Font */
  --font-sans: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
  --font-mono: ui-monospace, 'SF Mono', Menlo, Monaco, 'Cascadia Mono', monospace;
}

/* Dark Theme */
@media (prefers-color-scheme: dark) {
  :root {
    --background: #1a1a1a;
    --surface: #2a2a2a;
    --text: #ffffff;
    --text-secondary: #a0a0a0;
    --border: #404040;
    --primary: #3b82f6;
    --primary-hover: #60a5fa;
    --success: #22c55e;
    --success-bg: #14532d;
    --warning: #eab308;
    --warning-bg: #422006;
    --error: #ef4444;
    --error-bg: #450a0a;
  }
}

/* Reset */
*,
*::before,
*::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

/* Base */
html {
  font-family: var(--font-sans);
  font-size: 16px;
  line-height: 1.5;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

body {
  background-color: var(--background);
  color: var(--text);
  min-height: 100vh;
}

/* Disable text selection and drag on non-input elements for native feel */
body:not(input):not(textarea) {
  user-select: none;
  -webkit-user-select: none;
}

/* Re-enable selection for text content */
p, span, h1, h2, h3, h4, h5, h6 {
  user-select: text;
  -webkit-user-select: text;
}

/* Links */
a {
  color: var(--primary);
  text-decoration: none;
}

a:hover {
  text-decoration: underline;
}

/* Focus styles */
:focus-visible {
  outline: 2px solid var(--primary);
  outline-offset: 2px;
}

/* Scrollbar styling */
::-webkit-scrollbar {
  width: 8px;
  height: 8px;
}

::-webkit-scrollbar-track {
  background: var(--surface);
}

::-webkit-scrollbar-thumb {
  background: var(--border);
  border-radius: 4px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--text-secondary);
}

/* Utility classes */
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  padding: 0;
  margin: -1px;
  overflow: hidden;
  clip: rect(0, 0, 0, 0);
  white-space: nowrap;
  border-width: 0;
}
```

### Success Criteria

#### Automated Verification
- [x] `npm run check` passes
- [x] `ls src/main.ts src/App.svelte src/app.css` shows all files

#### Manual Verification
- [x] `npm run tauri dev` launches the application
- [x] Light/dark theme follows system preference
- [x] Backend test (greet) works correctly

### Code Review
Run background code-reviewer agent on Svelte and CSS files. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 1.7: Create README Documentation

### Overview
Create comprehensive README with setup instructions for developers.

### Changes Required

#### 1.7.1 Create README

**File**: `README.md`

```markdown
# DupliFind

A cross-platform desktop application for finding and removing duplicate files on Mac and Windows systems.

## Features

- **Smart Duplicate Detection**: Uses multi-stage detection (file size → partial hash → full hash) for efficient scanning
- **Fast Hashing**: Powered by BLAKE3 for cryptographically secure, high-performance hashing
- **Safe Deletion**: Files are moved to system Trash/Recycle Bin, not permanently deleted
- **Protected Folders**: Designate folders as protected to prevent accidental deletion
- **Incremental Scanning**: Quick scan mode uses cached hashes for faster subsequent scans
- **Cross-Platform**: Native experience on both macOS and Windows

## Tech Stack

- **Framework**: [Tauri 2.x](https://v2.tauri.app/) (Rust backend + web frontend)
- **Frontend**: [Svelte 5](https://svelte.dev/) + TypeScript
- **Database**: SQLite
- **Hash Algorithm**: BLAKE3

## Prerequisites

### macOS

- macOS 10.15 or later
- Xcode Command Line Tools
- Node.js 20+
- Rust toolchain

Run the setup script to install prerequisites:

```bash
chmod +x scripts/setup-mac.sh
./scripts/setup-mac.sh
```

### Windows

- Windows 10 (1803+) or Windows 11
- Visual Studio Build Tools 2022
- WebView2 Runtime
- Node.js 20+
- Rust toolchain

Run the setup script in PowerShell as Administrator:

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
.\scripts\setup-windows.ps1
```

## Getting Started

1. **Clone the repository**

   ```bash
   git clone <repository-url>
   cd duplicate-file-finder-CC-n-Interview
   ```

2. **Install dependencies**

   ```bash
   npm install
   ```

3. **Start development server**

   ```bash
   npm run tauri dev
   ```

   This will compile the Rust backend and start the frontend with hot reload.

4. **Build for production**

   ```bash
   npm run tauri build
   ```

   Built applications will be in `src-tauri/target/release/bundle/`.

## Development Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start Vite dev server (frontend only) |
| `npm run tauri dev` | Start Tauri dev mode with hot reload |
| `npm run build` | Build frontend for production |
| `npm run tauri build` | Build complete application |
| `npm run check` | Run Svelte type checking |
| `npm run lint` | Run ESLint |
| `npm run test` | Run frontend tests |
| `cargo test` | Run Rust tests (from src-tauri/) |
| `cargo clippy` | Run Rust linter (from src-tauri/) |

## Project Structure

```
.
├── src/                    # Frontend (Svelte + TypeScript)
│   ├── lib/               # Shared components and utilities
│   ├── App.svelte         # Main application component
│   ├── app.css            # Global styles
│   └── main.ts            # Entry point
├── src-tauri/             # Backend (Rust)
│   ├── src/
│   │   ├── commands/      # Tauri command handlers
│   │   ├── lib.rs         # Library entry point
│   │   └── state.rs       # Application state
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── scripts/               # Setup and build scripts
└── plans/                 # Implementation plans
```

## License

MIT
```

### Success Criteria

#### Automated Verification
- [x] `ls README.md` shows file exists
- [x] `head -1 README.md` shows "# DupliFind"

#### Manual Verification
- [x] README is comprehensive and follows standard format
- [x] All commands work as documented

### Code Review
Run background code-reviewer agent on README.md. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## Phase 1.8: Add Testing Infrastructure

### Overview
Set up Vitest for frontend component testing and verify Rust test infrastructure.

### Changes Required

#### 1.8.1 Create Vitest Configuration

**File**: `vitest.config.ts`

```typescript
import { defineConfig } from 'vitest/config';
import { svelte } from '@sveltejs/vite-plugin-svelte';

export default defineConfig({
  plugins: [svelte({ hot: !process.env.VITEST })],
  test: {
    include: ['src/**/*.{test,spec}.{js,ts}', 'tests/**/*.{test,spec}.{js,ts}'],
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./tests/setup.ts'],
  },
});
```

#### 1.8.2 Create Test Setup File

**File**: `tests/setup.ts`

```typescript
// Test setup file for Vitest

import { vi } from 'vitest';

// Mock Tauri APIs for testing
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

// Reset mocks before each test
beforeEach(() => {
  vi.clearAllMocks();
});
```

#### 1.8.3 Create Sample Test

**File**: `tests/example.test.ts`

```typescript
import { describe, it, expect, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';

describe('Tauri API Mock', () => {
  it('should mock invoke correctly', async () => {
    const mockInvoke = vi.mocked(invoke);
    mockInvoke.mockResolvedValue('Hello, Test! Welcome to DupliFind.');

    const result = await invoke('greet', { name: 'Test' });

    expect(result).toBe('Hello, Test! Welcome to DupliFind.');
    expect(mockInvoke).toHaveBeenCalledWith('greet', { name: 'Test' });
  });
});

describe('Basic Tests', () => {
  it('should pass a simple test', () => {
    expect(1 + 1).toBe(2);
  });

  it('should handle string operations', () => {
    const str = 'DupliFind';
    expect(str.toLowerCase()).toBe('duplifind');
  });
});
```

#### 1.8.4 Update package.json test scripts

Ensure package.json has the test scripts (already added in Phase 1.2, but verify):

```json
{
  "scripts": {
    "test": "vitest run",
    "test:watch": "vitest",
    "test:coverage": "vitest run --coverage"
  }
}
```

### Success Criteria

#### Automated Verification
- [ ] `npm run test` passes all tests
- [ ] `cargo test --manifest-path src-tauri/Cargo.toml` passes all Rust tests
- [ ] `ls vitest.config.ts tests/setup.ts tests/example.test.ts` shows all files

#### Manual Verification
- [ ] `npm run test:watch` works and shows passing tests
- [ ] Test infrastructure is ready for component tests

### Code Review
Run background code-reviewer agent on test configuration files. Iterate until "Code looks good. No significant issues found."

### Commit
Execute `/cl:commit` to commit changes with meaningful message.

---

## End of File 01

After completing all phases in this file, you should have:

1. Developer setup scripts for Mac and Windows
2. A working Tauri + Svelte project structure
3. Configured Tauri application with proper settings
4. Modular Rust backend structure
5. Base Svelte frontend with theming support
6. Comprehensive README documentation
7. Testing infrastructure for both frontend and backend

**Next**: Proceed to [02-database-foundation.md](./02-database-foundation.md) to set up the SQLite database layer.
