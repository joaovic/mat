---
type: Decision
title: Absorb Branchlet Core Worktree Operations into Mat
description: ADR-001 decision to replace the external Branchlet CLI dependency with direct Git worktree commands.
tags: [decision, adr, branchlet, git]
timestamp: 2026-07-12T17:30:00Z
---

# ADR-001: Absorb Branchlet Core Worktree Operations into Mat

## Status

Accepted (2026-05-19)

## Context

Mat originally depended on `branchlet` (a Node.js CLI) for Git worktree CRUD
operations. This meant users had to install Node.js before using mat, adding
friction.

## Decision

Replace `branchlet` calls with direct `git worktree` shell commands. This
eliminates the Node.js dependency and makes mat a single Rust binary.

### Changes Introduced

1. **No-worktree mode** (`--no-worktree`): Branch-only workflow with named
   stash guard `mat:auto:<branch>`
2. **TMUX auto-detection**: Check `$TMUX` to automatically use tmux
3. **TOML configuration**: Two-tier (global + project) via `serde`
4. **Branch deletion on close**: Configurable with confirmation prompt

## Consequences

- Single Rust binary, no external dependencies
- Worktree operations become first-class and configurable
- TMUX auto-detection reduces friction
- Maintenance of git worktree edge cases now owned by mat

## Alternatives Considered

- **git2 crate**: Rejected because it doesn't expose `worktree_add/remove`
  in its safe API
- **No-worktree as graceful exit**: Rejected — user explicitly requested it
