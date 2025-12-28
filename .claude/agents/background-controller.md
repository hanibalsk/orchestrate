---
name: background-controller
description: Main orchestration daemon that manages agent lifecycle and communication. Use to start the orchestration system.
tools: Bash, Read, Write, Edit, Glob, Grep, Task
---

# Background Controller Agent

The central orchestration daemon that manages all other agents.

## Responsibilities

1. **Agent Lifecycle** - Spawn, monitor, terminate agents
2. **Communication** - Route messages between agents
3. **State Management** - Persist state to SQLite
4. **Resource Control** - Manage worktrees and sessions

## Starting the Controller

```bash
# Start orchestrate daemon
orchestrate daemon start --port 9999

# Check status
orchestrate status

# View web interface
open http://localhost:8080
```

## Agent Management

### Spawn Agents

```bash
# Spawn story developer
orchestrate agent spawn -t story-developer -T "implement user auth"

# Spawn with worktree
orchestrate agent spawn -t story-developer -T "task" -w feature-branch

# List agents
orchestrate agent list

# Show agent details
orchestrate agent show <id>
```

### Agent States

- `Created` - Initial state
- `Initializing` - Setting up
- `Running` - Actively executing
- `WaitingForInput` - Needs user input
- `WaitingForExternal` - Waiting on CI/API
- `Paused` - Temporarily stopped
- `Completed` - Successfully finished
- `Failed` - Encountered error
- `Terminated` - Manually stopped

### Control Agents

```bash
# Pause agent
orchestrate agent pause <id>

# Resume agent
orchestrate agent resume <id>

# Terminate agent
orchestrate agent terminate <id>
```

## Worktree Management

```bash
# Create worktree
orchestrate wt create feature-name --base main

# List worktrees
orchestrate wt list

# Remove worktree
orchestrate wt remove feature-name
```

## PR Queue

```bash
# View PR queue
orchestrate pr queue

# List PRs
orchestrate pr list

# Merge PR
orchestrate pr merge <number> --strategy squash
```

## BMAD Workflow

```bash
# Process epics
orchestrate bmad process "epic-*.md"

# Check BMAD status
orchestrate bmad status

# Reset state
orchestrate bmad reset
```

## Session Management

Sessions enable token optimization through forking:
- Parent session context inherited by children
- Only new messages consume tokens
- Sessions pruned when agents complete

## Communication Flow

1. User input → Web interface → WebSocket
2. WebSocket → Controller → Route to agent
3. Agent output → Controller → Broadcast
4. State changes → SQLite → UI update

## Coordination Patterns

### Sequential
```
Agent A → completes → Agent B starts
```

### Parallel (with worktrees)
```
Agent A (worktree-1) ─┐
Agent B (worktree-2) ─┼→ PR Controller → Merge
Agent C (worktree-3) ─┘
```

### Pipeline
```
Planner → Developer → Reviewer → PR Shepherd
```
