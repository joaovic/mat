# mat - Multi-Agent Task CLI

A CLI tool that streamlines feature development using Git worktrees and TMUX integration.

## Overview

`mat` (Multi-Agent Task) manages your development workflow by creating isolated Git worktrees for new features or tasks, optionally integrated with TMUX. Inspired by tools in the Git worktree ecosystem, `mat` implements its own native workflow directly on top of Git commands.

It will:

1. Check prerequisites (Git repo, optional TMUX)
2. Create a new Git branch and worktree (or just a branch with `--no-worktree`)
3. Open a new TMUX window in the worktree directory (when available)
4. Name the window following the pattern: `{app}-{type}/{name}`
5. Copy a `cd` command to TMUX buffer for easy access from other panels

## Requirements

- **Git** — Current directory must be a Git repository
- **TMUX** — Optional, for window management (auto-detected)

## Installation

```bash
# Build the CLI
cd mat
cargo build --release

# Copy to your PATH
cp target/release/mat ~/local/bin/mat
# or
sudo cp target/release/mat /usr/local/bin/mat
```

## Usage

### Create a task

#### Arguments

| Argument | Description |
|----------|-------------|
| `type` | Task type (e.g., feat, fix, chore, refactor) |
| `name` | Task name (e.g., increase-counter) |

#### Options

| Option | Description |
|--------|-------------|
| `-s, --source <branch>` | Base branch to create from (defaults to current branch) |
| `--no-worktree` | Skip worktree creation, only create a branch |
| `--use-tmux` | Force TMUX window creation even outside TMUX |

#### Examples

```bash
# Create feature from current branch
mat create feat increase-counter

# Create feature from specific branch
mat create feat increase-counter -s develop

# Create bugfix
mat create fix login-error

# Create chore
mat create chore update-deps

# Create without worktree (branch only)
mat create fix hotfix --no-worktree
```

#### Output Example

```
ℹ Running prerequisite checks...
✓ Current directory is a git repository
ℹ Creating worktree: name=dashboard-feat/increase-counter, source=main, branch=feat/increase-counter
✓ Worktree created at: /path/to/project.worktree/dashboard-feat/increase-counter
✓ TMUX window created
✓ Window renamed to: dashboard-feat/increase-counter
✓ CD command copied to TMUX buffer

✓ Ready! Window 'dashboard-feat/increase-counter' is now open at: /path/to/project.worktree/dashboard-feat/increase-counter

💡 To cd into the new worktree from other TMUX panels:
  Press [Ctrl-a] then ] to paste the cd command
```

> Note: The TMUX prefix (Ctrl-a, Ctrl-b, etc.) is automatically detected from your TMUX configuration.

### Close a task

```bash
mat close [OPTIONS]
```

Closes the current task by checking out the source branch, optionally merging changes, removing the worktree, and deleting the feature branch.

| Option | Description |
|--------|-------------|
| `--no-merge` | Skip merge on close |

#### Examples

```bash
# Close and merge
mat close

# Close without merging
mat close --no-merge
```

### Configuration

`mat` supports two-tier configuration via TOML files:

- **Global**: `~/.config/mat/config.toml`
- **Project**: `.mat.toml` (in repo root)

Project config overrides global config.

#### Config commands

```bash
# List effective configuration with sources
mat config list

# Get a single config value
mat config get <key>

# Set a config value
mat config set <key> <value>

# Set globally
mat config set --global <key> <value>
```

#### Available config keys

| Key | Description | Default |
|-----|-------------|---------|
| `default_branch` | Default base branch | `main` |
| `delete_branch` | Delete branch after close | `true` |
| `merge_strategy` | `merge-commit` or `fast-forward` | `merge-commit` |
| `worktree_root` | Custom worktree path template | `{repo}.worktree/{name}` |
| `tmux.enabled` | `auto`, `always`, or `never` | `auto` |

#### Example `.mat.toml`

```toml
default_branch = "develop"
merge_strategy = "fast-forward"
```

## Window Naming Convention

Windows are named following this pattern:

```
{app-name}-{type}/{name}
```

Example: For app `dashboard`, running `mat feat increase-counter`, the window will be named `dashboard-feat/increase-counter`.

## Inspirations

The workflow design was inspired by ideas from the Git worktree tooling ecosystem, particularly the ergonomics of managing per-feature worktrees. `mat` implements its own native Git worktree management and adds integrated configuration, auto-merge on close, and flexible TMUX handling.

## License

MIT
