# Plan: Add Claude Code Pre-Commit Hooks

## Status: IMPLEMENTED

## Goal

Automate verification enforcement so Claude cannot commit without passing all checks defined in CLAUDE.md.

## What to do

Add a `PreToolUse` hook on the `Bash` tool in `.claude/settings.local.json`, pointing to a shell script that filters for `git commit` commands and runs all CLAUDE.md verification checks. The matcher only matches tool names (not arguments), so the script handles command filtering internally.

### Step 1: Create `.claude/hooks/pre-commit-verify.sh`

A bash script that:

1. Receives the tool input as JSON on standard input (Claude Code passes hook data via stdin, not command-line arguments)
2. Extracts the command using `python3` to parse `.tool_input.command` from the JSON
3. Exits 0 (allow) immediately if the command does not contain `git commit`
4. Runs all 7 verification checks sequentially, exiting 2 (block) on any failure

### Step 2: Update `.claude/settings.local.json`

Merge hooks into the existing file (which already contains `permissions`). **Important:** Use an absolute path to the script — `$CLAUDE_PROJECT_DIR` is not available in the hook execution environment, and relative paths may not resolve correctly.

```json
{
  "permissions": {
    "allow": [ ...existing entries... ]
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Bash",
        "hooks": [
          {
            "type": "command",
            "command": "/absolute/path/to/project/.claude/hooks/pre-commit-verify.sh",
            "timeout": 300
          }
        ]
      }
    ]
  }
}
```

### Step 3: Verify hooks work

1. Open `/hooks` in Claude Code to reload the configuration (or restart the session).
2. Make a trivial change and attempt a commit via `/cl:commit`.
3. Confirm all 7 checks run and the commit only proceeds if all pass.
4. Introduce a deliberate failure (e.g., lint error) and confirm the commit is blocked.

## Hook design

- **Matcher:** `"Bash"` — catches all Bash tool calls. The script filters internally because matchers only match tool names, not arguments.
- **Exit codes:** `0` = allow the tool call, `2` = block (standard error output shown as reason).
- **Output:** Progress labels (`[1/7]`, `[2/7]`, ...) written to standard error for visibility.

## Check order rationale

1. **Prettier write** — auto-format first so subsequent checks see clean code
2. **Vitest** — fast frontend tests
3. **svelte-check** — TypeScript/Svelte type checking
4. **ESLint** — linting with type-checked rules
5. **Vite build** — production build verification
6. **Cargo test** — Rust unit tests
7. **Cargo clippy** — Rust linter (last because it's thorough and slow)

Frontend checks run before backend checks since they're generally faster, giving quicker feedback on failures.

## Lessons learned

- **Hook input comes via standard input**, not command-line arguments (`$1`). The script must read JSON from standard input.
- **`$CLAUDE_PROJECT_DIR` is not set** in the hook execution environment. Use absolute paths in the hook command.
- **Relative paths don't resolve** because the hook may execute from a different working directory than the project root.
- **`/hooks` reloads configuration** without restarting the session. Use it after modifying `settings.local.json`.
- **Silent success is invisible** — hooks only show output in the UI when they error or are slow. Use a sentinel file (`echo ... >> /tmp/sentinel.txt`) to verify a hook is actually firing.
- **Match "contains" not "starts with"** — the `/cl:commit` skill chains `git add ... && git commit ...` in a single Bash call. A `git\ commit*` prefix check misses these. Use `*"git commit"*` (contains) instead.

## Notes

- `cargo test --quiet` suppresses passing test output to keep hook feedback concise.
- `npm test -- --run` ensures Vitest runs once and exits (no watch mode).
- All 7 checks match the full verification list in CLAUDE.md — nothing is omitted.
- Lives in `settings.local.json` so it doesn't affect other contributors' Claude Code setups.
- The script and settings are local-only (not committed to git).
- The hook fires on every Bash call but short-circuits instantly for non-commit commands (only the `python3` JSON parse runs, adding approximately 100 milliseconds of overhead).
