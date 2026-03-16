# Plan: Add Claude Code Pre-Commit Hooks

## Goal

Automate verification enforcement so Claude cannot commit without passing all checks already defined in CLAUDE.md.

## What to do

Create `.claude/settings.json` with pre-commit hooks matching the verification commands in CLAUDE.md:

```json
{
  "hooks": {
    "PreCommit": [
      {
        "command": "cd src-tauri && cargo clippy --all-targets -- -D warnings",
        "description": "Rust linter"
      },
      {
        "command": "cd src-tauri && cargo test --quiet",
        "description": "Rust tests"
      },
      {
        "command": "npm test -- --run",
        "description": "Frontend unit tests"
      },
      {
        "command": "npm run lint",
        "description": "ESLint"
      },
      {
        "command": "npx prettier --check .",
        "description": "Prettier format check"
      }
    ]
  }
}
```

## Notes

- `npm run check` (svelte-check) and `npm run build` are omitted from hooks — they're slower and better suited for manual verification or CI. Add them if commit speed isn't a concern.
- `cargo test --quiet` suppresses passing test output to keep hook feedback concise.
- `npm test -- --run` ensures vitest runs once and exits (no watch mode).
