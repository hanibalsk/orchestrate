# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2025-12-28

### Features

- **Enhanced agent definitions** (`d9fd7fd`)
  - Comprehensive system prompts for all agent types
  - TDD workflow, security checklists, and status signals
  - New `scheduler` agent for coordination

- **New skill definitions** (`d9fd7fd`)
  - `develop` - Start development in isolated worktree
  - `review` - Code review with multiple targets
  - `parallel` - Run multiple agents concurrently
  - `spawn` - Direct agent spawning
  - `done` - Mark worktree complete
  - `status` - View system dashboard

- **Task tool for sub-agent spawning** (`de364a7`)
  - Spawn sub-agents via Task tool in tools.rs
  - Maps subagent_type to AgentType enum
  - Returns spawn request for orchestrator execution

- **Agent prompt loading from files** (`de364a7`)
  - Load prompts from `.claude/agents/*.md`
  - Parse frontmatter and extract content
  - Fallback to default prompts

- **Shell state bridge** (`de364a7`)
  - New `shell_state.rs` module for shell interoperability
  - Read/write PR queue and current PR
  - Shepherd lock management and cleanup
  - CLI shows combined shell + database state

### CLI

- `pr queue` shows shell queue + database PRs
- `status` displays ASCII dashboard with all components
- JSON output includes queue_size, current_pr, active_shepherds

## [0.2.0] - 2025-12-28

### Features

- **Self-learning custom instructions system** (`5ce3b04`)
  - Automatic pattern detection from agent failures (errors, tool usage, behaviors)
  - Instruction generation with configurable confidence thresholds
  - Penalty scoring system with auto-disable and cleanup
  - Instructions injected into agent system prompts at runtime
  - Support for global and per-agent-type instruction scopes

- **REST API for instruction management**
  - CRUD endpoints for instructions and patterns
  - Effectiveness metrics endpoint
  - Pattern approval/rejection endpoints
  - Learning process and cleanup triggers

- **CLI commands for instructions and learning**
  - `instructions list|show|create|enable|disable|delete|stats`
  - `learn patterns|approve|reject|analyze|config|cleanup|reset-penalty`

### Database

- New migration: `004_custom_instructions.sql`
  - `custom_instructions` table with scope, priority, and confidence
  - `instruction_usage` for tracking when instructions are applied
  - `instruction_effectiveness` with penalty scoring
  - `learning_patterns` for storing detected patterns

### Testing

- 10 new integration tests for the instruction system
- Tests for CRUD, scoping, usage tracking, penalty system, and learning workflow

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
