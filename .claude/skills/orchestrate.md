---
name: orchestrate
description: Start the orchestrate daemon and manage multi-agent workflows.
---

# Orchestrate Skill

Control the multi-agent orchestration system.

## Usage

```
/orchestrate [command]
```

## Commands

### Start System

```bash
# Start daemon (background)
orchestrate daemon start

# Start with web UI
orchestrate web --port 8080
```

### Agent Management

```bash
# Spawn agent
orchestrate agent spawn -t story-developer -T "implement feature X"

# List all agents
orchestrate agent list

# Show agent details
orchestrate agent show <id>

# Control agent
orchestrate agent pause <id>
orchestrate agent resume <id>
orchestrate agent terminate <id>
```

### Worktree Management

```bash
# Create worktree for parallel work
orchestrate wt create feature-name --base main

# List worktrees
orchestrate wt list

# Clean up worktree
orchestrate wt remove feature-name
```

### PR Queue

```bash
# View merge queue
orchestrate pr queue

# Merge PR
orchestrate pr merge <number> --strategy squash
```

### System Status

```bash
# Show status
orchestrate status

# JSON output for scripting
orchestrate status --json
```

## Web Interface

Open `http://localhost:8080` to access:
- Real-time agent status
- Chat interface per agent
- PR queue visualization
- BMAD workflow progress

## Architecture

```
┌─────────────────────────────────────────┐
│           Background Controller          │
├─────────┬─────────┬─────────┬───────────┤
│ Story   │ Code    │ PR      │ BMAD      │
│ Dev     │ Review  │ Shepherd│ Orchestr  │
├─────────┴─────────┴─────────┴───────────┤
│              SQLite Database             │
└─────────────────────────────────────────┘
```
