# DupliFind - Windows Development Setup
# Run this script in PowerShell as Administrator

# Check for Administrator privileges
if (-NOT ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole] "Administrator")) {
    Write-Host "This script requires Administrator privileges." -ForegroundColor Red
    Write-Host "Please right-click PowerShell and select 'Run as Administrator'." -ForegroundColor Yellow
    exit 1
}

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
# Check both 64-bit and 32-bit registry paths
$webview2 = Get-ItemProperty -Path "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
if (-not $webview2) {
    $webview2 = Get-ItemProperty -Path "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" -ErrorAction SilentlyContinue
}

if ($webview2) {
    Write-Status "WebView2 Runtime is installed" $true
} else {
    Write-Host "Installing WebView2 Runtime..." -ForegroundColor Yellow
    winget install Microsoft.EdgeWebView2Runtime
}

# Check/Install Node.js
Write-Host "`nChecking Node.js..." -ForegroundColor Yellow
if (Test-Command "node") {
    try {
        $nodeVersionStr = (node -v).Replace("v", "").Split(".")[0]
        $nodeVersion = $nodeVersionStr -as [int]
        if ($nodeVersion -and $nodeVersion -ge 20) {
            Write-Status "Node.js v$nodeVersion is installed (20+ required)" $true
        } else {
            Write-Host "Node.js version is below 20. Installing latest LTS..." -ForegroundColor Yellow
            winget install OpenJS.NodeJS.LTS
        }
    } catch {
        Write-Host "Could not determine Node.js version. Installing latest LTS..." -ForegroundColor Yellow
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
    cargo install tauri-cli@^2
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
