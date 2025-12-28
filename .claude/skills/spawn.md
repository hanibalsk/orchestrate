---
name: spawn
description: Spawn a specific agent type with a task. Low-level agent creation.
---

# Spawn Skill

Directly spawn an agent of a specific type.

## Usage

```
/spawn <agent-type> <task-description>
```

## Agent Types

| Type | Purpose | Model |
|------|---------|-------|
| `story-developer` | Implement features with TDD | sonnet |
| `code-reviewer` | Review code quality | sonnet |
| `issue-fixer` | Fix CI/test failures | sonnet |
| `explorer` | Search and analyze code | haiku |
| `bmad-orchestrator` | Run BMAD workflow | sonnet |
| `bmad-planner` | Create epics/stories | sonnet |
| `pr-shepherd` | Watch PR lifecycle | sonnet |
| `pr-controller` | Manage PR queue | sonnet |
| `conflict-resolver` | Resolve merge conflicts | sonnet |

## Examples

```bash
# Spawn story developer
/spawn story-developer "Add user authentication with OAuth"

# Spawn explorer for research
/spawn explorer "Find all API endpoint definitions"

# Spawn issue fixer
/spawn issue-fixer "Fix failing test in auth.test.ts"

# Spawn code reviewer
/spawn code-reviewer "Review changes in src/api/"
```

## Options

```bash
# With specific worktree
/spawn story-developer "task" --worktree feature-x

# With PR context
/spawn pr-shepherd "Monitor PR" --pr 42

# With epic context
/spawn bmad-orchestrator "Process epic" --epic epic-001
```

## Agent Lifecycle

```
Spawned → Created → Initializing → Running → Completed
                                      │
                              WaitingForInput
                              WaitingForExternal
                                      │
                                   Paused
                                      │
                              Failed/Terminated
```

## Monitoring

```bash
# List all agents
orchestrate agent list

# Show specific agent
orchestrate agent show <id>

# View agent logs
orchestrate agent logs <id>
```

## Control

```bash
# Pause agent
orchestrate agent pause <id>

# Resume agent
orchestrate agent resume <id>

# Terminate agent
orchestrate agent terminate <id>
```

## Database Record

Spawning creates a record in SQLite:

| Field | Value |
|-------|-------|
| id | UUID |
| agent_type | specified type |
| state | created |
| task | description |
| created_at | now |
| worktree | optional |
| pr_number | optional |

## Tips

- Use `/develop` for feature work (auto-creates worktree)
- Use `/spawn` for specific agent needs
- Agents persist state in database
- Check `orchestrate status` for overview
