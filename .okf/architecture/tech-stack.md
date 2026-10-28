---
type: Tech Stack
title: Technology Stack
description: Rust ecosystem, dependencies, and toolchain used by mat.
tags: [rust, dependencies, tech-stack]
timestamp: 2026-07-12T17:30:00Z
---

# Tech Stack

## Language

**Rust** (edition 2021) — Chosen for performance, safety, and zero-cost
abstractions. The CLI produces a single statically-linked binary with no
external runtime dependencies.

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `clap` | 4.5 | CLI argument parsing (derive API, subcommands) |
| `serde` | 1 | Serialization/deserialization for TOML config |
| `serde_json` | 1 | JSON output support |
| `toml` | 0.8 | TOML config file parsing |
| `console` | 0.15 | Terminal output styling (checkmarks, icons, colors) |
| `dirs` | 5.0 | Platform-standard directory paths |
| `glob` | 0.3 | Pattern matching for file paths |

## Dev Dependencies

| Crate | Purpose |
|-------|---------|
| `tempfile` | Temporary directories for integration tests |

## Toolchain

- **Build**: `cargo build` / `cargo build --release`
- **Test**: `cargo test`
- **Cross-compile**: Targets include `x86_64-pc-windows-gnu` for Windows builds
- **No Node.js required**: After ADR-001 (absorb branchlet), mat has zero
  JavaScript dependencies
