---
type: Domain Concept
title: Git Worktrees
description: How mat uses Git worktrees to provide isolated development environments per task.
tags: [git, worktree, workflow]
timestamp: 2026-07-12T17:30:00Z
---

# Git Worktrees

## What is a Git Worktree?

A Git worktree is a linked copy of a repository that allows you to check out
multiple branches simultaneously. Each worktree has its own working directory
and index, but shares the same Git object database as the parent repository.

```
Parent repository (main branch)
└── .git/
    ├── objects/       ← shared
    ├── HEAD           → ref: main
    └── worktrees/
        └── feat/my-feature/  ← worktree metadata

Worktree directory (/path/to/repo.worktree/app-feat/my-feature)
├── .git              → file pointing to parent .git/worktrees/...
└── src/              ← own working tree
```

## How mat Uses Worktrees

mat creates worktrees following this naming convention:

```
{app-name}.worktree/{app-name}-{type}/{name}
```

Example: `mat create feat increase-counter` in the `dashboard` repo creates:

```
/path/to/dashboard.worktree/dashboard-feat/increase-counter
```

## Benefits

- **No branch switching** — Keep `main` checked out while working on a feature
- **Parallel tasks** — Multiple worktrees for different features simultaneously
- **Safe stashing** — No need to stash uncommitted changes when context-switching
- **Clean separation** — Each worktree has its own dependencies, node_modules, etc.

## No-worktree Mode

When `--no-worktree` is specified, mat creates only a branch (no linked
worktree). A named stash guard (`mat:auto:<branch>`) protects uncommitted
changes in the current directory.

## Lifecycle

1. **Create**: mat calls `git worktree add` with the new branch
2. **Use**: The user works in the worktree directory
3. **Close**: mat optionally merges changes back to the base branch and removes
   the worktree with `git worktree remove`
