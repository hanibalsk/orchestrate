---
name: done
description: Mark a worktree as complete and queue it for PR creation.
---

# Done Skill

Mark development work as complete and queue for pull request.

## Usage

```
/done <worktree-name> [pr-title]
```

## Parameters

| Parameter | Required | Description |
|-----------|----------|-------------|
| worktree-name | Yes | Name of the worktree |
| pr-title | No | Title for the PR (auto-generated if omitted) |

## Examples

```bash
# Mark auth-feature as done
/done auth-feature "Add user authentication"

# Auto-generate title from commits
/done feature-x
```

## Workflow

1. **Validate Worktree**
   - Ensure worktree exists
   - Check for uncommitted changes
   - Verify branch is pushed

2. **Push Changes**
   ```bash
   cd .worktrees/<name>
   git push -u origin <branch>
   ```

3. **Queue for PR**
   - If no PR is currently open: creates PR immediately
   - If PR exists: adds to queue

4. **Update State**
   - Worktree marked as ready
   - Queue file updated

## PR Queue Behavior

```
Current State          Action
────────────────────────────────────
No open PR         →   Create PR now
PR #42 open        →   Add to queue
Queue: [A, B]      →   Add to queue → [A, B, C]
```

## Queue File Format

Located at `~/.orchestrate/pr-queue`:

```
worktree-a|PR Title A|2024-01-15T10:30:00
worktree-b|PR Title B|2024-01-15T11:00:00
worktree-c|PR Title C|2024-01-15T11:30:00
```

## Checking Status

```bash
# View queue
orchestrate pr queue

# View current PR
orchestrate pr status

# View all worktrees
orchestrate wt list
```

## Auto-Title Generation

If no title provided, generates from:
1. Last commit message subject
2. Branch name (kebab-case to Title Case)

```bash
# Branch: add-user-auth
# Auto-title: "Add user auth"
```

## Pre-Checks

Before marking done:
1. All tests pass
2. Linting passes
3. Build succeeds
4. No uncommitted changes

```bash
# Run checks manually
npm test && npm run lint && npm run build
```

## What Happens Next

1. **PR Created** (if no queue)
   - PR opened on GitHub
   - CI starts automatically
   - Shepherd can be spawned

2. **Queued** (if PR exists)
   - Wait for current PR to merge
   - Auto-creates PR when turn comes

## Tips

- Run tests before `/done`
- Use descriptive PR titles
- Check `orchestrate pr queue` for position
