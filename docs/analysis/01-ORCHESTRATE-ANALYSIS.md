# Orchestrate Project Analysis

## Overview

**Repository:** `/Users/martinjanci/projects/github.com/hanibalsk/orchestrate`
**Technology:** Pure Bash (~1,300 lines)
**Purpose:** Simple Multi-Agent Orchestrator for Claude Code with PR queue system

---

## Project Structure

```
orchestrate/
├── orchestrate                    # Main CLI executable (Bash, ~966 lines)
├── install.sh                     # Installation manager (~307 lines)
├── README.md                      # User documentation
├── .gitignore                     # Git ignore rules
├── .claude/
│   ├── settings.local.json        # Permissions config
│   └── agents/                    # Agent definitions (7 agents)
│       ├── bmad-orchestrator.md
│       ├── bmad-planner.md
│       ├── story-developer.md
│       ├── pr-shepherd.md
│       ├── code-reviewer.md
│       ├── issue-fixer.md
│       └── explorer.md
├── .orchestrate/                  # Runtime state directory
│   ├── pr-queue                   # FIFO queue file
│   ├── current-pr                 # Current PR number
│   └── shepherd-*.lock            # Active agent PIDs
└── .worktrees/                    # Git worktrees for parallel dev
```

---

## Core Concepts

### 1. PR Queue (One-at-a-Time)

```
All Finished Work → Queue (FIFO) → One Active PR → Merge → Next from Queue
                     (pr-queue)    (current-pr)
```

**Files:**
- `.orchestrate/pr-queue` - Format: `wt_name|title|timestamp`
- `.orchestrate/current-pr` - Plain text PR number

**Key Functions:**
```bash
queue_add()    # Append to queue
queue_list()   # Display queue + current PR
queue_next()   # Get first item
queue_pop()    # Remove first item
```

### 2. Git Worktrees

Isolation pattern for parallel development:

```bash
# Create
git worktree add .worktrees/<name> worktree/<name>

# Use
cd .worktrees/<name> && develop...

# Remove
git worktree remove .worktrees/<name> --force
```

### 3. Agent Spawning

Via Claude CLI with Task tool:

```bash
claude -p "Use the Task tool with subagent_type=\"story-developer\" to: $task
Working directory: $wt_path" --dangerously-skip-permissions
```

---

## Agent Definitions

| Agent | Purpose | Tools | Model |
|-------|---------|-------|-------|
| `bmad-orchestrator` | Orchestrate BMAD epics | Bash,Read,Write,Edit,Glob,Grep,Task | Sonnet |
| `bmad-planner` | Create epics/stories | Bash,Read,Write,Edit,Glob,Grep | Sonnet |
| `story-developer` | Implement features (TDD) | Bash,Read,Write,Edit,Glob,Grep | Sonnet |
| `pr-shepherd` | Watch PRs, fix issues | Bash,Read,Write,Edit,Glob,Grep,Task | Sonnet |
| `code-reviewer` | Read-only analysis | Bash,Read,Glob,Grep | Sonnet |
| `issue-fixer` | Fix CI/test failures | Bash,Read,Write,Edit,Glob,Grep | Sonnet |
| `explorer` | Fast codebase search | Read,Glob,Grep | Haiku |

---

## Commands

### PR Queue Commands
```bash
orchestrate pr set <number>        # Set existing PR as current
orchestrate pr queue               # Show queued finished work
orchestrate pr status              # Check current PR
orchestrate pr create [wt_name]    # Create PR from worktree
orchestrate pr next                # Create PR from next queued
orchestrate pr clear               # Clear current PR tracking
orchestrate done <wt_name> [title] # Mark worktree done, queue for PR
```

### Worktree Commands
```bash
orchestrate wt create <name> [base]  # Create isolated worktree
orchestrate wt remove <name>         # Remove worktree
orchestrate wt list                  # List all worktrees
```

### Development Commands
```bash
orchestrate develop <task> [wt]      # Spawn story-developer
orchestrate bmad [epic] [wt]         # Spawn bmad-orchestrator
orchestrate story <id> [wt]          # Spawn story-developer for story
orchestrate shepherd <pr>            # Spawn pr-shepherd
orchestrate review <target>          # Spawn code-reviewer
orchestrate parallel <t1> <t2>...    # Spawn explorer agents in parallel
orchestrate spawn <agent> <task>     # Spawn any agent
```

### Automation
```bash
orchestrate loop                     # Run fully automated loop
orchestrate status                   # One-shot status display
orchestrate interactive              # Interactive Claude prompt
```

### Loop Environment Variables
```bash
LOOP_INTERVAL=30                    # Check interval (default: 30s)
MAX_CONCURRENT_SHEPHERDS=3          # Parallel shepherd limit
AUTO_APPROVE_TOKEN=ghp_xxx          # GitHub token for auto-approval
AUTO_MERGE=true                     # Enable auto-merge
```

---

## State Management

### File-Based State

| File | Purpose | Format |
|------|---------|--------|
| `pr-queue` | Queue of finished work | `wt_name\|title\|timestamp` |
| `current-pr` | Current open PR | Plain text number |
| `shepherd-*.lock` | Active shepherd PIDs | Process ID |
| `work.lock` | Active epic worker PID | Process ID |

### No Database

- All state in plain files
- Easy to debug and inspect
- Fragile - no transactions
- No history/audit trail

---

## Communication Patterns

### 1. Agent Spawning (Prompt-based)
```bash
claude -p "<prompt>" --dangerously-skip-permissions
```

### 2. File-based State
```bash
echo "$pr_num" > "$CURRENT_PR_FILE"
cat "$CURRENT_PR_FILE"
```

### 3. GitHub API (via gh CLI)
```bash
gh pr view 123 --json state,title
gh api graphql -f query='...'
```

### 4. Process Management (PID files)
```bash
echo "$!" > "$SHEPHERD_LOCK"
kill -0 "$(cat "$SHEPHERD_LOCK")" 2>/dev/null
```

---

## Strengths

1. **Simple** - Pure Bash, no dependencies beyond git/gh/claude
2. **One-PR-at-a-Time** - Ensures quality, prevents merge conflicts
3. **Concurrent Shepherds** - Up to 3 parallel PR monitors
4. **ASCII UI** - Real-time status dashboard
5. **Auto-Merge** - Healthy PRs merged automatically
6. **Worktree Isolation** - Parallel development without conflicts

## Weaknesses

1. **No Database** - State is fragile, no transactions
2. **No Session Management** - Each agent starts fresh
3. **No Web Interface** - CLI only
4. **Limited Scalability** - Single-threaded main loop
5. **No Token Optimization** - No session reuse
6. **No History** - No audit trail of agent actions

---

## Key Code Patterns

### Main Loop
```bash
while true; do
    # Check pending PRs
    check_all_pending_prs

    # Spawn shepherds for PRs needing attention
    for pr in $open_prs; do
        spawn_shepherd "$pr" &
    done

    # Process epic queue
    process_next_epic

    sleep "$LOOP_INTERVAL"
done
```

### Agent Invocation
```bash
spawn_agent() {
    local agent_type="$1"
    local task="$2"
    local wt_path="$3"

    claude -p "Use the Task tool with subagent_type=\"$agent_type\" to: $task
Working directory: $wt_path" --dangerously-skip-permissions
}
```

### PR Status Check
```bash
pr_status() {
    local pr_num="$1"
    gh pr view "$pr_num" --json state,reviews,checks \
        -q '{state:.state, approved:([.reviews[]|select(.state=="APPROVED")]|length>0), ci:(.checks|all(.conclusion=="success"))}'
}
```
