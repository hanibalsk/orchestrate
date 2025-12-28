# Design Requirements for Greenfield Orchestrator

## Core Requirements

### 1. Rust-Based Controller

**Must Have:**
- Single binary executable
- Background daemon mode
- CLI interface for control
- Async runtime (tokio)
- Cross-platform support (macOS, Linux)

**Architecture:**
```
orchestrate daemon start   → Starts background process
orchestrate daemon stop    → Graceful shutdown
orchestrate daemon status  → Show running state
orchestrate <command>      → Communicates with daemon via IPC
```

### 2. SQLite Database for State

**Must Have:**
- Embedded database (no external dependencies)
- ACID transactions for state changes
- Migration system for schema evolution
- Full audit trail of agent actions

**Core Tables:**
```sql
agents          -- Agent instances and state
agent_messages  -- Full message history
sessions        -- Claude API sessions
pr_queue        -- PR lifecycle tracking
worktrees       -- Git worktree management
epics           -- BMAD epic tracking
stories         -- BMAD story tracking
```

### 3. Agent State Machines

**States:**
```
Created → Initializing → Running ←→ WaitingForInput
                ↓              ↓
          WaitingForExternal   Paused
                ↓              ↓
           Completed ←─────────┘
                ↓
              Failed → Terminated
```

**Lifecycle Events:**
- Spawn: Create agent with task and context
- Start: Initialize and begin loop
- Pause: Suspend execution (preservable)
- Resume: Continue from saved state
- Complete: Task finished successfully
- Fail: Unrecoverable error
- Terminate: Force stop

### 4. Claude Agent SDK Integration

**Must Have:**
- Loop functionality with turn limits
- Tool execution framework
- Session management
- Token tracking per agent

**Loop Pattern:**
```rust
loop {
    let response = agent.step().await?;

    for tool_call in response.tool_calls {
        let result = execute_tool(tool_call).await?;
        agent.submit_tool_result(result).await?;
    }

    if is_complete(&response) { break; }
    if needs_external_wait(&response) { pause(); break; }
}
```

### 5. Session Forking for Token Optimization

**Must Have:**
- Fork session from parent agent
- Child inherits conversation context
- Token savings tracked
- Session cleanup on completion

**Pattern:**
```rust
// Parent agent completes phase 1
let parent = agent.complete_phase1().await?;

// Fork for phase 2 - inherits context
let child = fork_session(&parent, "Continue with phase 2").await?;
// Child has full context without re-sending history
```

### 6. BMAD Workflow Support

**Must Have:**
- Epic parsing from `_bmad-output/epics*.md`
- Story extraction and tracking
- Workflow phase progression
- Integration with existing BMAD commands

**Workflow:**
```
FIND_EPIC → CREATE_BRANCH → DEVELOP_STORIES → CODE_REVIEW →
CREATE_PR → (add to pending) → FIND_EPIC (next)
```

### 7. PR Controller

**Must Have:**
- One-PR-at-a-time queue (optional parallel)
- Merge strategies (squash, rebase, merge)
- Conflict resolution
- Auto-merge when healthy
- Thread resolution via GraphQL

**Merge Strategies:**
```rust
enum MergeStrategy {
    Squash,       // Default: squash commits
    Rebase,       // Rebase onto main
    Merge,        // Merge commit
    FastForward,  // Fast-forward if possible
}
```

**Conflict Resolution:**
```rust
enum ConflictStrategy {
    RebaseAuto,   // Try git rebase
    SpawnAgent,   // Spawn agent to resolve
    Block,        // Mark as blocked, notify
}
```

### 8. Worktree Management

**Must Have:**
- Create worktrees for parallel development
- Track worktree-to-agent associations
- Cleanup stale worktrees
- On-demand creation (not preemptive)

### 9. Web Interface

**Must Have:**
- Real-time agent status
- Chat with individual agents
- Switch between agents
- Message history view
- WebSocket for live updates

**Optional:**
- Dashboard with all agents
- PR queue visualization
- Token usage graphs

### 10. Entry Points

**1. Claude Code Skill/Command:**
```bash
# Spawns controller as background agent
claude -p "spawn orchestrator-controller..."
```

**2. Direct CLI:**
```bash
orchestrate daemon start
orchestrate agent spawn --type story-developer --task "..."
```

**3. Web UI:**
```bash
orchestrate web --port 8080
# Opens browser to http://localhost:8080
```

---

## Agent Types

### Development Agents

| Type | Purpose | Tools | Model |
|------|---------|-------|-------|
| `story-developer` | Implement features (TDD) | Bash,Read,Write,Edit,Glob,Grep | Sonnet |
| `code-reviewer` | Read-only analysis | Bash,Read,Glob,Grep | Sonnet |
| `issue-fixer` | Fix CI/test failures | Bash,Read,Write,Edit,Glob,Grep | Sonnet |
| `explorer` | Fast codebase search | Read,Glob,Grep | Haiku |

### BMAD Agents

| Type | Purpose | Tools | Model |
|------|---------|-------|-------|
| `bmad-orchestrator` | Orchestrate epic workflow | Bash,Read,Write,Edit,Glob,Grep,Task | Sonnet |
| `bmad-planner` | Create epics/stories | Bash,Read,Write,Edit,Glob,Grep | Sonnet |

### System Agents

| Type | Purpose | Tools | Model |
|------|---------|-------|-------|
| `pr-shepherd` | Watch PRs, fix issues | Bash,Read,Write,Edit,Glob,Grep,Task | Sonnet |
| `pr-controller` | Manage PR queue | Bash,Read | Sonnet |
| `background-controller` | Main orchestration | All | Sonnet |
| `conflict-resolver` | Resolve merge conflicts | Bash,Read,Write,Edit | Sonnet |

---

## API Design

### CLI Commands

```bash
# Daemon
orchestrate daemon start [--port PORT]
orchestrate daemon stop
orchestrate daemon status

# Agents
orchestrate agent spawn --type TYPE --task TASK [--worktree NAME]
orchestrate agent list [--state STATE]
orchestrate agent show ID
orchestrate agent pause ID
orchestrate agent resume ID
orchestrate agent terminate ID
orchestrate agent chat ID

# PRs
orchestrate pr list [--status STATUS]
orchestrate pr create [--worktree NAME] [--title TITLE]
orchestrate pr merge NUMBER [--strategy STRATEGY]
orchestrate pr queue

# Worktrees
orchestrate wt create NAME [--base BRANCH]
orchestrate wt list
orchestrate wt remove NAME

# BMAD
orchestrate bmad process [PATTERN]
orchestrate bmad status
orchestrate bmad reset

# Web
orchestrate web [--port PORT]

# Status
orchestrate status [--json]
```

### REST API

```
# Agents
GET    /api/agents                    List agents
POST   /api/agents                    Spawn agent
GET    /api/agents/:id                Get agent
DELETE /api/agents/:id                Terminate agent
POST   /api/agents/:id/pause          Pause agent
POST   /api/agents/:id/resume         Resume agent
GET    /api/agents/:id/messages       Get messages
POST   /api/agents/:id/messages       Send message
WS     /api/agents/:id/chat           WebSocket chat

# PRs
GET    /api/prs                       List PRs
POST   /api/prs                       Create PR
GET    /api/prs/:number               Get PR details
POST   /api/prs/:number/merge         Merge PR

# Worktrees
GET    /api/worktrees                 List worktrees
POST   /api/worktrees                 Create worktree
DELETE /api/worktrees/:name           Remove worktree

# System
GET    /api/status                    System status
GET    /api/epics                     List epics
GET    /api/epics/:id                 Get epic
```

### WebSocket Messages

```typescript
// Server → Client
{ type: "agent_state", agent_id: string, state: AgentState }
{ type: "agent_message", agent_id: string, message: Message }
{ type: "pr_update", pr_number: number, status: PrStatus }
{ type: "system_status", data: SystemStatus }

// Client → Server
{ type: "send_message", agent_id: string, content: string }
{ type: "subscribe", channels: string[] }
{ type: "unsubscribe", channels: string[] }
```

---

## Database Schema

```sql
-- Agents
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'created',
    task TEXT NOT NULL,
    context JSON DEFAULT '{}',
    session_id TEXT REFERENCES sessions(id),
    parent_agent_id TEXT REFERENCES agents(id),
    worktree_id TEXT REFERENCES worktrees(id),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Messages
CREATE TABLE agent_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL REFERENCES agents(id),
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    tool_calls JSON,
    tool_results JSON,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Sessions
CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id),
    parent_id TEXT REFERENCES sessions(id),
    api_session_id TEXT,
    total_tokens INTEGER DEFAULT 0,
    is_forked BOOLEAN DEFAULT FALSE,
    forked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    closed_at TEXT
);

-- PR Queue
CREATE TABLE pr_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    epic_id TEXT,
    worktree_id TEXT REFERENCES worktrees(id),
    branch_name TEXT NOT NULL,
    title TEXT,
    body TEXT,
    pr_number INTEGER,
    status TEXT NOT NULL DEFAULT 'queued'
        CHECK (status IN ('queued', 'creating', 'open', 'reviewing', 'fixing', 'merging', 'merged', 'failed', 'closed')),
    merge_strategy TEXT DEFAULT 'squash',
    agent_id TEXT REFERENCES agents(id),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    merged_at TEXT
);

-- Worktrees
CREATE TABLE worktrees (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    base_branch TEXT NOT NULL DEFAULT 'main',
    status TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'stale', 'removed')),
    agent_id TEXT REFERENCES agents(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    removed_at TEXT
);

-- Epics (BMAD)
CREATE TABLE epics (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    source_file TEXT,
    pattern TEXT,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'in_progress', 'completed', 'blocked', 'skipped')),
    current_phase TEXT,
    agent_id TEXT REFERENCES agents(id),
    pr_id INTEGER REFERENCES pr_queue(id),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Stories (BMAD)
CREATE TABLE stories (
    id TEXT PRIMARY KEY,
    epic_id TEXT NOT NULL REFERENCES epics(id),
    title TEXT NOT NULL,
    description TEXT,
    acceptance_criteria JSON,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'in_progress', 'completed', 'blocked', 'skipped')),
    agent_id TEXT REFERENCES agents(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Audit Log
CREATE TABLE audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,
    old_value JSON,
    new_value JSON,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes
CREATE INDEX idx_agents_state ON agents(state);
CREATE INDEX idx_agents_type ON agents(agent_type);
CREATE INDEX idx_messages_agent ON agent_messages(agent_id);
CREATE INDEX idx_sessions_agent ON sessions(agent_id);
CREATE INDEX idx_pr_queue_status ON pr_queue(status);
CREATE INDEX idx_epics_status ON epics(status);
CREATE INDEX idx_stories_epic ON stories(epic_id);
CREATE INDEX idx_audit_entity ON audit_log(entity_type, entity_id);
```

---

## Configuration

### Environment Variables

```bash
# Required
ANTHROPIC_API_KEY=sk-ant-...

# Optional
ORCHESTRATE_DB_PATH=~/.orchestrate/orchestrate.db
ORCHESTRATE_LOG_LEVEL=info
ORCHESTRATE_DAEMON_PORT=9999
ORCHESTRATE_WEB_PORT=8080
ORCHESTRATE_WORKTREE_DIR=.worktrees

# GitHub
GITHUB_TOKEN=ghp_...

# BMAD
BMAD_EPICS_DIR=_bmad-output
BMAD_DEFAULT_BASE_BRANCH=main

# Agent defaults
DEFAULT_MAX_TURNS=80
DEFAULT_CHECK_INTERVAL=30
MAX_CONCURRENT_AGENTS=5
MAX_PENDING_PRS=3
```

### Config File

```toml
# ~/.orchestrate/config.toml

[daemon]
port = 9999
log_level = "info"

[web]
port = 8080
host = "127.0.0.1"

[database]
path = "~/.orchestrate/orchestrate.db"
backup_interval = "1h"

[agents]
max_concurrent = 5
default_max_turns = 80
default_model = "sonnet"

[prs]
max_pending = 3
merge_strategy = "squash"
auto_merge = true

[worktrees]
directory = ".worktrees"
auto_cleanup = true

[bmad]
epics_directory = "_bmad-output"
base_branch = "main"
```
