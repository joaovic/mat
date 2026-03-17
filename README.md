# mat - Multi-Agent Task CLI

A CLI tool that creates a new TMUX window with a Git worktree for developing new features or tasks.

## Overview

`mat` (Multi-Agent Task) streamlines the workflow of creating a new branch and worktree for developing features. It:

1. Checks prerequisites (TMUX running, Branchlet installed, Git repo)
2. Creates a new Git worktree using Branchlet
3. Opens a new TMUX window in the worktree directory
4. Names the window following the pattern: `{app}-{type}/{name}`
5. Copies a `cd` command to TMUX buffer for easy access from other panels

## Requirements

- **TMUX** - Must be running
- **Branchlet** - Git worktree manager ([https://github.com/raghavpillai/branchlet](https://github.com/raghavpillai/branchlet))
- **Git** - Current directory must be a Git repository
- **Branchlet config** - `~/.branchlet/settings.json` must exist

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

```bash
mat <type> <name> [-s|--source <base-branch>]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `type` | Task type (e.g., feat, fix, chore, refactor) |
| `name` | Task name (e.g., increase-counter) |
| `-s, --source` | Base branch to create worktree from (optional, defaults to current branch) |

### Examples

```bash
# Create feature from current branch
mat feat increase-counter

# Create feature from specific branch
mat feat increase-counter -s develop

# Create bugfix
mat fix login-error

# Create chore
mat chore update-deps
```

### Output Example

```
ℹ Running prerequisite checks...
✓ TMUX is running
✓ Branchlet is installed
✓ Current directory is a git repository
✓ Branchlet config exists
ℹ Creating worktree: name=dashboard-increase-counter, source=main, branch=feat/increase-counter
✓ Worktree created at: /path/to/project.worktree/dashboard-increase-counter
✓ TMUX window created
✓ Window renamed to: dashboard-feat/increase-counter
✓ CD command copied to TMUX buffer

✓ Ready! Window 'dashboard-feat/increase-counter' is now open at: /path/to/project.worktree/dashboard-increase-counter

💡 To cd into the new worktree from other TMUX panels:
  Press [Ctrl-a] then ] to paste the cd command
```

> Note: The TMUX prefix (Ctrl-a, Ctrl-b, etc.) is automatically detected from your TMUX configuration.

## Window Naming Convention

Windows are named following this pattern:

```
{app-name}-{type}/{name}
```

Example: For app `dashboard`, running `mat feat increase-counter`, the window will be named `dashboard-feat/increase-counter`.

## License

MIT
