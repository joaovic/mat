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
- **TMUX / PSMUX** — Optional, for window management (auto-detected)

### Windows

- **PSMUX** — Native Windows tmux replacement. Install via:
  ```powershell
  winget install psmux
  # or: cargo install psmux
  # or: scoop install psmux
  ```
  PSMUX installs `psmux.exe`, `pmux.exe`, and `tmux.exe` — all identical. `mat` calls `tmux`, so it works immediately.

## Installation

### Linux / macOS

```bash
cargo build --release
cp target/release/mat ~/local/bin/mat
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

### Advanced Settings (`.mat/settings.toml`)

`mat` supports advanced worktree configuration via a separate settings file. This file uses a section-based format to allow future expansion.

#### Settings File Location

Settings are loaded with the following precedence (highest to lowest):

1. **Project**: `<repo>/.mat/settings.toml`
2. **Global**: `$HOME/.mat/settings.toml` (or `%USERPROFILE%\.mat\settings.toml` on Windows)
3. **Default**: Built-in defaults (created automatically at `<repo>/.mat/settings.toml` if no file exists)

#### Available Settings

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `worktree.copy_patterns` | `string[]` | `[".env*", ".vscode/**"]` | Glob patterns for files to copy into the worktree |
| `worktree.copy_ignores` | `string[]` | `["**/dist/**", ...]` | Glob patterns for files to ignore during copy |
| `worktree.path_template` | `string` | `"$BASE_PATH.wtree"` | Template for the worktree directory path |
| `worktree.post_create_cmd` | `string[]` | `["npm install"]` | Commands to run after worktree creation |
| `worktree.terminal_command` | `string` | `""` | Custom terminal command (empty = system default) |
| `worktree.delete_branch_with_worktree` | `bool` | `false` | Delete branch when removing worktree |

#### Path Template Variables

The `path_template` supports these variables:

| Variable | Description | Example |
|----------|-------------|---------|
| `$BASE_PATH` | Repository root path | `/home/user/project` |
| `$APP_NAME` | Application name (directory basename) | `myapp` |
| `$TYPE` | Task type | `feat`, `fix`, `chore` |
| `$NAME` | Task name | `login-page` |

#### Example `.mat/settings.toml`

```toml
[worktree]
copy_patterns = [
    ".env*",
    ".vscode/**",
    "docker-compose.yml"
]

copy_ignores = [
    "**/dist/**",
    "**/node_modules/**",
    "**/.git/**",
    "**/Thumbs.db",
    "**/.DS_Store"
]

path_template = "$BASE_PATH.wtree"

post_create_cmd = ["npm install", "npm run build"]

terminal_command = ""

delete_branch_with_worktree = false
```

#### Post-Create Commands

Commands are executed using `sh -c` on Unix-like systems and `powershell -Command` on Windows:

```toml
# Node.js project
post_create_cmd = ["npm install", "npm run build"]

# Python project
post_create_cmd = ["pip install -r requirements.txt"]

# Rust project
post_create_cmd = ["cargo build"]
```

#### Example: Custom Worktree Path

```toml
[worktree]
path_template = "/tmp/worktrees/$APP_NAME/$TYPE/$NAME"
```

This will create worktrees at `/tmp/worktrees/myapp/feat/login` instead of the default `<repo>.worktree/` location.

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
