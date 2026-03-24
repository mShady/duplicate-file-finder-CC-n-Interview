# Contributing to DupliFind

## Prerequisites

> Development currently targets **macOS**. Windows support is planned for the platform polish stage.

- macOS 10.15 or later
- Xcode Command Line Tools
- Node.js 20+
- Rust toolchain (stable)

```bash
chmod +x scripts/setup-mac.sh
./scripts/setup-mac.sh
```

## Development Setup

```bash
npm install
npm run tauri:dev    # Start Tauri dev mode with hot reload
```

## Verification Checks

All checks must pass before committing:

### Frontend

```bash
npm test              # Vitest unit tests
npm run check         # svelte-check (TypeScript/Svelte types)
npm run lint          # ESLint (includes type-checked rules)
npm run build         # Vite production build
```

### Backend (from `src-tauri/`)

```bash
cargo test            # Rust unit tests
cargo clippy          # Zero warnings required
```

### Formatting

```bash
npx prettier --check .   # Verify formatting
npx prettier --write .   # Auto-fix formatting
```

## Project Structure

```
src/                    # Frontend (Svelte 5 + TypeScript)
├── lib/
│   ├── api/            # Typed Tauri command wrappers
│   ├── components/     # Svelte components
│   ├── stores/         # Svelte 5 rune-based stores
│   └── utils/          # Frontend utilities
src-tauri/              # Backend (Rust)
├── src/
│   ├── commands/       # Tauri command handlers
│   ├── db/             # SQLite data access layer
│   ├── scanner/        # File scanning & hashing
│   └── services/       # Business logic layer
tests/                  # Frontend tests (Vitest)
docs/                   # Documentation & plans
```

## Conventions

- **Vertical slices**: Each feature includes backend + frontend + tests
- **Commit size**: Target ~5 files per commit
- **Rust linting**: Clippy with `clippy::all` + `clippy::pedantic` — zero warnings
- **Frontend**: Svelte 5 runes for reactivity (not Svelte 4 stores)
- **API layer**: All Tauri `invoke()` calls go through typed wrappers in `src/lib/api/`
- **Database**: All SQL goes through the DAL in `src-tauri/src/db/queries.rs`
