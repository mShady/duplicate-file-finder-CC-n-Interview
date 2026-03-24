# DupliFind

[![CI](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/actions/workflows/claude.yml/badge.svg)](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/actions/workflows/claude.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Status: Alpha](https://img.shields.io/badge/Status-Alpha-orange.svg)](#roadmap)

A desktop application for finding and removing duplicate files, built with Tauri 2 and Svelte 5. Currently targeting **macOS**, with Windows support planned for a later stage.

<!-- TODO: Add screenshot or GIF demo here -->
<!-- ![DupliFind Screenshot](docs/assets/screenshot.png) -->

## Features

- **Smart Duplicate Detection**: Uses multi-stage detection (file size → partial hash → full hash) for efficient scanning
- **Fast Hashing**: Powered by BLAKE3 for cryptographically secure, high-performance hashing
- **Safe Deletion**: Files are moved to system Trash/Recycle Bin, not permanently deleted
- **Protected Folders**: Designate folders as protected to prevent accidental deletion
- **Incremental Scanning**: Quick scan mode uses cached hashes for faster subsequent scans
- **macOS First**: Native macOS experience, with Windows support planned

## Tech Stack

- **Framework**: [Tauri 2.x](https://v2.tauri.app/) (Rust backend + web frontend)
- **Frontend**: [Svelte 5](https://svelte.dev/) + TypeScript
- **Database**: SQLite
- **Hash Algorithm**: BLAKE3

## Prerequisites

- macOS 10.15 or later
- Xcode Command Line Tools
- Node.js 20+
- Rust toolchain

Run the setup script to install prerequisites:

```bash
chmod +x scripts/setup-mac.sh
./scripts/setup-mac.sh
```

> **Windows**: Not yet supported. Windows setup instructions and a setup script (`scripts/setup-windows.ps1`) exist for future use. See [ROADMAP.md](ROADMAP.md) for timeline.

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

| Command                 | Description                           |
| ----------------------- | ------------------------------------- |
| `npm run dev`           | Start Vite dev server (frontend only) |
| `npm run tauri:dev`     | Start Tauri dev mode with hot reload  |
| `npm run build`         | Build frontend for production         |
| `npm run tauri:build`   | Build complete application            |
| `npm run check`         | Run Svelte type checking              |
| `npm run lint`          | Run ESLint                            |
| `npm run lint:fix`      | Run ESLint with auto-fix              |
| `npm run format`        | Format code with Prettier             |
| `npm run test`          | Run frontend tests                    |
| `npm run test:watch`    | Run tests in watch mode               |
| `npm run test:coverage` | Run tests with coverage report        |
| `cargo test`            | Run Rust tests (from src-tauri/)      |
| `cargo clippy`          | Run Rust linter (from src-tauri/)     |

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
└── docs/                  # Documentation
    ├── Specification.md   # Product specification
    └── plans/             # Implementation plans
```

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full feature roadmap. Current status:

- **v0.1 Core** — Complete (scanning, detection, results UI, deletion)
- **v0.2 Polish** — Next (progress UI, settings, file operations, filtering, keyboard nav)
- **v0.3 Production Ready** — Planned (permissions, error handling, platform polish, system tray)

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for development setup and conventions.

## License

MIT
