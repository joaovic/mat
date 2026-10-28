---
type: CLI Reference
title: Commands
description: Complete reference for mat CLI subcommands and options.
tags: [cli, commands, create, close, config]
timestamp: 2026-07-12T17:30:00Z
---

# Commands

## mat create

Creates a new task with an isolated Git worktree.

### Syntax

```
mat create <type> <name> [options]
```

### Arguments

| Argument | Description | Example |
|----------|-------------|---------|
| `type`   | Task type: `feat`, `fix`, `chore`, `refactor` | `feat` |
| `name`   | Task name in kebab-case | `add-okf-bundle` |

### Options

| Option | Description |
|--------|-------------|
| `-s, --source <branch>` | Base branch to create from (defaults to current branch) |
| `--no-worktree` | Skip worktree creation, create a branch only |

### Examples

```
mat create feat increase-counter
mat create feat increase-counter -s develop
mat create fix login-error
mat create chore update-deps
mat create fix hotfix --no-worktree
```

### Output

```
✓ Git repository detected
ℹ Creating worktree: branch feat/increase-counter from main
✓ Worktree created at: /path/to/project.worktree/dashboard-feat/increase-counter

✓ Ready! Worktree created at: ...
```

When Herdr is running, a new workspace is also created.

## mat close

Closes the current task worktree.

### Syntax

```
mat close [options]
```

### Options

| Option | Description |
|--------|-------------|
| `--no-merge` | Skip merge on close |

### Examples

```
mat close         # Close and merge
mat close --no-merge  # Close without merging
```

## mat config

Manages configuration settings.

### Subcommands

| Command | Description |
|---------|-------------|
| `mat config list` | Show effective configuration with sources |
| `mat config get <key>` | Get a single config value |
| `mat config set <key> <value>` | Set a config value (project scope) |
| `mat config set --global <key> <value>` | Set a config value (global scope) |
