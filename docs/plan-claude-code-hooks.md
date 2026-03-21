# Plan: Add Claude Code Pre-Commit Hooks

## Goal

Automate verification enforcement so Claude cannot commit without passing all checks defined in CLAUDE.md.

## What to do

Add hooks to `.claude/settings.local.json` (local only, not committed) using Claude Code's `PreToolUse` event to intercept `git commit` calls. This ensures all verification passes before any commit executes.

### Step 1: Update `.claude/settings.local.json`

Merge hooks into the existing file (which already contains `permissions`). The hooks run sequentially — if any fails, the commit is blocked.

```json
{
  "permissions": {
    "allow": [ ...existing entries... ]
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash(git commit*)",
        "hooks": [
          {
            "type": "command",
            "command": "npx prettier --write ."
          },
          {
            "type": "command",
            "command": "npm test -- --run"
          },
          {
            "type": "command",
            "command": "npm run check"
          },
          {
            "type": "command",
            "command": "npm run lint"
          },
          {
            "type": "command",
            "command": "npm run build"
          },
          {
            "type": "command",
            "command": "cd src-tauri && cargo test --quiet"
          },
          {
            "type": "command",
            "command": "cd src-tauri && cargo clippy --all-targets -- -D warnings"
          }
        ]
      }
    ]
  }
}
```

### Step 2: Verify hooks work

1. Make a trivial change (e.g., add a comment).
2. Attempt a commit via `/cl:commit`.
3. Confirm all 7 hooks run and the commit only proceeds if all pass.
4. Introduce a deliberate failure (e.g., lint error) and confirm the commit is blocked.

## Hook order rationale

1. **Prettier write** — auto-format first so subsequent checks see clean code, and any formatting changes are included in the commit diff
2. **Vitest** — fast frontend tests
3. **svelte-check** — TypeScript/Svelte type checking
4. **ESLint** — linting with type-checked rules
5. **Vite build** — production build verification
6. **Cargo test** — Rust unit tests
7. **Cargo clippy** — Rust linter (last because it's thorough and slow)

Frontend checks run before backend checks since they're generally faster, giving quicker feedback on failures.

## Notes

- `cargo test --quiet` suppresses passing test output to keep hook feedback concise.
- `npm test -- --run` ensures Vitest runs once and exits (no watch mode).
- All 7 checks match the full verification list in CLAUDE.md — nothing is omitted.
- Lives in `settings.local.json` so it doesn't affect other contributors' Claude Code setups.
