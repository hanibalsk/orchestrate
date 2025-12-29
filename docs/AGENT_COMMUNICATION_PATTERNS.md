# Agent Communication Patterns: Caring About Other Agents

This document defines how agents in the Orchestrate system should "care about" each other through proper communication, coordination, and awareness patterns.

## Overview

In a multi-agent orchestration system, agents must be aware of and responsive to each other's states, outputs, and needs. This document outlines the patterns and practices that enable effective inter-agent cooperation.

## Core Principles

### 1. **Agent Awareness**
Agents should be aware of related agents working on the same task or PR.

**Implementation:**
- Use `parent_agent_id` field to link child agents to parent orchestrators
- Store related agent IDs in `AgentContext.custom` field (e.g., `shepherd_agent_id`)
- Query database for related agents before spawning duplicates

**Example:**
```rust
// From event_handlers.rs:292-293
if let Some(shepherd_id) = shepherd_agent_id {
    custom["shepherd_agent_id"] = serde_json::json!(shepherd_id.to_string());
}
```

### 2. **Context Sharing**
Agents should share relevant context through the database and agent context.

**Current Implementation:**
- **AgentContext**: Stores PR number, branch name, working directory, and custom data
- **Custom Data**: Flexible JSON field for agent-specific context like:
  - `shepherd_agent_id`: Link to PR shepherd agent
  - `review_body`: Review comments for issue-fixer
  - `ci_check_id`: CI check details for failure fixers
  - `event_delivery_id`: Webhook event tracking

**Example:**
```rust
// Creating agent context with shared data
let context = AgentContext {
    pr_number: Some(pr_number),
    branch_name: Some(branch_name),
    working_directory: Some(worktree.path.clone()),
    custom: serde_json::json!({
        "repository": repo_full_name,
        "shepherd_agent_id": shepherd_id,
        "review_body": review_comments,
    }),
    ..Default::default()
};
```

### 3. **State Machine Coordination**
Agents respect each other's states and coordinate transitions.

**Agent States:**
- `Created` → `Initializing` → `Running`
- `WaitingForInput` / `WaitingForExternal` (coordination points)
- `Paused` / `Completed` / `Failed` / `Terminated` (terminal states)

**Coordination Pattern:**
```rust
// Check parent/related agent state before proceeding
let parent_agent = database.get_agent(parent_id).await?;
if parent_agent.state == AgentState::Failed {
    // Terminate child agent
    agent.transition_to(AgentState::Terminated)?;
}
```

### 4. **Worktree Sharing**
Multiple agents can work in the same worktree when appropriate.

**Current Implementation:**
- PR shepherd creates worktree for PR branch
- Issue-fixer agents can share the shepherd's worktree
- Database tracks worktree → agent associations

**Pattern:**
```rust
// Issue-fixer uses shepherd's worktree
let shepherd = find_shepherd_for_pr(pr_number).await?;
let context = AgentContext {
    working_directory: shepherd.context.working_directory.clone(),
    // ... link to shepherd
};
```

### 5. **Event-Driven Coordination**
Agents spawn and coordinate based on GitHub events.

**Current Event Handlers:**
1. **PR Opened** → Spawn `pr-shepherd`
2. **PR Review (changes requested)** → Spawn `issue-fixer`, link to shepherd
3. **CI Failure** → Spawn `issue-fixer`, link to shepherd if exists
4. **Push to Main** → Spawn `regression-tester`

**Deduplication:**
- Check for existing agents before spawning
- Use unique identifiers (PR number, check ID, commit SHA)
- Prevent duplicate fixers for same failure

## Communication Patterns

### Pattern 1: Parent-Child Hierarchy

**Use Case:** Orchestrator agents spawn and manage child agents

**Implementation:**
```rust
// Parent spawns child
let child = Agent::new(AgentType::IssueFixer, task)
    .with_parent(parent.id)
    .with_context(parent_context);

database.insert_agent(&child).await?;
```

**Benefits:**
- Clear ownership and lifecycle management
- Child agents inherit context from parent
- Parent can monitor child progress

### Pattern 2: Peer Coordination (Shepherd + Fixers)

**Use Case:** PR shepherd coordinates with issue-fixer agents

**Implementation:**
```rust
// Fixer links to shepherd
let shepherd_id = find_shepherd_for_pr(pr_number).await?;
let context = AgentContext {
    custom: serde_json::json!({
        "shepherd_agent_id": shepherd_id,
    }),
    // ...
};
```

**Benefits:**
- Issue-fixers report to shepherd
- Shepherd can track all active fixers
- Coordinated PR lifecycle management

### Pattern 3: Pipeline Stages

**Use Case:** Event-driven pipelines with stage dependencies

**Implementation:**
```yaml
# From pipeline.rs
stages:
  - name: validate
    depends_on: []
  - name: deploy
    depends_on: [validate]
    requires_approval: true
```

**Benefits:**
- Explicit stage ordering
- Approval gates for human oversight
- Rollback support for failures

### Pattern 4: Message Broadcasting

**Use Case:** Notify multiple agents of system events

**Future Implementation:**
```rust
// Broadcast event to all listening agents
coordinator.broadcast(NetworkEvent::PrMerged {
    pr_number,
    branch_name,
});
```

## Anti-Patterns to Avoid

### ❌ Agent Isolation
**Problem:** Agents work independently without awareness of related work

**Example:**
```rust
// BAD: Spawn fixer without checking for shepherd
let fixer = Agent::new(AgentType::IssueFixer, task);
database.insert_agent(&fixer).await?;
```

**Solution:**
```rust
// GOOD: Link fixer to shepherd if exists
let shepherd_id = find_shepherd_for_pr(pr_number).await?;
let fixer = Agent::new(AgentType::IssueFixer, task)
    .with_context(context_with_shepherd_link);
```

### ❌ Duplicate Work
**Problem:** Multiple agents spawn for same task/failure

**Example:**
```rust
// BAD: Always spawn fixer on CI failure
let fixer = Agent::new(AgentType::IssueFixer, task);
database.insert_agent(&fixer).await?;
```

**Solution:**
```rust
// GOOD: Check for existing fixer first
if let Some(existing) = find_duplicate_ci_fixer(check_id, sha, pr).await? {
    return Ok(()); // Skip duplicate
}
```

### ❌ Context Loss
**Problem:** Agents don't share critical context

**Example:**
```rust
// BAD: Create agent without context
let agent = Agent::new(AgentType::IssueFixer, "Fix issue");
```

**Solution:**
```rust
// GOOD: Include all relevant context
let context = AgentContext {
    pr_number: Some(pr_number),
    branch_name: Some(branch_name),
    custom: serde_json::json!({
        "review_body": review_comments,
        "shepherd_agent_id": shepherd_id,
        "ci_check_id": check_id,
    }),
    // ...
};
```

## Implementation Checklist

When implementing agent coordination:

- [ ] Check for related agents in database before spawning
- [ ] Link to parent/shepherd agent via `parent_agent_id` or `custom` field
- [ ] Share context through `AgentContext`
- [ ] Implement deduplication logic for repeated events
- [ ] Respect agent state transitions
- [ ] Use shared worktrees when appropriate
- [ ] Add tracing/logging for coordination events
- [ ] Document agent relationships in code comments

## Current Implementation Status

### ✅ Implemented
- Agent state machine with transition validation
- AgentContext with custom JSON data
- Parent-child relationships via `parent_agent_id`
- Peer coordination (shepherd + fixers) via custom context
- Worktree sharing for PR agents
- Event-driven agent spawning (webhooks)
- Deduplication for CI failures
- Pipeline stage dependencies

### 🔲 Not Yet Implemented
- Agent-to-agent messaging system
- Network coordinator for broadcast events
- Agent discovery/registry
- State synchronization across agents
- Agent capability negotiation
- Cross-agent memory/learning
- Agent supervision trees (Erlang-style)

## Future Enhancements

### 1. Agent Network Coordinator
Centralized coordinator for agent communication and lifecycle management.

```rust
pub struct NetworkCoordinator {
    agents: HashMap<Uuid, AgentHandle>,
    events: broadcast::Sender<NetworkEvent>,
}

impl NetworkCoordinator {
    pub async fn broadcast(&self, event: NetworkEvent) {
        self.events.send(event).await;
    }

    pub async fn find_agents_for_pr(&self, pr_number: i32) -> Vec<&Agent> {
        self.agents.values()
            .filter(|a| a.context.pr_number == Some(pr_number))
            .collect()
    }
}
```

### 2. Agent Capabilities and Skills
Agents declare capabilities and can request help from specialized agents.

```rust
pub enum AgentCapability {
    TestGeneration,
    SecurityScanning,
    CodeReview,
    Deployment,
}

impl Agent {
    pub fn has_capability(&self, cap: AgentCapability) -> bool {
        self.agent_type.capabilities().contains(&cap)
    }

    pub async fn request_help(&self, cap: AgentCapability) -> Option<Agent> {
        coordinator.find_agent_with_capability(cap).await
    }
}
```

### 3. Agent Supervision
Supervisor agents monitor and restart failed child agents.

```rust
pub struct Supervisor {
    strategy: RestartStrategy,
    children: Vec<Uuid>,
}

impl Supervisor {
    pub async fn supervise(&self) {
        for child_id in &self.children {
            let child = database.get_agent(child_id).await?;
            if child.state == AgentState::Failed {
                self.restart_child(child_id).await?;
            }
        }
    }
}
```

## Conclusion

"Caring about other agents" means:
1. **Being aware** of related agents and their states
2. **Sharing context** through database and message passing
3. **Coordinating actions** to avoid duplication and conflicts
4. **Respecting lifecycle** states and transitions
5. **Communicating** through events and shared data structures

By following these patterns, the Orchestrate system enables effective multi-agent coordination for autonomous software development.
