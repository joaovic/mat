---
type: Source Layout
title: Source Code Organization
description: Module structure and file organization of the mat source tree.
tags: [architecture, source, modules]
timestamp: 2026-07-12T17:30:00Z
---

# Source Layout

The project is a single Cargo workspace with a flat `src/` hierarchy.

```
mat/
├── Cargo.toml            # Package manifest (name: mat, edition: 2021)
├── Cargo.lock            # Dependency lockfile
├── src/
│   ├── main.rs           # Entry point: dispatches to command handlers
│   ├── cli.rs            # Clap-derived CLI argument parser
│   ├── commands/
│   │   ├── mod.rs        # Module exports
│   │   ├── create.rs     # `mat create` implementation
│   │   ├── close.rs      # `mat close` implementation
│   │   └── config.rs     # `mat config` subcommands
│   ├── config.rs         # Two-tier TOML config loading & merging
│   ├── display.rs        # Terminal output helpers (console crate)
│   ├── error.rs          # Custom error types and formatting
│   ├── git.rs            # Git worktree operations (shelling out to `git`)
│   ├── naming.rs         # Worktree and branch naming conventions
│   └── herdr.rs          # Herdr workspace integration
├── tests/                # Integration tests
├── docs/                 # Documentation files
└── .opencode/            # OpenCode AI agent configuration
```

## Module Responsibilities

| Module | Responsibility |
|--------|---------------|
| `main.rs` | Command dispatch and error handling |
| `cli.rs` | CLI argument parsing via Clap derive API |
| `commands/create.rs` | Worktree/branch creation logic |
| `commands/close.rs` | Worktree removal and optional merge |
| `commands/config.rs` | Config get/set/list operations |
| `config.rs` | TOML file loading, serialization, merging |
| `display.rs` | Consistent terminal UI (icons, colors, formatting) |
| `error.rs` | MatError enum, Display impl, error messages |
| `git.rs` | Git commands via `std::process::Command` |
| `naming.rs` | Naming conventions for branches, worktrees |
| `herdr.rs` | Herdr IPC and workspace lifecycle |
