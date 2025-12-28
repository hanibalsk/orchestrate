---
name: pr-queue
description: Manage PR merge queue with auto-approve and conflict resolution.
---

# PR Queue Skill

Manage the pull request merge queue.

## Usage

```
/pr-queue [action]
```

## Actions

### View Queue

```bash
orchestrate pr queue
```

Shows:
- Position in queue
- CI status
- Review status
- Merge readiness

### Add to Queue

```bash
orchestrate pr queue add <number>
```

### Process Queue

Automatically processes PRs in order:
1. Check CI status
2. Check review status
3. Resolve conflicts if needed
4. Merge when ready

## Merge Strategies

- **squash** (default) - Clean single commit
- **rebase** - Linear history
- **merge** - Preserve branch history

```bash
orchestrate pr merge <number> --strategy squash
```

## Conflict Resolution

When conflicts detected:
1. Spawns `conflict-resolver` agent
2. Attempts automatic resolution
3. Falls back to manual if complex

## Auto-Approve Rules

Configure in `.orchestrate/config.toml`:

```toml
[auto_approve]
patterns = [
  "dependabot/*",
  "renovate/*",
]
require_ci = true
```

## Status Indicators

```
✓ Ready to merge
⏳ Waiting for CI
⏳ Waiting for review
⚠️ Has conflicts
✗ CI failed
```
