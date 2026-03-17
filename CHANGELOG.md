# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2026-03-17

### Added
- New `--close` / `-c` flag to close a previously created task worktree
  - Checks for uncommitted changes before closing (prevents closing if dirty)
  - Deletes worktree via Branchlet
  - Closes current TMUX window and returns to window 0
  - Saves merge command to TMUX clipboard for easy paste
  - Prints merge command to console for manual use
  - Respects user's custom TMUX prefix configuration

## [0.1.0] - 2026-03-17

### Added
- Initial release of `mat` (Multi-Agent Task CLI)
- Prerequisite checks for TMUX, Branchlet, Git repo, and Branchlet config
- Git worktree creation using Branchlet
- New TMUX window creation in the worktree directory
- Window naming with pattern `{app}-{type}/{name}`
- CD command copied to TMUX buffer for easy access from other panels
- Support for task types: feat, fix, chore, refactor
- Base branch option (`-s, --source`) for creating worktrees from specific branches

### Requirements
- TMUX (must be running)
- Branchlet (git worktree manager)
- Git repository
- Branchlet config at `~/.branchlet/settings.json`
