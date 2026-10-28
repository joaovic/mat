---
type: Configuration Guide
title: Configuration
description: Two-tier TOML configuration system for mat.
tags: [cli, config, toml, settings]
timestamp: 2026-07-12T17:30:00Z
---

# Configuration

mat uses a two-tier TOML configuration system:

- **Global**: `~/.config/mat/config.toml`
- **Project**: `.mat.toml` (in repository root)

Project-level settings override global settings for conflicting keys.

## Available Keys

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_branch` | string | `main` | Default base branch for worktree creation |
| `delete_branch` | bool | `true` | Delete branch after close |
| `merge_strategy` | enum | `merge-commit` | Merge strategy: `merge-commit` or `fast-forward` |
| `worktree_root` | string | `{repo}.worktree/{name}` | Custom worktree path template |
| `herdr.enabled` | enum | `auto` | Herdr integration: `auto`, `always`, or `never` |

## Example

```toml
# .mat.toml
default_branch = "develop"
merge_strategy = "fast-forward"
```

## Commands

```bash
# List effective config
mat config list

# Get a value
mat config get herdr.enabled

# Set a value (project scope)
mat config set default_branch develop

# Set a value (global scope)
mat config set --global herdr.enabled never
```
