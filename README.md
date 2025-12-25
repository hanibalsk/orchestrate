# Simple Multi-Agent Orchestrator

Minimal orchestrator for Claude Code agents with:
- **Git worktree isolation** - parallel development
- **PR queue** - only ONE PR open at a time

## Installation

```bash
./install.sh install           # Install globally
./install.sh install-local .   # Install to project
./install.sh uninstall         # Remove (auto-backup)
```

## Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  Work in parallel worktrees                                 │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐                  │
│  │ feature-1│  │ feature-2│  │ feature-3│                  │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘                  │
│       │             │             │                         │
│       ▼             ▼             ▼                         │
│  ┌─────────────────────────────────────┐                   │
│  │           PR Queue                   │                   │
│  │  1. feature-1  2. feature-2  3. ... │                   │
│  └──────────────────┬──────────────────┘                   │
│                     │                                       │
│                     ▼                                       │
│  ┌─────────────────────────────────────┐                   │
│  │     Only ONE PR open at a time      │                   │
│  │         PR #123 (feature-1)         │                   │
│  └──────────────────┬──────────────────┘                   │
│                     │ merged/closed                         │
│                     ▼                                       │
│              Next PR created                                │
└─────────────────────────────────────────────────────────────┘
```

## Quick Start

```bash
# 1. Develop in isolated worktree
orchestrate develop "Add user auth" auth-feature

# 2. Agent works, commits changes...

# 3. Mark done - queues for PR (or creates if none open)
orchestrate done auth-feature "User authentication"

# 4. Watch the PR
orchestrate shepherd 123

# 5. When PR merges, next in queue auto-creates PR
orchestrate pr status
```

## Commands

### PR Queue (one at a time)

| Command | Description |
|---------|-------------|
| `pr queue` | Show queued finished work |
| `pr status` | Check current PR, process queue if closed |
| `pr create` | Create PR from next in queue |
| `done <wt> [title]` | Mark worktree done, queue for PR |

### Development

| Command | Description |
|---------|-------------|
| `develop <task> [wt]` | Implement in worktree |
| `bmad [epic] [wt]` | BMAD workflow |
| `story <id> [wt]` | Implement story |
| `shepherd <pr>` | Watch PR, auto-fix |
| `review <target>` | Code review |
| `parallel <t1> <t2>` | Parallel agents |

### Worktrees

| Command | Description |
|---------|-------------|
| `wt create <name>` | Create worktree |
| `wt list` | List worktrees |
| `wt remove <name>` | Remove worktree |

## Available Agents

| Agent | Purpose |
|-------|---------|
| `bmad-orchestrator` | Run BMAD workflow |
| `bmad-planner` | Create epics/stories |
| `story-developer` | Implement features (TDD) |
| `pr-shepherd` | Watch PRs, auto-fix |
| `code-reviewer` | Code quality review |
| `issue-fixer` | Fix CI/test failures |
| `explorer` | Fast codebase search |

## Project Structure

```
.
├── install.sh            # Installer
├── orchestrate           # Main CLI
├── README.md
├── .orchestrate/         # State (queue, current PR)
├── .worktrees/           # Isolated worktrees
└── .claude/agents/       # Agent definitions
```

## Requirements

- [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code)
- `gh` CLI for PR operations
- Git with worktree support
