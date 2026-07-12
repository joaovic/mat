---
type: Domain Concept
title: Herdr Integration
description: Integration between mat and Herdr, the terminal workspace manager.
tags: [herdr, workspace, integration]
timestamp: 2026-07-12T17:30:00Z
---

# Herdr Integration

[Herdr](https://herdr.dev) is a terminal workspace manager for AI coding agents.
When Herdr is running and `herdr.enabled` is not `never`, mat automatically
creates a Herdr workspace for each new task.

## How It Works

When you run `mat create` with Herdr available:

1. mat creates the Git worktree
2. mat calls `herdr workspace create --cwd <worktree-path> --label <name>`
3. A new workspace is created with the worktree directory as its working directory
4. The workspace is automatically cleaned up by `mat close`

## Configuration

| Config Key | Values | Behavior |
|------------|--------|----------|
| `herdr.enabled` | `auto` | Enable if Herdr is running |
| `herdr.enabled` | `always` | Always attempt Herdr integration |
| `herdr.enabled` | `never` | Skip Herdr integration entirely |

## Disabling Herdr Integration

```bash
mat config set --global herdr.enabled never
```
