# Enabling Claude on a GitHub Repository

A comprehensive guide to setting up [Claude Code Action](https://github.com/anthropics/claude-code-action) on your GitHub repository. This guide uses the [DupliFind](https://github.com/mShady/duplicate-file-finder-CC-n-Interview) project's workflows as a real-world reference, explaining every configuration choice — what's essential, what's customized for this specific repo, and how to adapt it for your own projects.

---

## Table of Contents

1. [What You Get](#1-what-you-get)
2. [Prerequisites](#2-prerequisites)
3. [Quick Start](#3-quick-start)
4. [The Two Workflow Files](#4-the-two-workflow-files)
   - [4.1 `claude.yml` — Interactive @claude Mentions](#41-claudeyml--interactive-claude-mentions)
   - [4.2 `claude-code-review.yml` — Automatic PR Code Review](#42-claude-code-reviewyml--automatic-pr-code-review)
5. [Line-by-Line Workflow Breakdown](#5-line-by-line-workflow-breakdown)
   - [5.1 `claude.yml` Explained](#51-claudeyml-explained)
   - [5.2 `claude-code-review.yml` Explained](#52-claude-code-reviewyml-explained)
6. [Essential vs. Repo-Specific Configuration](#6-essential-vs-repo-specific-configuration)
7. [Customization Guide](#7-customization-guide)
8. [How This Configuration Evolved (Lessons Learned)](#8-how-this-configuration-evolved-lessons-learned)
9. [Troubleshooting](#9-troubleshooting)
10. [Best Practices](#10-best-practices)
11. [Further Reading](#11-further-reading)

---

## 1. What You Get

After following this guide, your repository will have two Claude-powered capabilities:

| Capability                | Trigger                                               | What Claude Does                                                                                      |
| ------------------------- | ----------------------------------------------------- | ----------------------------------------------------------------------------------------------------- |
| **Interactive assistant** | Type `@claude` in any issue, PR comment, or PR review | Reads the codebase, runs tests, creates branches/PRs, posts inline comments                           |
| **Automatic code review** | Open or update a PR                                   | Reviews the diff for bugs, security issues, and code quality; posts inline comments on specific lines |

---

## 2. Prerequisites

### Required

- **Repository admin access** — you'll need to install a GitHub App and add secrets.
- **One authentication credential** (choose one):

  | Method          | Secret Name               | How to Get It                                                                                 |
  | --------------- | ------------------------- | --------------------------------------------------------------------------------------------- |
  | **API Key**     | `ANTHROPIC_API_KEY`       | From [console.anthropic.com](https://console.anthropic.com) (starts with `sk-ant-`)           |
  | **OAuth Token** | `CLAUDE_CODE_OAUTH_TOKEN` | Run `claude setup-token` in [Claude Code CLI](https://claude.com/claude-code) (Pro/Max plans) |

### Recommended

- A `CLAUDE.md` file at the repo root — this is Claude's project instruction file. It tells Claude about your project's conventions, build commands, linting rules, and architecture. Claude reads it automatically on every invocation.

---

## 3. Quick Start

### Option A: Automatic Setup (Fastest)

Open the Claude Code CLI and run:

```
/install-github-app
```

This walks you through GitHub App installation, secret configuration, and workflow file creation interactively.

### Option B: Manual Setup

**Step 1: Install the Claude GitHub App**

Go to [github.com/apps/claude](https://github.com/apps/claude) and install it on your repository (or organization).

**Step 2: Add an authentication secret**

Go to your repo's **Settings > Secrets and variables > Actions > New repository secret** and add one of:

- `ANTHROPIC_API_KEY` — your Anthropic API key, or
- `CLAUDE_CODE_OAUTH_TOKEN` — your OAuth token from `claude setup-token`

**Step 3: Create the workflow files**

Create `.github/workflows/claude.yml` and (optionally) `.github/workflows/claude-code-review.yml` using the templates in sections [4.1](#41-claudeyml--interactive-claude-mentions) and [4.2](#42-claude-code-reviewyml--automatic-pr-code-review) below.

**Step 4: Test it**

Open a new issue with `@claude` in the body (e.g., `@claude summarize this repo`). You should see a workflow run start within seconds.

---

## 4. The Two Workflow Files

### 4.1 `claude.yml` — Interactive @claude Mentions

This is the workflow that responds when someone types `@claude` in an issue, PR comment, or PR review. Claude acts as an interactive assistant — it can read your codebase, run build/test commands, create branches, open PRs, and post inline code comments.

Here is DupliFind's production workflow:

```yaml
name: Claude Code

on:
  issue_comment:
    types: [created]
  pull_request_review_comment:
    types: [created]
  issues:
    types: [opened, assigned]
  pull_request_review:
    types: [submitted]

jobs:
  claude:
    if: |
      (github.event_name == 'issue_comment' && contains(github.event.comment.body, '@claude')) ||
      (github.event_name == 'pull_request_review_comment' && contains(github.event.comment.body, '@claude')) ||
      (github.event_name == 'pull_request_review' && contains(github.event.review.body, '@claude')) ||
      (github.event_name == 'issues' && (contains(github.event.issue.body, '@claude') || contains(github.event.issue.title, '@claude')))
    runs-on: ubuntu-latest
    permissions:
      contents: write
      pull-requests: write
      issues: write
      id-token: write
      actions: read
    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 1

      - name: Install system dependencies for Tauri
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: npm

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy

      - name: Cache Cargo dependencies
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            src-tauri/target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Install frontend dependencies
        run: npm ci

      - name: Run Claude Code
        id: claude
        uses: anthropics/claude-code-action@v1
        with:
          claude_code_oauth_token: ${{ secrets.CLAUDE_CODE_OAUTH_TOKEN }}
          allowed_bots: ''
          additional_permissions: |
            actions: read
          claude_args: |
            --model claude-opus-4-6
            --allowedTools "mcp__github_inline_comment__create_inline_comment,Bash(npm:*),Bash(npx:*),Bash(cargo:*),Bash(gh:*),Bash(git:*)"
```

### 4.2 `claude-code-review.yml` — Automatic PR Code Review

This workflow triggers automatically on every PR. Claude reviews the diff and posts inline comments on specific code lines.

```yaml
name: Claude Code Review

on:
  pull_request:
    types: [opened, synchronize, ready_for_review, reopened]

jobs:
  claude-review:
    runs-on: ubuntu-latest
    permissions:
      contents: read
      pull-requests: write
      issues: read
      id-token: write

    steps:
      - name: Checkout repository
        uses: actions/checkout@v4
        with:
          fetch-depth: 1

      - name: Run Claude Code Review
        id: claude-review
        uses: anthropics/claude-code-action@v1
        with:
          claude_code_oauth_token: ${{ secrets.CLAUDE_CODE_OAUTH_TOKEN }}
          allowed_bots: 'claude[bot]'
          track_progress: true
          use_sticky_comment: true
          plugin_marketplaces: 'https://github.com/anthropics/claude-code.git'
          plugins: 'code-review@claude-code-plugins'
          prompt: '/code-review:code-review ${{ github.repository }}/pull/${{ github.event.pull_request.number }}'
          claude_args: |
            --model claude-opus-4-6
            --allowedTools "mcp__github_inline_comment__create_inline_comment,Bash(gh pr diff:*),Bash(gh pr view:*)"
```

---

## 5. Line-by-Line Workflow Breakdown

### 5.1 `claude.yml` Explained

#### Trigger Events

```yaml
on:
  issue_comment:
    types: [created]
  pull_request_review_comment:
    types: [created]
  issues:
    types: [opened, assigned]
  pull_request_review:
    types: [submitted]
```

This listens for four types of GitHub events:

| Event                         | When it fires                                           |
| ----------------------------- | ------------------------------------------------------- |
| `issue_comment`               | A new comment is posted on an issue or PR               |
| `pull_request_review_comment` | A new comment is posted on a specific line in a PR diff |
| `issues`                      | An issue is opened or assigned                          |
| `pull_request_review`         | A PR review is submitted                                |

All four are needed so `@claude` works everywhere — issues, PR conversations, inline diff comments, and review summaries.

#### The `if` Condition (Trigger Filter)

```yaml
if: |
  (github.event_name == 'issue_comment' && contains(github.event.comment.body, '@claude')) ||
  (github.event_name == 'pull_request_review_comment' && contains(github.event.comment.body, '@claude')) ||
  (github.event_name == 'pull_request_review' && contains(github.event.review.body, '@claude')) ||
  (github.event_name == 'issues' && (contains(github.event.issue.body, '@claude') || contains(github.event.issue.title, '@claude')))
```

Even though the workflow triggers on _every_ comment/issue, this condition ensures the job only runs when `@claude` is actually mentioned. Without this filter, Claude would run (and bill you) on every single comment in the repo.

> **Customization:** You can change `@claude` to any trigger phrase. If you do, also set the `trigger_phrase` input on the action itself.

#### Permissions

```yaml
permissions:
  contents: write
  pull-requests: write
  issues: write
  id-token: write
  actions: read
```

| Permission             | Why it's needed                                                                        |
| ---------------------- | -------------------------------------------------------------------------------------- |
| `contents: write`      | Claude needs to create branches, commit code, and push changes                         |
| `pull-requests: write` | Claude needs to create PRs and post comments on them                                   |
| `issues: write`        | Claude needs to post comments on issues                                                |
| `id-token: write`      | Required for the action's internal token system (and for OIDC if using Bedrock/Vertex) |
| `actions: read`        | Lets Claude read CI workflow results to understand if tests passed or failed           |

> **Important:** If you only give `contents: read`, Claude can analyze code but **cannot create branches or PRs**. This was [the first major issue](#8-how-this-configuration-evolved-lessons-learned) discovered in DupliFind's setup.

#### Build Toolchain Steps (Repo-Specific)

```yaml
- name: Install system dependencies for Tauri
  run: |
    sudo apt-get update
    sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

- name: Setup Node.js
  uses: actions/setup-node@v4
  with:
    node-version: 20
    cache: npm

- name: Install Rust stable
  uses: dtolnay/rust-toolchain@stable
  with:
    components: clippy

- name: Cache Cargo dependencies
  uses: actions/cache@v4
  with:
    path: |
      ~/.cargo/bin/
      ~/.cargo/registry/index/
      ~/.cargo/registry/cache/
      ~/.cargo/git/db/
      src-tauri/target/
    key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

- name: Install frontend dependencies
  run: npm ci
```

**These steps are entirely specific to DupliFind** — a Tauri app with a Svelte/TypeScript frontend and Rust backend. They install the system libraries, language toolchains, and project dependencies that Claude needs to run `npm test`, `cargo test`, `cargo clippy`, etc.

**For your project, replace these with your own build setup.** Examples:

| Project type       | What you'd put here                                        |
| ------------------ | ---------------------------------------------------------- |
| Node.js/TypeScript | `actions/setup-node` + `npm ci`                            |
| Python             | `actions/setup-python` + `pip install -r requirements.txt` |
| Go                 | `actions/setup-go` (Go usually doesn't need extra deps)    |
| Simple HTML/CSS/JS | Nothing — you can skip straight to the action step         |

> **Why bother?** Without a build toolchain, Claude cannot run your tests, linter, or build. It can still read/write code, but it can't verify its changes pass CI.

#### The Action Step

```yaml
- name: Run Claude Code
  id: claude
  uses: anthropics/claude-code-action@v1
  with:
    claude_code_oauth_token: ${{ secrets.CLAUDE_CODE_OAUTH_TOKEN }}
    allowed_bots: ''
    additional_permissions: |
      actions: read
    claude_args: |
      --model claude-opus-4-6
      --allowedTools "mcp__github_inline_comment__create_inline_comment,Bash(npm:*),Bash(npx:*),Bash(cargo:*),Bash(gh:*),Bash(git:*)"
```

| Input                                   | Purpose                                                                                                                                                                                                                     |
| --------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `claude_code_oauth_token`               | Authentication. Use this OR `anthropic_api_key` — not both.                                                                                                                                                                 |
| `allowed_bots: ''`                      | **No bots can trigger this workflow.** This prevents infinite loops — if Claude posts a comment that contains `@claude`, it won't trigger itself. See [Lessons Learned](#8-how-this-configuration-evolved-lessons-learned). |
| `additional_permissions: actions: read` | Grants the action's internal token permission to read CI results. This must match the job-level `actions: read` permission.                                                                                                 |
| `--model claude-opus-4-6`               | Explicitly selects Claude Opus 4.6. Without this, the action defaults to Sonnet.                                                                                                                                            |
| `--allowedTools "..."`                  | Restricts which tools Claude can use. **Bash commands are disabled by default for security** — you must explicitly allow them.                                                                                              |

**Allowed tools breakdown:**

| Tool                                                | What it allows                                                   |
| --------------------------------------------------- | ---------------------------------------------------------------- |
| `mcp__github_inline_comment__create_inline_comment` | Post comments on specific code lines in PR diffs                 |
| `Bash(npm:*)`                                       | Run any `npm` command (e.g., `npm test`, `npm run build`)        |
| `Bash(npx:*)`                                       | Run any `npx` command (e.g., `npx prettier --check .`)           |
| `Bash(cargo:*)`                                     | Run any `cargo` command (e.g., `cargo test`, `cargo clippy`)     |
| `Bash(gh:*)`                                        | Run any `gh` CLI command (e.g., `gh pr create`, `gh issue list`) |
| `Bash(git:*)`                                       | Run any `git` command (e.g., `git checkout`, `git push`)         |

> **For your project**, replace `Bash(npm:*)`, `Bash(npx:*)`, `Bash(cargo:*)` with whatever CLI tools your project uses. A Python project might use `Bash(python:*)`, `Bash(pip:*)`, `Bash(pytest:*)`.

---

### 5.2 `claude-code-review.yml` Explained

#### Trigger Events

```yaml
on:
  pull_request:
    types: [opened, synchronize, ready_for_review, reopened]
```

This runs Claude's code review on every PR event:

| Event type         | When it fires                            |
| ------------------ | ---------------------------------------- |
| `opened`           | A new PR is created                      |
| `synchronize`      | New commits are pushed to an existing PR |
| `ready_for_review` | A draft PR is marked ready for review    |
| `reopened`         | A closed PR is reopened                  |

> **Customization:** You can filter by file path to only review changes to specific parts of the codebase:
>
> ```yaml
> on:
>   pull_request:
>     types: [opened, synchronize, ready_for_review, reopened]
>     paths:
>       - 'src/**'
>       - '*.config.*'
> ```

> **Customization:** You can filter by PR author to only review PRs from specific users:
>
> ```yaml
> if: |
>   github.event.pull_request.author_association == 'FIRST_TIME_CONTRIBUTOR'
> ```

#### Permissions

```yaml
permissions:
  contents: read
  pull-requests: write
  issues: read
  id-token: write
```

Notice the differences from `claude.yml`:

| Permission      | `claude.yml` | `claude-code-review.yml` | Why                                                         |
| --------------- | ------------ | ------------------------ | ----------------------------------------------------------- |
| `contents`      | **write**    | **read**                 | Reviews only read code — they don't create branches or push |
| `pull-requests` | write        | write                    | Both need to post comments                                  |
| `issues`        | write        | **read**                 | Reviews don't modify issues                                 |
| `actions`       | read         | _(not needed)_           | Reviews don't check CI results                              |

> **Principle of least privilege:** The review workflow has narrower permissions because it only needs to read code and post comments.

#### The Action Step

```yaml
- name: Run Claude Code Review
  id: claude-review
  uses: anthropics/claude-code-action@v1
  with:
    claude_code_oauth_token: ${{ secrets.CLAUDE_CODE_OAUTH_TOKEN }}
    allowed_bots: 'claude[bot]'
    track_progress: true
    use_sticky_comment: true
    plugin_marketplaces: 'https://github.com/anthropics/claude-code.git'
    plugins: 'code-review@claude-code-plugins'
    prompt: '/code-review:code-review ${{ github.repository }}/pull/${{ github.event.pull_request.number }}'
    claude_args: |
      --model claude-opus-4-6
      --allowedTools "mcp__github_inline_comment__create_inline_comment,Bash(gh pr diff:*),Bash(gh pr view:*)"
```

| Input                                        | Purpose                                                                                                                                                                                                                             |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `allowed_bots: 'claude[bot]'`                | **Allows Claude's own PRs to be reviewed.** Safe here because the review workflow can only post comments — it can't create new PRs or comments that would re-trigger itself.                                                        |
| `track_progress: true`                       | Posts a tracking comment on the PR with visual progress checkboxes. Without this, Claude writes results only to the GitHub Actions Step Summary (invisible on the PR page).                                                         |
| `use_sticky_comment: true`                   | Updates the same comment on re-pushes instead of creating a new comment each time. Keeps the PR conversation clean. Only works with `claude[bot]` authentication.                                                                   |
| `plugin_marketplaces`                        | Points to the official Claude Code plugin registry.                                                                                                                                                                                 |
| `plugins: 'code-review@claude-code-plugins'` | Installs the `code-review` plugin, which uses a multi-agent architecture: 4 parallel agents check CLAUDE.md compliance, scan for bugs, and analyze git blame history. Issues are scored 0-100; only those scoring >= 80 are posted. |
| `prompt`                                     | Invokes the code-review plugin's skill with the current PR number. The `${{ github.repository }}` and `${{ github.event.pull_request.number }}` variables are filled in by GitHub Actions.                                          |
| `--allowedTools "..."`                       | Narrower than `claude.yml` — only allows inline comments and reading PR diffs/metadata. No `npm`, `cargo`, `git push`, etc.                                                                                                         |

> **Note on `--allowedTools`:** The review workflow intentionally restricts Claude to read-only operations (`gh pr diff`, `gh pr view`) plus posting comments. This is the principle of least privilege — a code reviewer doesn't need to run your tests or push code.

---

## 6. Essential vs. Repo-Specific Configuration

### Essential for Any Repository (Keep These)

These parts are required for Claude to function and should be kept in any project:

**In `claude.yml`:**

| Line(s)                                          | What                     | Why Essential                                                |
| ------------------------------------------------ | ------------------------ | ------------------------------------------------------------ |
| `on:` triggers                                   | 4 event types            | Covers all places where `@claude` can be mentioned           |
| `if:` condition                                  | `@claude` filter         | Prevents unnecessary runs on every comment                   |
| `permissions:` block                             | 5 permissions            | Minimum for Claude to read code + create PRs + post comments |
| `actions/checkout@v4`                            | Checkout step            | Claude needs the source code                                 |
| `anthropics/claude-code-action@v1`               | The action itself        | Core integration                                             |
| `claude_code_oauth_token` or `anthropic_api_key` | Authentication           | Required — one or the other                                  |
| `allowed_bots: ''`                               | Infinite loop prevention | Critical safety measure                                      |

**In `claude-code-review.yml`:**

| Line(s)                            | What               | Why Essential                                                                                         |
| ---------------------------------- | ------------------ | ----------------------------------------------------------------------------------------------------- |
| `on: pull_request`                 | PR trigger         | Fires on PR activity                                                                                  |
| `permissions:` block               | 4 permissions      | Minimum for reading code + posting review comments                                                    |
| `actions/checkout@v4`              | Checkout step      | Claude needs the source code                                                                          |
| `anthropics/claude-code-action@v1` | The action itself  | Core integration                                                                                      |
| Authentication secret              | One of the two     | Required                                                                                              |
| `track_progress: true`             | Comment visibility | Without this, reviews only appear in the Actions Summary tab, not on the PR itself                    |
| `prompt:`                          | Review instruction | Tells Claude what to do — without this, it runs in interactive mode and waits for a `@claude` mention |

### Customized for DupliFind (Adapt for Your Project)

| Configuration                                                                | DupliFind-specific value | How to adapt                                                                                                            |
| ---------------------------------------------------------------------------- | ------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| **Build toolchain steps** (Tauri deps, Node.js, Rust, Cargo cache, `npm ci`) | Tauri/Svelte/Rust stack  | Replace with your project's build steps (Python, Go, Java, etc.) or remove entirely if Claude doesn't need to run tests |
| `--model claude-opus-4-6`                                                    | Opus 4.6                 | Remove to use the default (Sonnet) or change to another model ID                                                        |
| `Bash(npm:*),Bash(npx:*),Bash(cargo:*)`                                      | Node + Rust tools        | Replace with your project's CLI tools                                                                                   |
| `Bash(gh pr diff:*),Bash(gh pr view:*)`                                      | PR reading tools         | Usually keep as-is for code review                                                                                      |
| `plugins: 'code-review@claude-code-plugins'`                                 | Multi-agent code review  | Can be removed if you write your own review prompt instead                                                              |

---

## 7. Customization Guide

### Choosing a Model

```yaml
claude_args: |
  --model claude-opus-4-6
```

Available models (via direct Anthropic API):

- `claude-opus-4-6` — Most capable, best for complex tasks
- `claude-sonnet-4-6` — Default; faster and cheaper, good for most tasks
- `claude-haiku-4-5` — Fastest and cheapest, good for simple/triage tasks

For cloud providers:

```yaml
# AWS Bedrock
use_bedrock: "true"
claude_args: --model anthropic.claude-4-0-sonnet-20250805-v1:0

# Google Vertex AI
use_vertex: "true"
claude_args: --model claude-4-0-sonnet@20250805
```

### Restricting Tool Access

The `--allowedTools` flag controls what Claude can do. Be as restrictive as possible.

**Common presets:**

| Use case           | `--allowedTools` value                                                                      |
| ------------------ | ------------------------------------------------------------------------------------------- |
| Code review only   | `"mcp__github_inline_comment__create_inline_comment,Bash(gh pr diff:*),Bash(gh pr view:*)"` |
| Node.js project    | `"Bash(npm:*),Bash(npx:*),Bash(gh:*),Bash(git:*)"`                                          |
| Python project     | `"Bash(python:*),Bash(pip:*),Bash(pytest:*),Bash(gh:*),Bash(git:*)"`                        |
| Go project         | `"Bash(go:*),Bash(gh:*),Bash(git:*)"`                                                       |
| Read-only analysis | `"Bash(gh:*)"`                                                                              |

### Filtering Which PRs Get Reviewed

```yaml
# Only review changes to specific directories
on:
  pull_request:
    paths:
      - "src/**"
      - "lib/**"

# Only review PRs from first-time contributors
jobs:
  claude-review:
    if: github.event.pull_request.author_association == 'FIRST_TIME_CONTRIBUTOR'

# Only review PRs from specific users
jobs:
  claude-review:
    if: |
      github.event.pull_request.user.login == 'junior-dev' ||
      github.event.pull_request.user.login == 'external-contractor'
```

### Using a Custom Trigger Phrase

```yaml
# In the action step:
with:
  trigger_phrase: '/ask-claude'

# Update the `if` condition to match:
if: contains(github.event.comment.body, '/ask-claude')
```

### Adding CI Result Awareness

If you want Claude to check whether tests passed before suggesting changes:

```yaml
# Job-level:
permissions:
  actions: read

# Action-level:
with:
  additional_permissions: |
    actions: read
```

### Custom Review Prompt (Without Plugin)

Instead of using the `code-review` plugin, you can write your own review prompt:

```yaml
prompt: |
  Review PR #${{ github.event.pull_request.number }} in ${{ github.repository }}.
  Focus on:
  - Security vulnerabilities
  - Performance regressions
  - API contract changes
  Post inline comments on specific lines where issues are found.
```

---

## 8. How This Configuration Evolved (Lessons Learned)

DupliFind's workflow files went through **11 commits across 9 PRs** before reaching their current state. Here are the key lessons, in chronological order:

### Lesson 1: Claude Needs Write Permissions to Be Useful

**Problem:** The initial workflow had `contents: read` and `pull-requests: read`. Claude could analyze code but couldn't create branches, push commits, or open PRs.

**Fix ([PR #4](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/4)):** Changed to `write` permissions for `contents`, `pull-requests`, and `issues`.

**Takeaway:** If you want Claude to _fix_ things (not just comment), you need write permissions.

### Lesson 2: Claude Needs Your Build Toolchain

**Problem:** Claude was given Bash access but the CI runner had no Node.js, no Rust, and no project dependencies. Claude couldn't run `npm test` or `cargo clippy`.

**Fix ([PR #4](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/4)):** Added build toolchain setup steps (Node.js, Rust, system deps) and `npm ci`.

**Takeaway:** Install everything Claude might need to verify its changes _before_ the action step runs.

### Lesson 3: Cache Your Dependencies

**Problem:** Rust builds are slow. Every CI run was downloading and compiling all Cargo dependencies from scratch.

**Fix ([PR #4](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/4), related to [Issue #5](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/issues/5)):** Added `actions/cache@v4` for Cargo dependencies, keyed on `Cargo.lock`.

**Takeaway:** Cache aggressively. Claude's workflow may run many times per day.

### Lesson 4: Prevent Infinite Loops with `allowed_bots`

**Problem:** When Claude created a PR, its own PR comment triggered the workflow again. Claude commented on its own PR, which triggered the workflow again... infinite loop.

**Fix ([PR #7](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/7) created it, [PR #10](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/10) reverted it to `''`):** Set `allowed_bots: ''` in `claude.yml` to block all bot triggers.

**Takeaway:** In the interactive (`@claude`) workflow, set `allowed_bots: ''`. In the review workflow, `allowed_bots: 'claude[bot]'` is safe because review-only workflows can't trigger themselves.

### Lesson 5: Code Review Needs `pull-requests: write`

**Problem:** The review workflow ran successfully but Claude couldn't post comments because it only had `pull-requests: read`.

**Fix ([PR #14](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/14)):** Changed to `pull-requests: write`.

**Takeaway:** Posting comments (even review comments) requires write permission on pull-requests.

### Lesson 6: Use `track_progress` for Automated Workflows

**Problem:** After fixing permissions, Claude's review still wasn't visible on the PR page. It was only in the GitHub Actions Step Summary.

**Fix ([PR #15](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/15)):** Added `track_progress: true` and `use_sticky_comment: true`.

**Takeaway:** When using a `prompt` input (automation mode), the action defaults to writing results to the Step Summary only. Add `track_progress: true` to force it to post comments on the PR.

### Lesson 7: Inline Comments Need Explicit Tool Access

**Problem:** Claude was posting review feedback as top-level PR comments, not as inline comments on specific code lines.

**Fix ([PR #16](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/16), [PR #17](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/17)):** Added `mcp__github_inline_comment__create_inline_comment` to `--allowedTools` in both workflows.

**Takeaway:** The inline comment MCP tool is not enabled by default. You must explicitly include it in `--allowedTools`.

### Lesson 8: Default Model May Not Be What You Want

**Problem:** Both workflows defaulted to Sonnet because no model was specified.

**Fix ([PR #18](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/pull/18), related to [Issue #13](https://github.com/mShady/duplicate-file-finder-CC-n-Interview/issues/13)):** Added `--model claude-opus-4-6` to `claude_args`.

**Takeaway:** If you want a specific model, set it explicitly. Don't rely on defaults.

---

## 9. Troubleshooting

| Symptom                                        | Likely Cause                                                    | Fix                                                                                                                               |
| ---------------------------------------------- | --------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `@claude` mention does nothing                 | Workflow not running                                            | Check the Actions tab; verify the `if` condition matches your trigger phrase                                                      |
| Workflow runs but Claude says "no permissions" | Missing write permissions                                       | Add `contents: write`, `pull-requests: write`, `issues: write`                                                                    |
| Claude can't run tests                         | No build toolchain in CI                                        | Add setup steps for your language/framework before the action step                                                                |
| Code review runs but no comments on PR         | Missing `track_progress: true`                                  | Add `track_progress: true` to the review workflow                                                                                 |
| Review comments are top-level, not inline      | Missing inline comment tool                                     | Add `mcp__github_inline_comment__create_inline_comment` to `--allowedTools`                                                       |
| Claude triggers itself in a loop               | `allowed_bots` allows `claude[bot]` in the interactive workflow | Set `allowed_bots: ''` in `claude.yml`                                                                                            |
| `use_sticky_comment` not working               | Using a custom `github_token`                                   | Sticky comments only work with the default `claude[bot]` authentication                                                           |
| "Resource not accessible by integration" error | Permissions mismatch                                            | Ensure job-level `permissions` block matches what the action needs                                                                |
| Claude can't read CI results                   | Missing `actions: read`                                         | Add both job-level `actions: read` and `additional_permissions: actions: read`                                                    |
| Workflow can't push to `.github/workflows/`    | Expected limitation                                             | The GitHub App token cannot modify workflow files. Push workflow changes manually or grant the `workflows` permission to the app. |
| Claude uses wrong model                        | No `--model` in `claude_args`                                   | Add `--model <model-id>` as the first line in `claude_args`                                                                       |

---

## 10. Best Practices

1. **Start with the minimum.** Get the basic `claude.yml` working first, then add the review workflow. Don't try to configure everything at once.

2. **Always set `allowed_bots: ''` in the interactive workflow.** Infinite loops will burn through your API quota fast.

3. **Restrict tools to what's needed.** Use `--allowedTools` to grant only the specific CLI commands your project requires. Don't use `Bash(*)` (allows everything).

4. **Use `CLAUDE.md` for project context.** Claude reads this file automatically. Put your build commands, coding conventions, linting rules, and architectural guidelines here. This is far more effective than cramming instructions into the `prompt` input.

5. **Cache your dependencies.** Claude workflows may run many times per day. Uncached Rust, Python, or Node.js builds add up.

6. **Use `use_sticky_comment: true` for reviews.** Without it, every push to a PR creates a new review comment, flooding the conversation.

7. **Keep `show_full_output: false` (the default) on public repos.** When enabled, full tool outputs (which may contain secrets or sensitive file contents) are visible in the workflow run logs.

8. **Specify the model explicitly.** Default models can change. Pin to a specific model ID so your workflows behave consistently.

9. **Test workflow changes in a branch.** Workflow files are active as soon as they're on the default branch. Push changes to a feature branch first and test with a dummy `@claude` mention.

10. **Give Claude the same tools your developers use.** If your team runs `npm test` and `cargo clippy` locally, give Claude `Bash(npm:*)` and `Bash(cargo:*)`. If Claude can't verify its changes, it's working blind.

---

## 11. Further Reading

- [claude-code-action — Official GitHub repo](https://github.com/anthropics/claude-code-action)
- [Setup Guide](https://github.com/anthropics/claude-code-action/blob/main/docs/setup.md) — Installation and authentication
- [Usage Guide](https://github.com/anthropics/claude-code-action/blob/main/docs/usage.md) — All configuration inputs
- [Configuration Reference](https://github.com/anthropics/claude-code-action/blob/main/docs/configuration.md) — MCP servers, settings, environment variables
- [Security Guide](https://github.com/anthropics/claude-code-action/blob/main/docs/security.md) — Access control, commit signing
- [FAQ / Troubleshooting](https://github.com/anthropics/claude-code-action/blob/main/docs/faq.md) — Common issues and fixes
- [Solutions & Workflow Examples](https://github.com/anthropics/claude-code-action/blob/main/docs/solutions.md) — Ready-to-use automation patterns
- [Official Plugins](https://github.com/anthropics/claude-code/tree/main/plugins) — All available plugins including `code-review`
- [Migration Guide (v0.x to v1.0)](https://github.com/anthropics/claude-code-action/blob/main/docs/migration-guide.md)
