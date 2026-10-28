---
type: Guide
title: Setup & Build
description: Instructions for building and installing mat locally.
tags: [setup, build, install]
timestamp: 2026-07-12T17:30:00Z
---

# Setup & Build

## Prerequisites

- **Rust toolchain** — Install via [rustup.rs](https://rustup.rs)
- **Git** — Must be installed and available on PATH

Optional:
- **Herdr** — Terminal workspace manager (see [herdr integration](/domain/herdr.md))

## Build

```bash
# Clone the repository
git clone git@github.com:joaovic/mat.git
cd mat

# Build release binary
cargo build --release

# The binary is at target/release/mat
```

## Install

```bash
# Linux / macOS
sudo cp target/release/mat /usr/local/bin/mat

# Verify
mat --version
```

## Cross-compile to Windows

```bash
# From Linux
sudo apt install mingw-w64
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
# Binary at target/x86_64-pc-windows-gnu/release/mat.exe
```

## Run Tests

```bash
cargo test
```
