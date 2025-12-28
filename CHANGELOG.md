# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.1] - 2025-12-28

### Features

- Add version bump script with changelog generation (`0b933c1`)
- Add full HTML web interface for agent management (`45d5946`)
- Add logging verbosity flags and debug utilities (`861af8a`)
- Add step output persistence for agent workflow data flow (`bec2840`)
- Add Rust workspace with agent network system (`47b9ad0`)
- Add auto-approve, auto-merge, and merge conflict resolution (`f682450`)
- Add status command for viewing UI from any terminal (`fe0ee7b`)
- Add concurrent PR processing and ASCII UI dashboard (`a879990`)
- Add pr set command to watch existing PRs (`e2e297d`)

### Bug Fixes

- Address security and code quality issues in step output persistence (`41e16f5`)
- Handle stale worktrees in shepherd command (`2a3bdb1`)

### Other Changes

- Initial commit: Simple multi-agent orchestrator (`8525642`)
