---
name: status
description: View system status including agents, PRs, worktrees, and queue.
---

# Status Skill

View comprehensive system status.

## Usage

```
/status [component]
```

## Components

| Component | Description |
|-----------|-------------|
| (none) | Full dashboard |
| `agents` | Running agents |
| `pr` | Current PR and queue |
| `wt` | Worktrees |
| `queue` | PR queue only |

## Full Status Display

```
┌─────────────────────────────────────────────────────┐
│                 ORCHESTRATE STATUS                   │
├─────────────────────────────────────────────────────┤
│ Agents                                               │
│  ● Running: 2  ○ Paused: 0  ✓ Completed: 5          │
│                                                      │
│  [a1b2c3] story-developer  Running   auth-feature   │
│  [d4e5f6] pr-shepherd      Waiting   PR #42         │
├─────────────────────────────────────────────────────┤
│ Current PR                                           │
│  #42: Add user authentication                        │
│  ✓ Build  ✓ Test  ⏳ Review                          │
├─────────────────────────────────────────────────────┤
│ Queue                                                │
│  1. api-routes     "Add API endpoints"               │
│  2. dashboard      "Add admin dashboard"             │
├─────────────────────────────────────────────────────┤
│ Worktrees                                            │
│  auth-feature   → PR #42 (active)                    │
│  api-routes     → queued                             │
│  dashboard      → queued                             │
│  wip-refactor   → in progress                        │
└─────────────────────────────────────────────────────┘
```

## Agent Status

```bash
/status agents
```

```
AGENTS (2 running, 1 paused)

ID        Type              State    Task                    Worktree
────────  ────────────────  ───────  ──────────────────────  ─────────────
a1b2c3d4  story-developer   Running  Add user authentication auth-feature
e5f6g7h8  pr-shepherd       Waiting  Watch PR #42            -
i9j0k1l2  code-reviewer     Paused   Review auth module      -
```

## PR Status

```bash
/status pr
```

```
CURRENT PR: #42
Title: Add user authentication
Branch: auth-feature → main
Author: agent-a1b2c3

Checks:
  ✓ build         passed    2m ago
  ✓ test          passed    1m ago
  ✓ lint          passed    1m ago
  ⏳ review       pending   waiting for approval

Reviews:
  ○ @reviewer1    pending

Mergeable: Yes (after approval)
```

## Queue Status

```bash
/status queue
```

```
PR QUEUE (3 items)

#  Worktree      Title                        Queued At
─  ────────────  ───────────────────────────  ─────────────
1  api-routes    Add API endpoints            10 min ago
2  dashboard     Add admin dashboard          5 min ago
3  settings      Add settings page            just now

Current PR: #42 (auth-feature) - waiting for review
```

## Worktree Status

```bash
/status wt
```

```
WORKTREES (4 total)

Name          Branch          Status      Agent
────────────  ──────────────  ──────────  ──────────────
auth-feature  feat/auth       PR #42      pr-shepherd
api-routes    feat/api        queued      -
dashboard     feat/dashboard  queued      -
wip-refactor  refactor/main   working     story-developer
```

## JSON Output

```bash
orchestrate status --json
```

Returns machine-readable JSON for scripting.

## Web Dashboard

```bash
orchestrate web -p 8080
```

Opens browser with real-time dashboard showing all status information.
