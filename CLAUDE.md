# DupliFind - Claude Code Project Instructions

## Guiding Principles

- Always optimize for correctness over speed.

## Development Workflow

These rules govern how all work on this project is done. They override default behavior.

1. **Skills first.** Use the superpowers skills. Before any creative or implementation work, check for an applicable skill (`using-superpowers`), and use `test-driven-development` for all coding.
2. **TDD is mandatory.** Follow red-green-refactor at the core of every feature, bug fix, refactor, and behavior change: write one minimal failing test, watch it fail for the right reason, write the minimal code to make it pass, then refactor while staying green. **No production code without a failing test first.** The only exceptions are throwaway prototypes, generated code, and configuration/docs, and only after asking.
3. **BDD for end-to-end behavior.** User-facing and UI behavior is covered with BDD-style Given/When/Then scenarios, including UI tests where a user-facing flow exists. This project favors the **BDD route over raw E2E**. The framework is intentionally unspecified until the BDD harness is established; standing up that harness is the next priority after issue #39.
4. **Surface design changes.** It is expected and encouraged to surface design or plan changes discovered during the TDD/BDD flow rather than forcing the original plan.
5. **Never assume, ask.** Always surface open questions to the user instead of proceeding on silent assumptions.

## Automated Verification

When asked to run automated verification (or when plan steps call for it), run **all** of the following checks. All must pass before considering verification complete.

### Frontend

```bash
npm test              # Vitest - all frontend unit tests
npm run check         # svelte-check - TypeScript/Svelte type checking
npm run lint          # ESLint - linting (includes type-checked rules)
npm run build         # Vite production build
```

### Backend (from src-tauri/)

```bash
cargo test            # Rust unit tests (all must pass)
cargo clippy          # Rust linter - must produce zero warnings
```

### Formatting

```bash
npx prettier --check .   # Prettier - verify formatting (do not auto-fix without asking)
```

If any check fails, fix the issues and re-run until all checks pass.

## GitHub API

> **Claude Cloud Environment (Claude.ai/Code) only:** Do **not** use the `gh` CLI — it is unavailable in this environment. Instead, use `curl` with the `$GH_TOKEN` environment variable to call the GitHub REST API directly for all GitHub operations (creating PRs, commenting on issues, etc.).

```bash
curl -s -X POST "https://api.github.com/repos/OWNER/REPO/pulls" \
  -H "Authorization: token $GH_TOKEN" \
  -H "Accept: application/vnd.github+json" \
  -d '{ ... }'
```

## Committing

Always use `/cl:commit` to create commits. Before invoking the command, run `npx prettier --write .` to auto-format so that any Prettier changes are visible in the diff and reflected in the commit message. `/cl:commit` can create multiple commits in a single invocation — group related changes into meaningful commits.

## Code Reviews

All code review reports go in `docs/reviews/`. Use the naming convention `CODE-REVIEW-PR<number>.md` for PR-specific reviews. The main repository-wide review is `docs/reviews/CODE-REVIEW.md`.

## Dead Code Annotations

The following items have `#[allow(dead_code)]` or `#[allow(unused_imports)]` because they are not yet used in production code but are needed in future plan phases. When a phase starts **using** one of these items, remove its suppression annotation:

- `scanner/mod.rs` — `#![allow(unused_imports)]` for the `HashError` re-export (needed once hash errors are surfaced to callers)
- `scanner/detector.rs` — `#[allow(dead_code)]` on `DuplicateGroup::original()` (needed for "Original" badge in Results UI)
- `scanner/hasher.rs` — `#[allow(dead_code)]` on `HashResult`, `full_hash_parallel()`, `compute_hashes()`, `hash_data()`
