---
name: develop
description: Start development in an isolated worktree. Creates branch and spawns story-developer agent.
---

# Develop Skill

Start development work in an isolated git worktree.

## Usage

```
/develop <task-description> [worktree-name]
```

## Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| task-description | Yes | What to implement |
| worktree-name | No | Branch/worktree name (auto-generated if omitted) |

## Workflow

1. **Create Worktree**
   ```bash
   orchestrate wt create <name> --base main
   ```

2. **Spawn Agent**
   ```bash
   orchestrate agent spawn -t story-developer -T "<task>" -w <worktree>
   ```

3. **Monitor Progress**
   - Agent implements task using TDD
   - Commits changes with descriptive messages
   - Runs tests and linting

4. **Mark Done**
   ```bash
   orchestrate done <worktree> "PR title"
   ```

## Example

```
/develop "Add user authentication with JWT" auth-feature
```

This will:
1. Create worktree `auth-feature` from main
2. Spawn story-developer agent
3. Agent implements JWT authentication
4. Creates commits as it progresses

## Parallel Development

Multiple `/develop` commands can run in parallel:

```
/develop "Add login endpoint" login
/develop "Add registration endpoint" register
/develop "Add password reset" reset
```

Each runs in its own worktree, merging via PR queue.

## Completion

When done, the worktree is queued for PR:
- If no PR is open: creates PR immediately
- If PR exists: queues for later

## Tips

- Keep tasks focused (single feature/fix)
- Agent will ask for clarification if needed
- Check status with `orchestrate status`
