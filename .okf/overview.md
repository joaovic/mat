---
type: Overview
title: mat Project Overview
description: Multi-Agent Task CLI — manages Git worktrees for feature development.
tags: [mat, cli, rust, worktree]
timestamp: 2026-07-12T17:30:00Z
---

# Overview

**mat** (Multi-Agent Task) is a CLI tool written in Rust that streamlines
feature development using [Git worktrees](/domain/worktrees.md) and optional
[Herdr](/domain/herdr.md) integration.

## Purpose

mat automates the workflow of creating isolated development environments for
each feature or task. Instead of manually running Git commands to create
branches and worktrees, mat handles the entire lifecycle:

1. **Create** — Creates a new Git branch and worktree from a base branch,
   optionally creating a Herdr workspace
2. **Close** — Removes the worktree, optionally merges changes, and cleans up
   the branch
3. **Configure** — Manages persistent settings via TOML configuration files

## Key Features

- **Isolated environments**: Each task gets its own Git worktree — no switching
  branches, no stashing changes
- **Herdr integration**: Automatically creates and manages terminal workspaces
  when Herdr is running
- **No-worktree mode**: Branch-only workflow for constrained environments
- **Two-tier configuration**: Global defaults with per-project overrides
- **Zero external runtime dependencies**: Single Rust binary, no Node.js required

## Inspiration

The workflow design was inspired by the Git worktree tooling ecosystem, but
mat implements its own native Git worktree management rather than wrapping
external tools like Branchlet.
