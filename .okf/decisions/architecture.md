---
type: Decision
title: Architecture Decisions
description: Key architectural decisions and rationale for the mat project.
tags: [decision, architecture, cli, rust]
timestamp: 2026-07-12T17:30:00Z
---

# Architecture Decisions

## Why Rust?

- **Performance**: Near-zero overhead for CLI operations
- **Safety**: Memory safety without garbage collection
- **Distribution**: Single statically-linked binary per platform
- **CLI ecosystem**: Clap provides industry-standard argument parsing

## Why TOML for Configuration?

- **Rust-native**: Built-in TOML support via `toml` crate with `serde`
- **Human-readable**: More ergonomic than JSON for hand-edited config files
- **Existing convention**: Cargo uses TOML, so Rust developers are familiar

## Why Shell-out to Git Instead of libgit2?

- The safe Rust API of `git2` doesn't expose `git worktree add/remove`
- Shelling out is the industry standard approach (all CLI tools do this)
- Avoids complex C dependency (libgit2)

## Why Two-tier Configuration?

- **Global**: Sensible defaults that apply everywhere
- **Project**: Per-repo overrides without editing global config
- **Override**: Project config merges over global for conflicting keys
