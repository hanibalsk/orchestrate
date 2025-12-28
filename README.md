# Orchestrate - Multi-Agent Orchestrator

A powerful multi-agent orchestrator for Claude Code with self-learning capabilities.

## Key Features

- **Git worktree isolation** - Parallel development in isolated worktrees
- **PR queue** - Only ONE PR open at a time for clean merges
- **Self-learning instructions** - Automatically learns from agent failures
- **Web dashboard** - Full HTML interface for agent management
- **Rust-based core** - High-performance database and API

## Installation

```bash
# Global installation (shell-based orchestrator)
./install.sh install

# Local project installation
./install.sh install-local .

# Build and install Rust binary
./install.sh install-rust

# Uninstall
./install.sh uninstall
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

### Shell Orchestrator

#### PR Queue (one at a time)

| Command | Description |
|---------|-------------|
| `pr queue` | Show queued finished work |
| `pr status` | Check current PR, process queue if closed |
| `pr create` | Create PR from next in queue |
| `done <wt> [title]` | Mark worktree done, queue for PR |

#### Development

| Command | Description |
|---------|-------------|
| `develop <task> [wt]` | Implement in worktree |
| `bmad [epic] [wt]` | BMAD workflow |
| `story <id> [wt]` | Implement story |
| `shepherd <pr>` | Watch PR, auto-fix |
| `review <target>` | Code review |
| `parallel <t1> <t2>` | Parallel agents |

#### Worktrees

| Command | Description |
|---------|-------------|
| `wt create <name>` | Create worktree |
| `wt list` | List worktrees |
| `wt remove <name>` | Remove worktree |

### Rust CLI (orchestrate-rs)

#### Agent Management

| Command | Description |
|---------|-------------|
| `agent spawn -t <type> -T <task>` | Spawn a new agent |
| `agent list` | List all agents |
| `agent show <id>` | Show agent details |
| `agent pause <id>` | Pause an agent |
| `agent resume <id>` | Resume an agent |
| `agent terminate <id>` | Terminate an agent |

#### Custom Instructions

| Command | Description |
|---------|-------------|
| `instructions list` | List all instructions |
| `instructions show <id>` | Show instruction details |
| `instructions create -n <name> -c <content>` | Create new instruction |
| `instructions enable <id>` | Enable an instruction |
| `instructions disable <id>` | Disable an instruction |
| `instructions delete <id>` | Delete an instruction |
| `instructions stats` | Show effectiveness stats |

#### Learning System

| Command | Description |
|---------|-------------|
| `learn patterns` | List detected patterns |
| `learn approve <id>` | Approve pattern → create instruction |
| `learn reject <id>` | Reject a pattern |
| `learn analyze` | Process patterns and create instructions |
| `learn cleanup` | Remove ineffective instructions |
| `learn config` | Show learning configuration |

#### Web Interface

| Command | Description |
|---------|-------------|
| `web -p 8080` | Start web server |
| `status` | Show system status |

## Self-Learning Instructions

The system automatically learns from agent failures:

1. **Pattern Detection** - Analyzes failed agent runs for recurring errors
2. **Instruction Generation** - Creates instructions to prevent future issues
3. **Effectiveness Tracking** - Monitors success/failure rates
4. **Penalty Scoring** - Auto-disables ineffective instructions

### Penalty Thresholds

| Penalty Score | Action |
|---------------|--------|
| 0.5 | Warning logged |
| 0.7 | Instruction auto-disabled |
| 1.0+ | Eligible for deletion (learned only) |

### CLI Examples

```bash
# Create a global instruction
orchestrate instructions create \
  --name "no-force-push" \
  --content "Never use git push --force without explicit user approval"

# Create instruction for specific agent type
orchestrate instructions create \
  --name "security-check" \
  --content "Always check for SQL injection vulnerabilities" \
  --scope agent_type \
  --agent-type code_reviewer

# View instruction effectiveness
orchestrate instructions stats

# Process learned patterns
orchestrate learn analyze

# Cleanup ineffective instructions
orchestrate learn cleanup
```

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
| `pr-controller` | Manage PR lifecycle |
| `conflict-resolver` | Resolve merge conflicts |

## Project Structure

```
.
├── install.sh              # Shell installer
├── orchestrate             # Shell CLI
├── README.md
├── CHANGELOG.md
├── Cargo.toml              # Rust workspace
├── crates/
│   ├── orchestrate-core/   # Core types, database, learning
│   ├── orchestrate-claude/ # Claude API client
│   ├── orchestrate-web/    # Web interface
│   ├── orchestrate-github/ # GitHub integration
│   └── orchestrate-cli/    # Rust CLI
├── migrations/             # Database migrations
├── .orchestrate/           # State (queue, current PR)
├── .worktrees/             # Isolated worktrees
└── .claude/agents/         # Agent definitions
```

## REST API

When running the web server (`orchestrate web`):

### Agent Endpoints
- `GET /api/agents` - List agents
- `POST /api/agents` - Create agent
- `GET /api/agents/:id` - Get agent details
- `POST /api/agents/:id/pause` - Pause agent
- `POST /api/agents/:id/resume` - Resume agent
- `POST /api/agents/:id/terminate` - Terminate agent

### Instruction Endpoints
- `GET /api/instructions` - List instructions
- `POST /api/instructions` - Create instruction
- `GET /api/instructions/:id` - Get instruction
- `PUT /api/instructions/:id` - Update instruction
- `DELETE /api/instructions/:id` - Delete instruction
- `POST /api/instructions/:id/enable` - Enable
- `POST /api/instructions/:id/disable` - Disable
- `GET /api/instructions/:id/effectiveness` - Get metrics

### Learning Endpoints
- `GET /api/patterns` - List patterns
- `POST /api/patterns/:id/approve` - Approve pattern
- `POST /api/patterns/:id/reject` - Reject pattern
- `POST /api/learning/process` - Process patterns
- `POST /api/learning/cleanup` - Cleanup ineffective

## Requirements

- [Claude Code CLI](https://docs.anthropic.com/en/docs/claude-code)
- `gh` CLI for PR operations
- Git with worktree support
- Rust (for building the Rust CLI)
