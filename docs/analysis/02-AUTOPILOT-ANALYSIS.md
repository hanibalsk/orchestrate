# Autopilot Project Analysis

## Overview

**Repository:** `/Users/martinjanci/projects/github.com/hanibalsk/autopilot`
**Technology:** Pure Bash (~1,877 lines)
**Purpose:** State-machine-driven autonomous development orchestrator for BMAD workflow

---

## Project Structure

```
autopilot/
├── scripts/
│   └── bmad-autopilot.sh          # Main orchestrator (~1,877 lines)
├── commands/
│   ├── autopilot.md               # Claude slash command /autopilot
│   └── bmad-autopilot.md          # Alternative command
├── skills/
│   ├── bmad-autopilot/SKILL.md    # Skill definition
│   └── gh-pr-handling/SKILL.md    # GitHub PR skill
├── docs/
│   ├── ARCHITECTURE.md            # State machine details
│   ├── CONFIGURATION.md           # Environment variables
│   └── TROUBLESHOOTING.md         # Debug guide
├── workflows/                      # GitHub Actions
├── install.sh                      # Installer (~500 lines)
├── uninstall.sh                    # Uninstaller
├── CLAUDE.md                       # Project context for Claude
└── README.md                       # User documentation
```

---

## State Machine

### Sequential Mode (PARALLEL_MODE=0)

```
CHECK_PENDING_PR → FIND_EPIC → CREATE_BRANCH → DEVELOP_STORIES →
CODE_REVIEW → CREATE_PR → WAIT_COPILOT → FIND_EPIC (next)
                                ↓
                           FIX_ISSUES ←── (if unresolved threads)
                                ↓
                           WAIT_COPILOT (re-review)
```

### Parallel Mode (PARALLEL_MODE=1)

```
┌─────────────────────────────────────────────────────────────────────────┐
│  ACTIVE DEVELOPMENT                    PENDING PRs (background)         │
│  ┌───────────────────┐                 ┌─────────────────────┐         │
│  │ FIND_EPIC         │                 │ PR #1: epic-7A      │         │
│  │ CREATE_BRANCH     │                 │ status: WAIT_REVIEW │◄─check──┤
│  │ DEVELOP_STORIES   │ (interactive)   └─────────────────────┘         │
│  │ CODE_REVIEW       │ (interactive)   ┌─────────────────────┐         │
│  │ CREATE_PR ────────┼──add to queue──►│ PR #2: epic-8A      │         │
│  │      │            │                 │ status: WAIT_CI     │◄─check──┤
│  │      ▼            │                 └─────────────────────┘         │
│  │ FIND_EPIC (next)  │                                                  │
│  └───────────────────┘                 If PR needs fixes:              │
│                                        → pause active work             │
│  If MAX_PENDING_PRS reached:           → switch to worktree            │
│  → WAIT_PENDING_PRS                    → fix & push                    │
│  → check periodically                  → resume active work            │
│  → resume when slot opens                                              │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## State Persistence

### Sequential Mode State
```json
{
  "phase": "DEVELOP_STORIES",
  "current_epic": "7A",
  "completed_epics": ["1A", "2A", "3B"]
}
```

### Parallel Mode State
```json
{
  "mode": "parallel",
  "active_epic": "8A",
  "active_phase": "DEVELOP_STORIES",
  "active_worktree": "/path/to/worktree",
  "pending_prs": [
    {
      "epic": "7A",
      "pr_number": 123,
      "worktree": "/path/to/worktree",
      "status": "WAIT_REVIEW",
      "last_check": "2024-01-15T10:30:00Z",
      "last_copilot_id": "MDI..."
    }
  ],
  "completed_epics": ["1A", "2A", "3B"],
  "paused_context": null
}
```

---

## Phase Details

### CHECK_PENDING_PR
- Scans for open PRs from previous runs
- Checks current branch state
- Resumes unfinished work

### FIND_EPIC
- Parses `_bmad-output/epics*.md`
- Extracts epic IDs from `^#{2,4} Epic [0-9]`
- Filters by pattern if provided
- Skips completed epics

### CREATE_BRANCH
```bash
git fetch origin
git checkout $BASE_BRANCH
git pull origin $BASE_BRANCH
git checkout -b "feature/epic-$EPIC_ID"
git push -u origin "feature/epic-$EPIC_ID"
```

### DEVELOP_STORIES
- Interactive Claude session
- Uses BMAD workflow: `/bmad:bmm:workflows:dev-story`
- Commits after each story
- Runs local checks

### CODE_REVIEW
- Interactive Claude session
- Uses BMAD workflow: `/bmad:bmm:workflows:code-review`
- Fixes issues found
- Verifies checks pass

### CREATE_PR
```bash
gh pr create --fill --label "epic,automated,epic-$EPIC_ID"
```
- Adds to pending list
- Continues to next epic (non-blocking)

### WAIT_COPILOT
- Polls for Copilot review
- Checks both comments and reviews
- Tracks last processed comment ID
- Checks review threads

### FIX_ISSUES
1. Fetch unresolved threads via GraphQL
2. Get Copilot review body
3. Get CI failures
4. Send to Claude for fixes
5. Post reply to PR
6. Resolve threads via GraphQL
7. Loop back to WAIT_COPILOT

### MERGE_PR
```bash
gh pr merge --squash --delete-branch
```
- Cleans up worktree
- Marks epic completed

---

## Key Functions

### Claude Integration

**Interactive Mode (foreground):**
```bash
run_claude_interactive() {
  claude -p "$prompt" --permission-mode acceptEdits 2>&1 | tee "$output_file"
}
```

**Headless Mode (background):**
```bash
run_claude_headless() {
  claude -p "$prompt" \
    --permission-mode acceptEdits \
    --allowedTools "Bash,Read,Write,Edit,Grep" \
    --max-turns 30
}
```

### GraphQL Operations

**Get Unresolved Threads:**
```bash
gh api graphql -f query='
  query($owner: String!, $repo: String!, $pr: Int!) {
    repository(owner: $owner, name: $repo) {
      pullRequest(number: $pr) {
        reviewThreads(first: 100) {
          nodes {
            id
            isResolved
            path
            line
            comments(first: 10) {
              nodes { body author { login } }
            }
          }
        }
      }
    }
  }
'
```

**Resolve Thread:**
```bash
gh api graphql -f query='
  mutation($threadId: ID!) {
    resolveReviewThread(input: {threadId: $threadId}) {
      thread { isResolved }
    }
  }
'
```

### Worktree Management

```bash
worktree_create() {
  git worktree add "$wt_path" "$branch_name"
}

worktree_remove() {
  git worktree remove "$wt_path" --force
}

worktree_exec() {
  (cd "$wt_path" && "$@")
}
```

### Pending PR Management

```bash
state_add_pending_pr()      # Add PR to pending list
state_update_pending_pr()   # Update PR status
state_remove_pending_pr()   # Remove from list
state_get_pending_pr()      # Get PR info
state_get_all_pending_prs() # Get all pending
state_count_pending_prs()   # Count pending
```

---

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `AUTOPILOT_DEBUG` | 0 | Enable debug logging |
| `AUTOPILOT_VERBOSE` | 0 | Enable verbose console output |
| `MAX_TURNS` | 80 | Claude turns per phase |
| `CHECK_INTERVAL` | 30 | Seconds between polls |
| `MAX_CHECK_WAIT` | 60 | Max poll iterations |
| `MAX_COPILOT_WAIT` | 60 | Max Copilot wait iterations |
| `PARALLEL_MODE` | 0 | Enable parallel development |
| `MAX_PENDING_PRS` | 2 | Max concurrent pending PRs |
| `PARALLEL_CHECK_INTERVAL` | 60 | Seconds between pending checks |
| `AUTOPILOT_BASE_BRANCH` | auto | Override base branch |

### Config File
```bash
# .autopilot/config
AUTOPILOT_DEBUG=1
MAX_TURNS=100
CHECK_INTERVAL=60
PARALLEL_MODE=1
MAX_PENDING_PRS=3
```

---

## Auto-Approve Workflow

GitHub Actions workflow for automatic PR approval:

**Trigger:** `pull_request_review: submitted`

**Conditions (ALL must be met):**
1. At least 10 minutes since last push
2. Copilot review exists
3. All review threads resolved
4. All CI checks passed

**Flow:**
1. Wait 2 min for CI to start
2. Poll CI every 30s until complete
3. Check time since last push (≥10 min)
4. Check Copilot has reviewed
5. Check 0 unresolved threads via GraphQL
6. Dismiss stale approvals if threads exist
7. Approve if all conditions met

---

## Strengths

1. **True State Machine** - JSON persistence, resumable
2. **Parallel Mode** - Work on next epic while PRs wait
3. **GraphQL** - Proper thread resolution
4. **Auto-Continue** - Never blocks on approval
5. **Copilot-Aware** - Handles reviews and threads
6. **Interactive Mode** - Foreground development sessions
7. **Safe Config** - Whitelist-only parsing

## Weaknesses

1. **Bash Complexity** - String handling is error-prone
2. **No Database** - JSON file is fragile
3. **No Session Reuse** - Each Claude call starts fresh
4. **Single-Threaded** - No true parallelism
5. **No Web Interface** - CLI only
6. **No Token Tracking** - No optimization

---

## Key Patterns to Preserve

### 1. Non-Blocking Flow
```
CREATE_PR → add to pending → FIND_EPIC (next)
```
Never wait for approval, continue working.

### 2. Context Preservation
```bash
state_save_active_context()    # Before fixing PR
state_restore_active_context() # After fixing
```

### 3. Thread Resolution
Always: fix → reply → resolve → push

### 4. Copilot ID Tracking
```bash
echo "$latest_id" > "$TMP_DIR/last_copilot_comment_id.txt"
```
Avoid reacting to old reviews.

### 5. Worktree On-Demand
Create worktrees only when fixes needed, not preemptively.
