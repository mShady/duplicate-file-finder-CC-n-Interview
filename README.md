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
   npm run tauri:dev
   ```

   This will compile the Rust backend and start the frontend with hot reload.

4. **Build for production**

   ```bash
   npm run tauri:build
   ```

   Built applications will be in `src-tauri/target/release/bundle/`.

## Development Commands

| Command | Description |
|---------|-------------|
| `npm run dev` | Start Vite dev server (frontend only) |
| `npm run tauri:dev` | Start Tauri dev mode with hot reload |
| `npm run build` | Build frontend for production |
| `npm run tauri:build` | Build complete application |
| `npm run check` | Run Svelte type checking |
| `npm run lint` | Run ESLint |
| `npm run lint:fix` | Run ESLint with auto-fix |
| `npm run format` | Format code with Prettier |
| `npm run test` | Run frontend tests |
| `npm run test:watch` | Run tests in watch mode |
| `npm run test:coverage` | Run tests with coverage report |
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
