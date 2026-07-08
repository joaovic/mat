# mat - Multi-Agent Task CLI

A CLI tool that streamlines feature development using Git worktrees and Herdr integration.

## Overview

`mat` (Multi-Agent Task) manages your development workflow by creating isolated Git worktrees for new features or tasks, optionally integrated with Herdr (terminal workspace manager). Inspired by tools in the Git worktree ecosystem, `mat` implements its own native workflow directly on top of Git commands.

It will:

1. Check prerequisites (Git repo, optional Herdr)
2. Create a new Git branch and worktree (or just a branch with `--no-worktree`)
3. Create a new Herdr workspace in the worktree directory (when available)
4. Name the workspace following the pattern: `{app}-{type}/{name}`

## Requirements

- **Git** — Current directory must be a Git repository
- **Herdr** (optional) — Terminal workspace manager for AI coding agents. Install via:
  ```bash
  curl -fsSL https://herdr.dev/install.sh | sh
  ```
  When Herdr is not running, `mat` creates the worktree without workspace integration.

### Windows

Windows support is currently limited to Git worktree management (Herdr workspace creation is skipped on Windows).

## Installation

### Linux / macOS

```bash
cargo build --release
cp target/release/mat /usr/local/bin/mat
```

### Windows (native build)

```powershell
# Prerequisites:
#   1. Install Rust from https://rustup.rs
#   2. Add Windows target:
rustup target add x86_64-pc-windows-msvc

# Build
cargo build --release --target x86_64-pc-windows-msvc

# Copy mat.exe to a directory in your PATH
copy target\x86_64-pc-windows-msvc\release\mat.exe C:\tools\mat.exe
```

### Cross-compile from Linux to Windows

```bash
# Add MinGW-w64 toolchain
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu

cargo build --release --target x86_64-pc-windows-gnu
# Binary at: target/x86_64-pc-windows-gnu/release/mat.exe
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
✓ Git repository detected
ℹ Creating worktree: branch feat/increase-counter from main
✓ Worktree created at: /path/to/project.worktree/dashboard-feat/increase-counter

✓ Ready! Worktree created at: /path/to/project.worktree/dashboard-feat/increase-counter
```

> When Herdr is running, a new workspace is also created with the worktree as its working directory.

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
| `herdr.enabled` | `auto`, `always`, or `never` | `auto` |

#### Example `.mat.toml`

```toml
default_branch = "develop"
merge_strategy = "fast-forward"
```

## Worktree Naming Convention

Worktree directories follow this pattern:

```
{app-name}.worktree/{app-name}-{type}/{name}
```

Example: For app `dashboard`, running `mat feat increase-counter`, the worktree will be at `dashboard.worktree/dashboard-feat/increase-counter`.

## Herdr Integration

When Herdr is running and `herdr.enabled` is not `never`, `mat create` also:

1. Creates a new Herdr workspace via `herdr workspace create --cwd <path> --label <name>`
2. The workspace is automatically cleaned up by `mat close`

### Disabling Herdr integration

```bash
mat config set --global herdr.enabled never
```

## Inspirations

The workflow design was inspired by ideas from the Git worktree tooling ecosystem, particularly the ergonomics of managing per-feature worktrees. `mat` implements its own native Git worktree management and adds integrated configuration, auto-merge on close, and flexible terminal workspace handling.

## License

MIT
