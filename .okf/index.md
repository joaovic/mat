---
okf_version: "0.1"
---

# mat — Multi-Agent Task CLI

Knowledge bundle for the **mat** project, a Rust CLI tool that streamlines
feature development using Git worktrees and optional Herdr integration.

## CLI

* [Commands](/cli/commands.md) — Complete CLI command reference (create, close, config)
* [Configuration](/cli/configuration.md) — Two-tier TOML configuration guide

## Domain Concepts

* [Git Worktrees](/domain/worktrees.md) — How mat manages isolated development environments
* [Herdr Integration](/domain/herdr.md) — Terminal workspace manager integration

## Architecture

* [Tech Stack](/architecture/tech-stack.md) — Rust, dependencies, and toolchain
* [Source Layout](/architecture/source-layout.md) — Codebase organization

## Guides

* [Setup & Build](/guides/setup.md) — How to build and install mat
* [Development Workflow](/guides/development.md) — Using mat in daily development

## Technical Decisions

* [Absorb Branchlet into Mat](/decisions/absorb-branchlet.md) — ADR: Replacing external CLI dependency with native Git worktree operations
* [Architecture](/decisions/architecture.md) — Key architectural decisions
