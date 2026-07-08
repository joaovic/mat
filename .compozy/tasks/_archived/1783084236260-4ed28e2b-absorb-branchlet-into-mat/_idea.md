# Absorb Branchlet into Mat

## Overview

Remove the external `branchlet` (Node.js) dependency from mat by absorbing its core git worktree CRUD operations — create, delete, and list — as direct `git worktree` commands. At the same time, add three new capabilities: a no-worktree mode that creates branches instead of worktrees (with stash guard), TMUX auto-detection from the `$TMUX` environment variable with a `--use-tmux` flag override, and a two-tier TOML configuration system. The result is a single Rust binary that manages the full task lifecycle without any external dependency beyond git and (optionally) tmux.

This is a Quick Win that eliminates setup friction (Node.js + branchlet install), makes mat self-contained, and adds the flexibility users requested for working without worktrees or outside TMUX.

## Problem

Developers using mat to manage git worktree-based task environments must install Node.js and the `branchlet` npm package before they can use mat. This creates a dependency on a 10k-line TypeScript project for three shell commands that mat already wraps. Meanwhile, mat has no configuration file, requires tmux unconditionally, and offers no fallback for developers who want to work on branches instead of worktrees.

### Market Data

- 10+ git worktree management tools emerged in 2025-2026 (Worktrunk, gw, git-parsec, wtp, rsworktree, etc.), most written in Rust
- All mature tools shell out to `git worktree` rather than using `git2` — the operation is standard CLI territory
- AI agent orchestration (Claude Code, Codex) is the #1 driver of worktree adoption; Anthropic officially recommends worktrees for parallel agent sessions
- TOML is the de facto standard for Rust CLI configuration (used by Cargo, Worktrunk, git-parsec, gwm)
- Terminal multiplexer integration (tmux, Zellij, WezTerm) is table-stakes for any serious worktree tool

## Core Features

| #  | Feature                    | Priority | Description                                                                                         |
|----|----------------------------|----------|-----------------------------------------------------------------------------------------------------|
| F1 | Worktree CRUD (native)     | Critical | Replace `branchlet create/list/delete` with direct `git worktree add/list/remove` commands         |
| F2 | No-worktree mode           | High     | `--no-worktree` flag: creates a branch via `git checkout -b` with named stash guard instead of worktree |
| F3 | TOML configuration          | High     | Two-tier config: global `~/.config/mat/config.toml` + project-local `.mat.toml` (local overrides global) |
| F4 | TMUX auto-detection        | Medium   | Detect `$TMUX` env var for automatic tmux integration; `--use-tmux` flag for explicit opt-in outside tmux |
| F5 | Configurable branch delete | Medium   | `delete_branch` config option (default: YES) with confirmation prompt on close                    |

## KPIs

| KPI                               | Target                     | How to Measure                                  |
|-----------------------------------|----------------------------|-------------------------------------------------|
| Dependency elimination            | 0 external CLIs (except git/tmux) | `mat` binary runs without `branchlet` installed |
| Setup time reduction              | < 2 min to first worktree  | Time from `cargo install mat` to successful `mat feat my-task` |
| No-worktree adoption              | > 30% of tasks use `--no-worktree` within 3 months | Usage analytics / survey                        |
| Close-task completion rate        | > 95% create→close cycles succeed without manual intervention | Error rate tracking in close mode               |
| Config discoverability            | > 80% of users find settings in `mat.toml` without docs | User feedback survey                            |

## Feature Assessment

| Criteria            | Question                                            | Score     |
|---------------------|-----------------------------------------------------|-----------|
| **Impact**          | How much more valuable does this make the product?  | Must do   |
| **Reach**           | What % of users would this affect?                  | Must do   |
| **Frequency**       | How often would users encounter this value?         | Must do   |
| **Differentiation** | Does this set us apart or just match competitors?   | Strong    |
| **Defensibility**   | Is this easy to copy or does it compound over time?  | Maybe     |
| **Feasibility**     | Can we actually build this?                          | Must do   |

Leverage type: Quick Win

## Council Insights

- **Recommended approach:** Implement the original vision (absorb core worktree CRUD + no-worktree mode + TMUX auto-detect + TOML config) with council-mandated guardrails: named stashes with `mat:auto:` prefix for the stash guard, explicit confirmation prompt for branch deletion, and clear disclaimers that no-worktree mode is not equivalent to worktree isolation
- **Key trade-offs:**
  - Stash guard inverts worktree safety guarantee (moves state vs preserves state) — mitigated by named stashes and clear UX messaging
  - Default-YES branch deletion is convenient but risky — mitigated by confirmation prompt
  - TMUX auto-detect adds a code path but is trivial (`$TMUX` check) — accepted for V1
- **Risks identified:**
  - Git worktree edge cases (locked worktrees, bare repos, detached HEADs, corrupted worktrees) currently handled by branchlet must now be handled by mat
  - Named stash pop can fail on merge conflicts — must provide clear error messages pointing to `git stash list`
  - Two-tier config requires merge strategy documentation (local > global)
- **Stretch goal (V2+):** Task lifecycle state tracking (created → in-progress → review → done) across worktrees

## Out of Scope (V1)

- **File copying to worktrees** — Branchlet's `worktreeCopyPatterns` and `worktreeCopyIgnores` features; users can manage `.env` files manually or add scripts
- **Post-create command execution** — No `postCreateCmd` equivalent; users can add hooks via git or shell aliases
- **TUI / interactive mode** — Mat remains a CLI-only tool; no interactive menus or fuzzy finders
- **Shell integration (`cd` into worktrees)** — No wrapper function or tab completion; users navigate via tmux windows or manual `cd`
- **Worktree path templating** — No `$BASE_PATH`, `$BRANCH_NAME`, or `$SOURCE_BRANCH` variable substitution
- **Update checking** — No npm registry check or version comparison

## Architecture Decision Records

- [ADR-001: Absorb Branchlet Core Worktree Operations into Mat](adrs/adr-001.md) — Replacing branchlet dependency with direct git commands, adding no-worktree mode, TOML config, and TMUX auto-detection

## Open Questions

- Should the confirmation prompt for branch deletion be a simple Y/N, or should it show the branch name and last commit before confirming?
- What should happen when `git worktree add` encounters a worktree locked by another process? Error with guidance, or force-remove and retry?
- Should `.mat.toml` be added to `.gitignore` automatically, or left as a project-committed config file?
- How should mat handle the transition from branchlet to native operations? Should it detect an existing `branchlet` installation and warn, or silently take over?
- What should the default value for `default_branch` be in config? `main`, or auto-detect from `git symbolic-ref refs/remotes/origin/HEAD`?