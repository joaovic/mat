# PRD: Absorb Branchlet into Mat

## Overview

Mat currently requires Node.js and the `branchlet` CLI to manage git worktrees, creating a two-tool dependency chain for a single workflow. This PRD defines the product requirements for absorbing branchlet's core worktree operations (create, delete, list) directly into mat, eliminating the external dependency and adding three new capabilities: no-worktree mode (branch-only workflow with stash guard), TMUX auto-detection with graceful fallback, and two-tier TOML configuration. Additionally, mat will transition from a "create window + delete worktree" tool to a full task lifecycle manager by introducing auto-merge on close.

The target user is a developer who creates feature branches/worktrees in tmux sessions, works on them, and wants a single command to both open and close a task's entire git+tmux lifecycle.

## Goals

- **Eliminate the branchlet dependency**: mat must work as a single Rust binary with only git and (optionally) tmux as external dependencies
- **Own the full task lifecycle**: from `mat feat login` (create) through `mat --close` (merge + cleanup) in one streamlined flow
- **Support both worktree and branch modes**: developers restricted by disk space, monorepo size, or CI environments can opt out of worktrees without losing mat's value
- **Configurable, not hardcoded**: branch deletion, merge strategy, default base branch, and tmux behavior should all be settable via TOML config
- **Achieve > 95% create-to-close success rate**: the close flow must handle the common case (clean merge) automatically and degrade gracefully for conflicts

## User Stories

### Primary Persona: Solo Developer with TMUX

**As a developer working in tmux**, I want to type `mat feat login` and have a new tmux window open in a worktree on branch `feat/login`, so I can start working immediately without managing branches and directories manually.

**As a developer finishing a task**, I want to type `mat --close` and have mat merge my feature branch into the base branch, delete the worktree, close the tmux window, and clean up the branch, so I can move on to the next task with a single command.

### Secondary Persona: Developer without TMUX

**As a developer not running tmux**, I want to type `mat feat login` and have mat create a worktree and open a new shell process in that directory, so I can still use mat's workflow without tmux.

### Tertiary Persona: Developer in Constrained Environments

**As a developer in a large monorepo**, I want to type `mat --no-worktree fix typo` and have mat create a branch with stash protection instead of a full worktree, so I can use mat even when disk space makes worktrees impractical.

### Configuration User

**As a team lead**, I want to commit `.mat.toml` to the repo with our team's default base branch and merge strategy, so that every team member gets consistent behavior without manual setup.

## Core Features

### F1: Native Worktree CRUD (Critical)

Replace all three `branchlet` CLI invocations with direct `git worktree` commands:

- **Create**: `git worktree add -b <branch> <path> <source>` — creates worktree with new branch from source. Path derived from naming convention: `{app}-{type}/{name}` (e.g., `dashboard-feat/login`).
- **List**: `git worktree list --porcelain` — parse output to find current worktree's branch, path, and source. Used primarily by close mode to identify the current worktree.
- **Delete**: `git worktree remove <path>` — delete the worktree on close, only after successful merge.

Naming convention (updated to prevent collisions): worktree name includes task_type as `{app}-{type}/{name}`, matching the branch name pattern `{type}/{name}`.

### F2: No-Worktree Mode (High)

`--no-worktree` flag creates a branch instead of a worktree:

- Checks for uncommitted changes. If found, creates a named stash with `git stash push -m "mat:auto:{branch}"` before switching branches.
- Creates the branch with `git checkout -b {branch} {source}`.
- After switching, prints a clear disclaimer: "No-worktree mode: changes are isolated to this branch, not a separate directory. Uncommitted changes were stashed with name 'mat:auto:{branch}'."
- On close in no-worktree mode: checks for the named stash, attempts `git stash pop`, merges the branch into base, deletes the branch on success.
- If stash pop fails due to conflicts, alerts the user with `git stash list` guidance and does not proceed.

### F3: TOML Configuration (High)

Two-tier config with CLI management commands:

**Global config**: `~/.config/mat/config.toml` — user defaults applied to all projects.

**Project config**: `.mat.toml` in the git repo root — team-shared overrides. Project config takes precedence over global config for conflicting keys.

**Config keys (V1)**:

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_branch` | string | `"main"` | Branch used as source when `--source` is not provided. Auto-detected via `git symbolic-ref refs/remotes/origin/HEAD` if not set. |
| `delete_branch` | bool | `true` | Whether to delete the branch after successful close + merge |
| `merge_strategy` | string | `"merge-commit"` | Merge strategy on close: `merge-commit` (default) or `fast-forward` |
| `worktree_root` | string | — | Custom root directory for worktrees. If not set, uses `{repo_dir}.worktree/` |
| `tmux.enabled` | string | `"auto"` | `auto` (detect $TMUX), `always`, or `never` |

**CLI commands**:

- `mat config list` — show all effective config values (merged global + project)
- `mat config get <key>` — show a single config value
- `mat config set <key> <value>` — set a value in project-local config (or `--global` for global)

### F4: TMUX Auto-Detection (Medium)

- Check `$TMUX` environment variable to determine if running inside a tmux session.
- If inside tmux: create a new tmux window in the worktree directory, rename it to the task name, and copy the `cd` command to the tmux buffer (current behavior).
- If not inside tmux but `--use-tmux` is passed: attempt to create a new tmux session (or attach to existing one), then follow the tmux flow.
- If not inside tmux and no `--use-tmux` flag: open a new shell process in the worktree directory. The shell type is detected from `$SHELL`.
- Config option `tmux.enabled` can override detection: `always` forces tmux behavior, `never` forces non-tmux behavior regardless of environment.

### F5: Auto-Merge on Close (High)

When closing a task:

1. **Check for uncommitted changes**: If dirty, alert and stop. User must commit or stash manually.
2. **Merge the feature branch into the base branch**: Switch to the base branch (`git checkout {source}`), then `git merge {branch}` with the configured strategy (`--no-ff` for merge-commit, `--ff-only` for fast-forward).
3. **Handle merge failure**: If conflicts occur, alert the user with the branch name and conflict files. Leave both branches intact. Do not delete worktree or branch.
4. **On merge success**: Delete the worktree (or switch branch in no-worktree mode), delete the branch if `delete_branch` is true, close tmux window if applicable.
5. **`--no-merge` flag**: Skip the merge step entirely. Delete worktree and optionally branch, but leave merging to the user. Copy the merge command to clipboard/buffer for convenience.

## User Experience

### Primary Flow: Create (Worktree + TMUX)

```
$ mat feat login

ℹ Running prerequisite checks...
✓ Git repository detected
✓ Worktree name: dashboard-feat/login
ℹ Creating worktree: branch feat/login from main
✓ Worktree created at /path/to/dashboard.worktree/dashboard-feat/login

✓ Ready! Window 'dashboard-feat/login' is now open at:
  /path/to/dashboard.worktree/dashboard-feat/login

💡 To cd into the new worktree from other tmux panels:
  Press Ctrl-a then ] to paste the cd command
```

### Primary Flow: Create (No TMUX)

```
$ mat fix typo

✓ Git repository detected
ℹ Creating worktree: branch fix/typo from main
✓ Worktree created at /path/to/dashboard.worktree/dashboard-fix/typo

✓ Opening new shell in worktree directory...
  (Type 'exit' to return to your original directory)
```

### Primary Flow: Create (No-Worktree)

```
$ mat --no-worktree feat login

ℹ Stashing uncommitted changes (2 files)...
✓ Changes stashed as 'mat:auto:feat/login'
ℹ Creating branch: feat/login from main
✓ Switched to branch feat/login

⚠ No-worktree mode: changes are isolated to this branch, not a separate directory.
  Stashed changes can be restored with: git stash pop

✓ Ready to work on feat/login
```

### Primary Flow: Close (Auto-Merge Success)

```
$ mat --close

ℹ Checking for uncommitted changes...
✓ No uncommitted changes
ℹ Current branch: feat/login (from main)
ℹ Merging feat/login into main...
✓ Merge successful (merge commit)
ℹ Deleting worktree: dashboard-feat/login
✓ Worktree deleted
ℹ Deleting branch: feat/login
✓ Branch deleted
✓ TMUX window closed

💡 You are now on main. Feature merged successfully!
```

### Primary Flow: Close (Merge Conflict)

```
$ mat --close

ℹ Checking for uncommitted changes...
✓ No uncommitted changes
ℹ Current branch: feat/login (from main)
ℹ Merging feat/login into main...
ERROR: Merge conflict detected. The following files have conflicts:
  - src/auth.rs
  - tests/login_test.rs

⚠ Merge aborted. Both branches are intact.
  Resolve conflicts manually:
    1. git checkout main
    2. git merge feat/login
    3. Resolve conflicts and commit
    4. Run 'mat --close' again or 'mat --close --no-merge'
```

### Configuration Flow

```
$ mat config list

default_branch    = main          (project: .mat.toml)
delete_branch     = true           (global)
merge_strategy    = merge-commit   (global)
tmux.enabled      = auto           (default)

$ mat config set merge_strategy fast-forward
✓ Set merge_strategy = fast-forward in .mat.toml

$ mat config set --global default_branch develop
✓ Set default_branch = develop in ~/.config/mat/config.toml
```

## High-Level Technical Constraints

- **Single binary distribution**: mat must compile to a single Rust binary with no runtime dependencies beyond git and (optionally) tmux
- **Shell-out to git**: All git operations use `std::process::Command` to invoke `git` directly — no git library dependency (consistent with all competitor tools and mat's existing architecture)
- **Config format**: TOML parsed with `serde` + `toml` crates — the standard Rust CLI config format
- **Config merge strategy**: Project-local overrides global; CLI flags override both
- **Naming convention**: Worktree and branch names include task_type to prevent collisions (`{app}-{type}/{name}` for worktree, `{type}/{name}` for branch)
- **Error handling**: Named stashes (`mat:auto:{branch}` prefix) prevent stash collision; merge conflicts leave branches intact

## Non-Goals (Out of Scope)

- **File copying to worktrees**: No automatic `.env`, `.vscode/`, or `node_modules/` replication. Users manage these manually or via scripts. (V2 candidate)
- **Post-create command execution**: No `postCreateCmd` equivalent. Users add hooks via git or shell aliases. (V2 candidate)
- **TUI / interactive mode**: Mat remains a CLI-only tool with no interactive menus or fuzzy finders.
- **Shell integration and tab completion**: No `eval "$(mat init)"` wrapper or completion scripts. (V2 candidate)
- **Worktree path templating**: No `$BASE_PATH`, `$BRANCH_NAME`, variable substitution in paths.
- **Auto-push**: Mat does not push to remotes. Merge is local only.
- **Auto-commit**: Mat does not commit uncommitted changes on behalf of the user.
- **Zellij / WezTerm support**: V1 only supports tmux and plain shell. (V2 candidate)

## Phased Rollout Plan

### MVP (Phase 1)

- F1: Native worktree CRUD (replace branchlet)
- F5: Auto-merge on close (with conflict handling)
- `--no-merge` flag to skip merge on close
- TMUX auto-detection and `--use-tmux` flag
- `--no-worktree` flag with named stash guard
- Two-tier TOML config with `mat config` commands
- Config keys: `default_branch`, `delete_branch`, `merge_strategy`, `worktree_root`, `tmux.enabled`
- Updated naming convention including task_type

**Success criteria**: mat can create and close a worktree-based task end-to-end without branchlet installed. Close flow auto-merges successfully on clean branches and degrades gracefully on conflicts.

### Phase 2

- File copying to worktrees (config-driven glob patterns, inspired by branchlet's `worktreeCopyPatterns`)
- Post-create command execution
- Shell integration (`mat cd` command, tab completion)
- `mat doctor` health check command (verify git version, tmux availability, config validity)

**Success criteria**: Users can configure file copying and post-create hooks per project.

### Phase 3

- Zellij and WezTerm support
- `mat list` command to view all worktrees with status
- `mat switch <name>` for quick navigation between worktrees
- `mat status` overview of all active tasks across worktrees

**Success criteria**: mat supports multiple terminal multiplexers and provides workspace overview.

## Success Metrics

| Metric | Target | Measurement |
|--------|--------|-------------|
| Dependency elimination | 0 external CLIs (except git/tmux) | mat runs without branchlet installed |
| Setup time | < 2 min from install to first worktree | Time from `cargo install mat` to successful `mat feat my-task` |
| Create-to-close success rate | > 95% for clean merges | Error rate tracking in close mode |
| No-worktree adoption | > 20% of tasks use `--no-worktree` within 3 months | Usage analytics or survey |
| Config discoverability | > 80% of users find `mat config` without docs | User feedback |
| Close-with-merge success rate | > 90% of `mat --close` calls result in successful merge | Error rate tracking (conflicts vs. success) |

## Risks and Mitigations

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| Merge conflicts during auto-merge | Medium | High — user must resolve manually | Abort merge, leave both branches intact, print clear resolution steps |
| Stash pop failure in no-worktree mode | Low | Medium — changes stuck in stash | Named stashes with `mat:auto:` prefix; clear error message with `git stash list` guidance |
| Wrong base branch for merge | Low | High — merging into wrong branch | Show base branch name in close output; derive from `git symbolic-ref refs/remotes/origin/HEAD` |
| Uncommitted changes preventing close | Medium | Medium — user must commit first | Clear error message with `git status` suggestion |
| Worktree naming collision with old convention | Low | Low — existing worktrees use old names | Document the convention change; old worktrees continue to work until closed |
| Config file conflicts between global and project | Low | Low — wrong value used | Document merge strategy (local > global); `mat config list` shows effective values with source |

## Architecture Decision Records

- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](adrs/adr-001.md) — Decision to replace branchlet dependency with direct git worktree commands, add no-worktree mode, TOML config, and TMUX auto-detection
- [ADR-002: Lifecycle Manager Approach for Task Closing](adrs/adr-002.md) — Decision to auto-merge on close with configurable strategy, delete branch only on success, and support `--no-merge` flag

## Open Questions

- Should the auto-merge prompt show the base branch name and ask for confirmation before merging, or should it merge silently if there are no uncommitted changes?
- When `mat --close --no-merge` is used, should mat still delete the worktree/branch, or should it only disconnect from the task and leave cleanup to the user?
- Should `.mat.toml` be added to `.gitignore` by default, or should it be committed for team sharing? (Different teams may want different defaults.)
- How should mat handle the transition period where some repos still have branchlet worktrees? Should mat detect these and offer migration?
- What should happen when `git worktree add` encounters a locked worktree? Error with guidance, or force-unlock and retry?
- Should the no-worktree stash guard also handle `--include-untracked` files, or only tracked+modified files?