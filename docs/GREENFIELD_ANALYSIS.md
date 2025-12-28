# Greenfield Orchestrator Analysis

## Executive Summary

This document provides a deep analysis of both **orchestrate** (Bash) and **autopilot** (Bash) projects to inform a complete greenfield rebuild in **Rust**. The new system will be a stateful agent orchestrator with:

- **Rust-based controller** running as a background daemon or spawned by Claude Code
- **SQLite database** for persistent agent state machines with lifecycle management
- **Claude Agent SDK integration** with looping functionality
- **Session forking** for token optimization
- **BMAD workflow support** (bmad-autopilot)
- **Parallel development** using git worktrees
- **PR controller** for merge strategies and conflict resolution
- **Web-based chat interface** for agent interaction

---

## Part 1: Existing Project Analysis

### 1.1 Orchestrate Project (Current Repository)

**Location:** `/Users/martinjanci/projects/github.com/hanibalsk/orchestrate`
**Technology:** Pure Bash (~1,300 lines)
**Purpose:** Multi-agent orchestrator for Claude Code with PR queue

#### Core Concepts

| Concept | Implementation | Notes |
|---------|----------------|-------|
| **PR Queue** | File-based FIFO (`.orchestrate/pr-queue`) | One PR at a time |
| **State** | Plain files (`current-pr`, `shepherd-*.lock`) | No database |
| **Agents** | 7 Claude Code agent specs (`.claude/agents/*.md`) | Spawned via `claude -p` |
| **Worktrees** | Git worktrees in `.worktrees/` | Parallel development |
| **Loop** | `orchestrate loop` - continuous automation | ASCII UI dashboard |

#### Agent Definitions

```
bmad-orchestrator.md  → Orchestrate BMAD epics/stories
bmad-planner.md       → Create epics/stories in docs/bmad/
story-developer.md    → Implement features (TDD workflow)
pr-shepherd.md        → Watch PRs, fix issues, resolve threads
code-reviewer.md      → Read-only code analysis
issue-fixer.md        → Fix CI/test failures
explorer.md           → Fast codebase search (uses Haiku)
```

#### Key Commands

```bash
orchestrate develop <task>      # Spawn story-developer
orchestrate bmad [epic]         # Spawn bmad-orchestrator
orchestrate shepherd <pr>       # Spawn pr-shepherd
orchestrate loop                # Continuous automation
orchestrate pr queue/status     # Manage PR queue
orchestrate wt create/remove    # Worktree management
```

#### Strengths

1. Simple file-based state (easy to debug)
2. One-PR-at-a-time ensures quality
3. Concurrent shepherds (up to 3)
4. ASCII UI for monitoring
5. Auto-merge when healthy

#### Weaknesses

1. No database - state is fragile
2. No session management - each agent starts fresh
3. No web interface
4. Limited scalability
5. No token optimization

---

### 1.2 Autopilot Project

**Location:** `/Users/martinjanci/projects/github.com/hanibalsk/autopilot`
**Technology:** Pure Bash (~1,877 lines)
**Purpose:** State machine for BMAD epic development cycle

#### State Machine Phases

```
CHECK_PENDING_PR → FIND_EPIC → CREATE_BRANCH → DEVELOP_STORIES →
CODE_REVIEW → CREATE_PR → WAIT_COPILOT → (add to pending) →
                                ↓              ↓
                           FIX_ISSUES ←────────┘
                                ↓
                           WAIT_COPILOT → FIND_EPIC (next)
```

#### State Persistence

```json
{
  "mode": "parallel",
  "active_epic": "7A",
  "active_phase": "DEVELOP_STORIES",
  "active_worktree": "/path/to/worktree",
  "pending_prs": [
    {
      "epic": "6B",
      "pr_number": 123,
      "worktree": "/path",
      "status": "WAIT_REVIEW",
      "last_check": "2024-01-15T10:30:00Z"
    }
  ],
  "completed_epics": ["1A", "2A", "3B"]
}
```

#### Key Features

1. **Parallel Mode**: Work on next epic while PRs wait for review
2. **Worktree Management**: Isolated development per epic
3. **Copilot Integration**: Wait for reviews, fix issues, resolve threads
4. **Auto-Continue**: Never block on approval, continue to next epic
5. **GraphQL for Threads**: Fetch and resolve review threads
6. **Claude Interactive Mode**: Foreground development sessions

#### Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `PARALLEL_MODE` | 0 | Enable parallel epic development |
| `MAX_PENDING_PRS` | 2 | Max concurrent PRs |
| `MAX_TURNS` | 80 | Claude turns per phase |
| `CHECK_INTERVAL` | 30 | Seconds between polls |

#### Claude Integration Patterns

**Interactive (foreground):**
```bash
claude -p "$prompt" --permission-mode acceptEdits 2>&1 | tee "$output_file"
```

**Headless (background):**
```bash
claude -p "$prompt" \
  --permission-mode acceptEdits \
  --allowedTools "Bash,Read,Write,Edit,Grep" \
  --max-turns 30
```

#### Strengths

1. True state machine with JSON persistence
2. Parallel epic development
3. GraphQL for thread resolution
4. Auto-approve workflow integration
5. Copilot-aware with stale approval handling

#### Weaknesses

1. Still Bash (complex string handling)
2. No real database
3. No session forking/reuse
4. Single-threaded (no true parallelism)
5. No web interface

---

## Part 2: New Greenfield Architecture

### 2.1 System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         ORCHESTRATOR SYSTEM                                  │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   ┌─────────────┐    ┌─────────────────┐    ┌─────────────────────────────┐ │
│   │ Claude Code │───►│ Skill/Command   │───►│ Background Controller       │ │
│   │ (Terminal)  │    │ (spawns daemon) │    │ (Rust binary)               │ │
│   └─────────────┘    └─────────────────┘    │                             │ │
│                                              │  ┌─────────────────────────┐│ │
│   ┌─────────────┐                           │  │ Agent Manager           ││ │
│   │ Web UI      │◄──────────────────────────┤  │ - State machines        ││ │
│   │ (Chat)      │    WebSocket              │  │ - Lifecycle control     ││ │
│   └─────────────┘                           │  │ - Session management    ││ │
│                                              │  └─────────────────────────┘│ │
│   ┌─────────────┐                           │                             │ │
│   │ Direct Run  │───────────────────────────┤  ┌─────────────────────────┐│ │
│   │ (CLI)       │                           │  │ Claude Agent SDK        ││ │
│   └─────────────┘                           │  │ - Loop functionality    ││ │
│                                              │  │ - Session forking       ││ │
│                                              │  │ - Token optimization    ││ │
│                                              │  └─────────────────────────┘│ │
│                                              │                             │ │
│                                              │  ┌─────────────────────────┐│ │
│                                              │  │ SQLite Database         ││ │
│                                              │  │ - Agent states          ││ │
│                                              │  │ - Messages/history      ││ │
│                                              │  │ - Session metadata      ││ │
│                                              │  └─────────────────────────┘│ │
│                                              │                             │ │
│                                              │  ┌─────────────────────────┐│ │
│                                              │  │ PR Controller           ││ │
│                                              │  │ - Merge strategies      ││ │
│                                              │  │ - Conflict resolution   ││ │
│                                              │  │ - Worktree management   ││ │
│                                              │  └─────────────────────────┘│ │
│                                              └─────────────────────────────┘ │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2.2 Entry Points

#### 1. AI-Spawned (via Claude Code)

```bash
# Claude Code spawns the controller as a background agent
claude -p "Use the Task tool with subagent_type='orchestrator-controller' to start..."
```

The skill/command triggers:
```bash
# Start the Rust daemon if not running
orchestrate daemon start
# Send command via IPC
orchestrate agent spawn --type story-developer --task "implement auth"
```

#### 2. Direct CLI

```bash
# Start daemon
orchestrate daemon start

# Spawn agent
orchestrate agent spawn --type story-developer --task "implement auth"

# Check status
orchestrate status

# Web interface
orchestrate web --port 8080
```

#### 3. Web Interface

- Chat with individual agents
- Switch between active agents
- View agent state and history
- Real-time updates via WebSocket

### 2.3 Agent State Machine

Each agent is a state machine stored in SQLite:

```rust
pub enum AgentState {
    Created,
    Initializing,
    Running,
    WaitingForInput,
    WaitingForExternal, // PR review, CI, etc.
    Paused,
    Completed,
    Failed,
    Terminated,
}

pub struct Agent {
    id: Uuid,
    agent_type: AgentType,
    state: AgentState,
    task: String,
    context: AgentContext,
    session_id: Option<String>,
    parent_session_id: Option<String>, // For forked sessions
    worktree_path: Option<PathBuf>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

pub struct AgentContext {
    epic_id: Option<String>,
    story_id: Option<String>,
    pr_number: Option<i32>,
    branch_name: Option<String>,
    custom: serde_json::Value,
}
```

#### State Transitions

```
Created → Initializing → Running ←→ WaitingForInput
                ↓              ↓
          WaitingForExternal   Paused
                ↓              ↓
           Completed ←─────────┘
                ↓
              Failed
                ↓
           Terminated
```

### 2.4 SQLite Database Schema

```sql
-- Core tables
CREATE TABLE agents (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL,
    state TEXT NOT NULL,
    task TEXT NOT NULL,
    context JSON,
    session_id TEXT,
    parent_session_id TEXT,
    worktree_path TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE agent_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL REFERENCES agents(id),
    role TEXT NOT NULL, -- 'user', 'assistant', 'system'
    content TEXT NOT NULL,
    tool_calls JSON,
    tool_results JSON,
    tokens_used INTEGER,
    created_at TEXT NOT NULL
);

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,
    agent_id TEXT REFERENCES agents(id),
    parent_id TEXT REFERENCES sessions(id),
    api_session_id TEXT, -- Claude API session ID
    context_tokens INTEGER DEFAULT 0,
    forked_at TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE pr_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    epic_id TEXT,
    worktree_name TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    title TEXT,
    pr_number INTEGER,
    status TEXT NOT NULL, -- 'queued', 'creating', 'open', 'reviewing', 'merging', 'merged', 'failed'
    agent_id TEXT REFERENCES agents(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE worktrees (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    agent_id TEXT REFERENCES agents(id),
    status TEXT NOT NULL, -- 'active', 'stale', 'removed'
    created_at TEXT NOT NULL
);

-- BMAD-specific tables
CREATE TABLE epics (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    source_file TEXT,
    status TEXT NOT NULL, -- 'pending', 'in_progress', 'completed', 'blocked'
    agent_id TEXT REFERENCES agents(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE stories (
    id TEXT PRIMARY KEY,
    epic_id TEXT NOT NULL REFERENCES epics(id),
    title TEXT NOT NULL,
    acceptance_criteria JSON,
    status TEXT NOT NULL,
    agent_id TEXT REFERENCES agents(id),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Indexes
CREATE INDEX idx_agents_state ON agents(state);
CREATE INDEX idx_agents_type ON agents(agent_type);
CREATE INDEX idx_messages_agent ON agent_messages(agent_id);
CREATE INDEX idx_sessions_agent ON sessions(agent_id);
CREATE INDEX idx_pr_queue_status ON pr_queue(status);
```

### 2.5 Claude Agent SDK Integration

#### Looping Pattern

```rust
use claude_agent_sdk::{Agent, Message, Tool, LoopConfig};

pub async fn run_agent_loop(
    agent: &mut Agent,
    db: &Database,
    config: LoopConfig,
) -> Result<()> {
    let sdk_agent = claude_agent_sdk::Agent::new(config.api_key)
        .with_model(config.model)
        .with_tools(get_tools_for_agent(&agent.agent_type))
        .with_max_turns(config.max_turns);

    // Resume from session if exists
    if let Some(session_id) = &agent.session_id {
        sdk_agent.resume_session(session_id).await?;
    }

    loop {
        // Update agent state
        agent.state = AgentState::Running;
        db.update_agent(agent).await?;

        // Run one turn
        let response = sdk_agent.step().await?;

        // Store message in database
        db.insert_message(AgentMessage {
            agent_id: agent.id,
            role: "assistant".into(),
            content: response.content.clone(),
            tool_calls: response.tool_calls.clone(),
            tokens_used: response.usage.total_tokens,
            created_at: Utc::now(),
        }).await?;

        // Handle tool calls
        for tool_call in &response.tool_calls {
            let result = execute_tool(tool_call, agent, db).await?;
            sdk_agent.submit_tool_result(tool_call.id, result).await?;
        }

        // Check loop termination conditions
        if response.stop_reason == StopReason::EndTurn {
            if is_agent_complete(&response) {
                agent.state = AgentState::Completed;
                break;
            }
        }

        // Check for external waits (PR review, CI, etc.)
        if needs_external_wait(&response) {
            agent.state = AgentState::WaitingForExternal;
            db.update_agent(agent).await?;
            // Will be resumed by PR controller or scheduler
            break;
        }

        // Rate limiting
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    db.update_agent(agent).await?;
    Ok(())
}
```

#### Session Forking for Token Optimization

```rust
pub async fn fork_session(
    parent_agent: &Agent,
    new_task: &str,
    db: &Database,
) -> Result<Agent> {
    // Create new agent with parent's session
    let child_agent = Agent {
        id: Uuid::new_v4(),
        agent_type: parent_agent.agent_type.clone(),
        state: AgentState::Created,
        task: new_task.to_string(),
        context: parent_agent.context.clone(),
        session_id: None,
        parent_session_id: parent_agent.session_id.clone(),
        ..Default::default()
    };

    // Fork the Claude API session
    let sdk_session = claude_agent_sdk::Session::fork(
        parent_agent.session_id.as_ref().unwrap()
    ).await?;

    // Store new session
    db.insert_session(Session {
        id: sdk_session.id.clone(),
        agent_id: child_agent.id,
        parent_id: parent_agent.session_id.clone(),
        api_session_id: sdk_session.api_id.clone(),
        context_tokens: parent_agent.context_tokens(),
        forked_at: Some(Utc::now()),
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }).await?;

    // Update child with session
    let mut child = child_agent;
    child.session_id = Some(sdk_session.id);

    db.insert_agent(&child).await?;
    Ok(child)
}
```

### 2.6 Agent Types

```rust
pub enum AgentType {
    // Development agents
    StoryDeveloper,
    CodeReviewer,
    IssueFixer,
    Explorer,

    // BMAD agents
    BmadOrchestrator,
    BmadPlanner,

    // PR management
    PrShepherd,
    PrController,

    // System agents
    BackgroundController,
    Scheduler,
}

impl AgentType {
    pub fn allowed_tools(&self) -> Vec<Tool> {
        match self {
            AgentType::StoryDeveloper => vec![
                Tool::Bash, Tool::Read, Tool::Write, Tool::Edit,
                Tool::Glob, Tool::Grep, Tool::Task,
            ],
            AgentType::CodeReviewer => vec![
                Tool::Bash, Tool::Read, Tool::Glob, Tool::Grep,
            ],
            AgentType::Explorer => vec![
                Tool::Read, Tool::Glob, Tool::Grep,
            ],
            // ... other agent types
        }
    }

    pub fn model(&self) -> Model {
        match self {
            AgentType::Explorer => Model::Haiku,
            _ => Model::Sonnet,
        }
    }
}
```

### 2.7 PR Controller

```rust
pub struct PrController {
    db: Database,
    github: GithubClient,
    max_concurrent_prs: usize,
    merge_strategy: MergeStrategy,
}

pub enum MergeStrategy {
    Squash,
    Rebase,
    Merge,
}

impl PrController {
    /// Check all pending PRs and take action
    pub async fn check_pending_prs(&self) -> Result<()> {
        let pending = self.db.get_pending_prs().await?;

        for pr in pending {
            let status = self.check_pr_status(&pr).await?;

            match status {
                PrStatus::Approved => {
                    self.merge_pr(&pr).await?;
                }
                PrStatus::NeedsReview => {
                    // Continue waiting
                }
                PrStatus::NeedsFixes => {
                    self.spawn_fixer_agent(&pr).await?;
                }
                PrStatus::Conflicted => {
                    self.resolve_conflicts(&pr).await?;
                }
                PrStatus::Merged => {
                    self.cleanup_pr(&pr).await?;
                }
            }
        }

        Ok(())
    }

    /// Resolve merge conflicts
    pub async fn resolve_conflicts(&self, pr: &PrQueue) -> Result<()> {
        // Strategy 1: Rebase onto main
        let worktree = self.db.get_worktree(&pr.worktree_name).await?;

        let result = Command::new("git")
            .args(["rebase", "origin/main"])
            .current_dir(&worktree.path)
            .output()
            .await?;

        if !result.status.success() {
            // Strategy 2: Spawn agent to resolve conflicts
            let agent = self.spawn_conflict_resolver(&pr, &worktree).await?;
            return Ok(());
        }

        // Push resolved changes
        Command::new("git")
            .args(["push", "--force-with-lease"])
            .current_dir(&worktree.path)
            .output()
            .await?;

        Ok(())
    }
}
```

### 2.8 Web Interface

```rust
use axum::{Router, routing::{get, post}, Json, Extension};
use tokio::sync::broadcast;

pub async fn start_web_server(
    db: Database,
    tx: broadcast::Sender<WebSocketMessage>,
    port: u16,
) -> Result<()> {
    let app = Router::new()
        .route("/api/agents", get(list_agents))
        .route("/api/agents/:id", get(get_agent))
        .route("/api/agents/:id/messages", get(get_messages).post(send_message))
        .route("/api/agents/:id/chat", get(agent_chat_ws))
        .route("/api/prs", get(list_prs))
        .route("/api/status", get(system_status))
        .layer(Extension(db))
        .layer(Extension(tx));

    axum::Server::bind(&format!("0.0.0.0:{}", port).parse()?)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn agent_chat_ws(
    ws: WebSocketUpgrade,
    Extension(db): Extension<Database>,
    Path(agent_id): Path<Uuid>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_agent_chat(socket, db, agent_id))
}

async fn handle_agent_chat(
    socket: WebSocket,
    db: Database,
    agent_id: Uuid,
) {
    let (mut sender, mut receiver) = socket.split();

    // Load agent
    let agent = db.get_agent(agent_id).await.unwrap();

    // Load message history
    let messages = db.get_messages(agent_id).await.unwrap();
    for msg in messages {
        sender.send(Message::Text(serde_json::to_string(&msg).unwrap())).await.ok();
    }

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
        if let Ok(Message::Text(text)) = msg {
            // Insert user message
            let user_msg = AgentMessage {
                agent_id,
                role: "user".into(),
                content: text,
                ..Default::default()
            };
            db.insert_message(&user_msg).await.ok();

            // Resume agent with new input
            resume_agent_with_input(&agent, &text, &db).await.ok();
        }
    }
}
```

### 2.9 BMAD Workflow Integration

```rust
pub struct BmadWorkflow {
    db: Database,
    controller: AgentController,
}

impl BmadWorkflow {
    /// Process an epic through the full cycle
    pub async fn process_epic(&self, epic_id: &str) -> Result<()> {
        // 1. Find and parse epic
        let epic = self.find_epic(epic_id).await?;
        self.db.upsert_epic(&epic).await?;

        // 2. Create branch
        let branch = format!("feature/epic-{}", epic_id);
        self.create_branch(&branch).await?;

        // 3. Spawn story developer agent for each story
        for story in &epic.stories {
            let agent = self.controller.spawn_agent(AgentType::StoryDeveloper)
                .with_task(format!("Implement story: {}", story.title))
                .with_context(json!({
                    "epic_id": epic_id,
                    "story_id": story.id,
                    "acceptance_criteria": story.acceptance_criteria,
                }))
                .with_worktree(&branch)
                .run()
                .await?;

            // Wait for story completion
            self.controller.wait_for_agent(&agent.id).await?;
        }

        // 4. Code review
        let reviewer = self.controller.spawn_agent(AgentType::CodeReviewer)
            .with_task(format!("Review epic {} implementation", epic_id))
            .fork_session(&agent) // Reuse context!
            .run()
            .await?;

        self.controller.wait_for_agent(&reviewer.id).await?;

        // 5. Create PR
        let pr = self.create_pr(&epic, &branch).await?;
        self.db.insert_pr(&pr).await?;

        // 6. Continue to next epic (non-blocking)
        Ok(())
    }
}
```

---

## Part 3: Implementation Roadmap

### Phase 1: Core Infrastructure
1. Rust project setup (Cargo workspace)
2. SQLite database layer with migrations
3. Agent state machine implementation
4. Basic CLI for daemon control

### Phase 2: Agent Management
1. Agent spawning and lifecycle
2. Message storage and retrieval
3. Worktree integration
4. Basic loop implementation

### Phase 3: Claude Integration
1. Claude Agent SDK integration
2. Tool execution framework
3. Session management
4. Token tracking

### Phase 4: PR Controller
1. GitHub API client
2. PR queue management
3. Merge strategies
4. Conflict resolution

### Phase 5: Web Interface
1. REST API endpoints
2. WebSocket for real-time updates
3. Chat interface
4. Status dashboard

### Phase 6: BMAD Integration
1. Epic/story parsing
2. BMAD workflow automation
3. Multi-agent orchestration
4. Session forking for context reuse

### Phase 7: Optimization
1. Token optimization with session forking
2. Parallel agent execution
3. Resource management
4. Performance tuning

---

## Part 4: Key Design Decisions

### 4.1 Why Rust?

1. **Performance**: Low latency for real-time agent management
2. **Safety**: Strong typing prevents state machine bugs
3. **Async**: Native async/await for concurrent agents
4. **SQLite**: Excellent rusqlite/sqlx support
5. **Single Binary**: Easy deployment

### 4.2 Why SQLite?

1. **Embedded**: No external dependencies
2. **ACID**: Reliable state persistence
3. **Simple**: Easy backup and debugging
4. **Fast**: Excellent for this workload
5. **Migration**: Easy schema evolution

### 4.3 Why Session Forking?

1. **Token Savings**: Reuse context across related agents
2. **Faster Starts**: Child agents inherit parent context
3. **Coherence**: Related agents share understanding
4. **Cost Reduction**: Fewer redundant context tokens

### 4.4 Agent as State Machine

1. **Predictable**: Clear state transitions
2. **Resumable**: Agents can be paused/resumed
3. **Observable**: State changes are auditable
4. **Recoverable**: Failed agents can be restarted

---

## Part 5: API Design

### 5.1 CLI Commands

```bash
# Daemon management
orchestrate daemon start [--port 8080]
orchestrate daemon stop
orchestrate daemon status

# Agent management
orchestrate agent spawn --type <type> --task <task> [--worktree <name>]
orchestrate agent list [--state <state>]
orchestrate agent show <id>
orchestrate agent pause <id>
orchestrate agent resume <id>
orchestrate agent terminate <id>

# PR management
orchestrate pr list [--status <status>]
orchestrate pr create [--worktree <name>] [--title <title>]
orchestrate pr merge <number>
orchestrate pr queue

# Worktree management
orchestrate wt create <name> [--base <branch>]
orchestrate wt list
orchestrate wt remove <name>

# BMAD workflow
orchestrate bmad process [<epic-pattern>]
orchestrate bmad status

# Web interface
orchestrate web [--port 8080]

# Status
orchestrate status
```

### 5.2 REST API

```
GET    /api/agents                    # List agents
POST   /api/agents                    # Spawn agent
GET    /api/agents/:id                # Get agent details
DELETE /api/agents/:id                # Terminate agent
POST   /api/agents/:id/pause          # Pause agent
POST   /api/agents/:id/resume         # Resume agent
GET    /api/agents/:id/messages       # Get message history
POST   /api/agents/:id/messages       # Send message to agent
WS     /api/agents/:id/chat           # WebSocket chat

GET    /api/prs                       # List PRs
POST   /api/prs                       # Create PR
GET    /api/prs/:number               # Get PR details
POST   /api/prs/:number/merge         # Merge PR

GET    /api/worktrees                 # List worktrees
POST   /api/worktrees                 # Create worktree
DELETE /api/worktrees/:name           # Remove worktree

GET    /api/status                    # System status
GET    /api/epics                     # List epics
GET    /api/epics/:id                 # Get epic details
```

---

## Appendix: File Structure

```
orchestrate/
├── Cargo.toml                     # Workspace root
├── crates/
│   ├── orchestrate-core/          # Core types and traits
│   │   ├── src/
│   │   │   ├── agent.rs           # Agent state machine
│   │   │   ├── database.rs        # SQLite layer
│   │   │   ├── session.rs         # Session management
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── orchestrate-claude/        # Claude SDK integration
│   │   ├── src/
│   │   │   ├── client.rs          # API client
│   │   │   ├── loop.rs            # Loop implementation
│   │   │   ├── tools.rs           # Tool execution
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── orchestrate-github/        # GitHub integration
│   │   ├── src/
│   │   │   ├── client.rs          # GitHub API
│   │   │   ├── pr.rs              # PR management
│   │   │   ├── review.rs          # Review handling
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   ├── orchestrate-web/           # Web interface
│   │   ├── src/
│   │   │   ├── api.rs             # REST endpoints
│   │   │   ├── websocket.rs       # WebSocket handling
│   │   │   └── lib.rs
│   │   └── Cargo.toml
│   └── orchestrate-cli/           # CLI application
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands/
│       │   │   ├── daemon.rs
│       │   │   ├── agent.rs
│       │   │   ├── pr.rs
│       │   │   └── bmad.rs
│       │   └── lib.rs
│       └── Cargo.toml
├── migrations/                     # SQLite migrations
│   ├── 001_initial.sql
│   └── 002_bmad.sql
├── .claude/
│   ├── agents/                     # Agent spec files
│   └── skills/                     # Claude Code skills
└── docs/
    ├── ARCHITECTURE.md
    └── GREENFIELD_ANALYSIS.md
```
