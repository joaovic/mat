---
type: Guide
title: Development Workflow
description: How to use mat in daily development for feature branches.
tags: [guide, workflow, development]
timestamp: 2026-07-12T17:30:00Z
---

# Development Workflow

## Daily Usage

### Start a new task

```bash
# From the main branch of your project
mat create feat my-new-feature

# mat will:
# 1. Create branch feat/my-new-feature from main
# 2. Create a Git worktree in <repo>.worktree/<app>-feat/my-new-feature
# 3. (Optional) Open a Herdr workspace in that directory
```

### Work on the task

```bash
cd /path/to/repo.worktree/app-feat/my-new-feature
# Make changes, commit, test...

git add -A
git commit -m "feat(scope): description"
git push -u origin HEAD
```

### Finish the task

```bash
# From the worktree directory
mat close

# Or from anywhere:
mat close --no-merge
```

## Best Practices

- **One task per worktree** — Each feature gets its own isolated environment
- **Commit early, push often** — Worktrees are independent but share the Git
  object database; pushing keeps backups
- **Use `--no-worktree` for quick fixes** — Small changes don't need a full
  worktree
- **Clean up regularly** — `mat close` removes worktrees automatically
