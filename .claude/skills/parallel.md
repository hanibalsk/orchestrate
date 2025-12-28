---
name: parallel
description: Run multiple agents in parallel across isolated worktrees.
---

# Parallel Skill

Execute multiple tasks simultaneously in separate worktrees.

## Usage

```
/parallel <task1> <task2> [task3] ...
```

## Examples

```bash
# Two parallel tasks
/parallel "Add login API" "Add registration API"

# Three parallel tasks
/parallel "Frontend auth" "Backend auth" "Auth tests"

# With explicit worktree names
/parallel "login:Add login" "register:Add registration"
```

## How It Works

1. **Create Worktrees** - One per task from main branch
2. **Spawn Agents** - story-developer for each
3. **Monitor** - Track all agents concurrently
4. **Queue PRs** - Each completion queues for merge

## Workflow Diagram

```
main ─────────────────────────────────────────────►
     │           │           │
     ▼           ▼           ▼
   wt-1        wt-2        wt-3
  (task1)     (task2)     (task3)
     │           │           │
     ▼           ▼           ▼
  Agent 1     Agent 2     Agent 3
     │           │           │
     ▼           ▼           ▼
  done →      done →      done →
     │           │           │
     ▼           ▼           ▼
┌────────────────────────────────┐
│         PR Queue               │
│  1. task1  2. task2  3. task3  │
└────────────────────────────────┘
```

## Resource Management

- Each task gets its own worktree
- Worktrees share git objects (space efficient)
- Agents run concurrently (token usage parallel)

## Status Monitoring

```bash
# View all agents
orchestrate agent list

# View ASCII dashboard
orchestrate status

# View web dashboard
orchestrate web -p 8080
```

## Merge Order

PRs merge in completion order:
1. First task done → PR created immediately
2. Subsequent → queued behind
3. Queue processes one at a time

## Conflict Handling

If task2 conflicts with merged task1:
1. conflict-resolver spawned automatically
2. Resolves conflicts
3. Re-runs tests
4. Continues merge

## Limitations

- Max parallel tasks: 5 (configurable)
- Each task should be independent
- Shared file changes may cause conflicts

## Example Session

```bash
$ /parallel "Add user model" "Add API routes" "Add tests"

Creating worktrees...
  ✓ wt-user-model
  ✓ wt-api-routes
  ✓ wt-tests

Spawning agents...
  ✓ Agent a1b2c3 (story-developer) → wt-user-model
  ✓ Agent d4e5f6 (story-developer) → wt-api-routes
  ✓ Agent g7h8i9 (story-developer) → wt-tests

Progress:
  [████████░░░░░░░░] user-model: Writing models...
  [██████████████░░] api-routes: Testing endpoints...
  [████░░░░░░░░░░░░] tests: Setting up fixtures...
```
