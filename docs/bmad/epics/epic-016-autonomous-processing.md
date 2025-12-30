# Epic 016: Autonomous Epic Processing

Implement fully autonomous workflow where the background-controller orchestrates the complete development lifecycle from epic discovery through PR merge.

**Priority:** Critical
**Effort:** Large
**Use Cases:** UC-020

## Overview

Enable a single command to trigger fully autonomous development where the controller discovers epics, spawns developer agents, evaluates completion, triggers reviews, manages PRs, and handles all edge cases without human intervention. This is the core autonomous development capability that ties all other features together.

## Stories

### Story 1: Autonomous Session Management

Create infrastructure for tracking autonomous processing sessions.

**Acceptance Criteria:**
- [ ] Create `autonomous_sessions` table with fields: id, state, started_at, updated_at, completed_at, current_epic_id, current_story_id, current_agent_id, config (JSON), work_queue (JSON), completed_items (JSON), metrics
- [ ] Implement session state machine: IDLE → ANALYZING → DISCOVERING → PLANNING → EXECUTING → REVIEWING → PR_CREATION → PR_MONITORING → PR_FIXING → PR_MERGING → COMPLETING → DONE
- [ ] Add BLOCKED state for intervention-required scenarios
- [ ] Track session metrics: stories_completed, stories_failed, reviews_passed, reviews_failed, total_iterations
- [ ] Support session pause/resume functionality

### Story 2: Decision Engine Core

Implement the decision engine that determines what action to take next.

**Acceptance Criteria:**
- [ ] Create `DecisionEngine` struct in `orchestrate-core`
- [ ] Implement `evaluate_agent_output()` to parse agent completion signals
- [ ] Implement decision types: SpawnAgent, ContinueAgent, TriggerReview, CompleteWork, Escalate, Wait
- [ ] Parse STATUS signals from agent output (COMPLETE, BLOCKED, WAITING)
- [ ] Check acceptance criteria completion against story definition
- [ ] Detect code changes requiring review

### Story 3: Agent Continuation Mechanism

Enable completed agents to be resumed with new tasks in the same Claude context.

**Acceptance Criteria:**
- [ ] Add `continue_agent()` method to `AgentLoop` in loop_runner.rs
- [ ] Load existing message history from database
- [ ] Add continuation message as new user input
- [ ] Preserve session ID for context continuity
- [ ] Transition agent from Completed/Paused → Running
- [ ] Create `agent_continuations` table to track continuation requests
- [ ] Track continuation reason: ReviewFeedback, TestFailures, IncompleteCriteria, AdditionalTask

### Story 4: Context Summarization Protocol

Implement token-efficient context handoffs between agents.

**Acceptance Criteria:**
- [ ] Define summary format: key decisions, files changed, tests added, blockers
- [ ] Add `summarize_output()` method to agents
- [ ] Controller receives condensed context, not full conversation
- [ ] Support structured summary in JSON format for machine parsing
- [ ] Implement summary extraction from agent's final message
- [ ] Track token savings from summarization

### Story 5: Model Selection Engine

Implement intelligent model selection based on task complexity.

**Acceptance Criteria:**
- [ ] Define model tiers: Opus, Sonnet, Sonnet-1M, Haiku
- [ ] Implement complexity scoring: story points, file count, dependency depth
- [ ] Use Opus for: architectural decisions, multi-file refactoring, ambiguous requirements
- [ ] Use Sonnet for: standard implementation, code reviews, issue fixing
- [ ] Use Sonnet-1M for: large file analysis, comprehensive exploration
- [ ] Use Haiku for: quick searches, simple edits, status checks
- [ ] Escalate model after 2 failed retries
- [ ] Use Opus for CRITICAL review issues
- [ ] Add `--model` flag to CLI commands

### Story 6: Stuck Agent Detection

Detect and handle agents that are stuck or making no progress.

**Acceptance Criteria:**
- [ ] Monitor turn count against max_turns threshold (alert at 80%)
- [ ] Detect no meaningful output in last N turns
- [ ] Detect CI check timeout (no update > 30 minutes)
- [ ] Detect PR review delay (async Copilot reviews)
- [ ] Detect merge conflict situations
- [ ] Detect API rate limits and implement backoff
- [ ] Detect approaching context token limits
- [ ] Create `work_evaluations` table for tracking evaluations

### Story 7: Recovery Strategies

Implement recovery actions for stuck or failed agents.

**Acceptance Criteria:**
- [ ] Implement pause and alert for human intervention
- [ ] Implement model escalation (switch to more capable model)
- [ ] Spawn specialized fixer agent for specific issues
- [ ] Fork context and retry with fresh session
- [ ] Escalate to parent controller
- [ ] Track recovery attempts and outcomes
- [ ] Define max retry limits per recovery type

### Story 8: Work Evaluation System

Evaluate if agent work is truly complete.

**Acceptance Criteria:**
- [ ] Check all acceptance criteria marked as met
- [ ] Verify tests passing (CI status check)
- [ ] Check for STATUS: BLOCKED signals
- [ ] Verify code review passed (no CRITICAL/HIGH issues)
- [ ] Check build and lint status
- [ ] Verify PR approved and mergeable
- [ ] Track evaluation history per story
- [ ] Generate feedback for agent continuation when incomplete

### Story 9: Code Review Integration

Trigger and process code reviews in the autonomous workflow.

**Acceptance Criteria:**
- [ ] Automatically trigger code-reviewer after story completion
- [ ] Parse review output for machine-readable verdict (APPROVED, CHANGES_REQUESTED, NEEDS_DISCUSSION)
- [ ] Extract issues with severity (CRITICAL, HIGH, MEDIUM, LOW, NITPICK)
- [ ] Generate continuation message from review feedback
- [ ] Track review iterations per story
- [ ] Escalate after 3 failed review iterations
- [ ] Support both automated and human review handling

### Story 10: PR Workflow Management

Manage the complete PR lifecycle in autonomous mode.

**Acceptance Criteria:**
- [ ] Create PR with structured description (summary, stories, testing)
- [ ] Monitor all CI checks (build, test, lint, security)
- [ ] Parse and address review comments automatically
- [ ] Handle delayed/async CI reviews (Copilot)
- [ ] Detect and handle merge conflicts
- [ ] Implement squash merge with proper commit message
- [ ] Clean up branches and worktrees after merge
- [ ] Transition through PR states: PR_CREATION → PR_MONITORING → PR_FIXING → PR_MERGING

### Story 11: Epic Discovery and Planning

Discover epics and create prioritized work plans.

**Acceptance Criteria:**
- [ ] Scan `docs/bmad/epics/` for epic files
- [ ] Parse epic markdown for stories and acceptance criteria
- [ ] Build dependency graph between stories
- [ ] Create prioritized work queue based on dependencies
- [ ] Track epic status: pending, in_progress, completed, blocked
- [ ] Support epic pattern matching (`--pattern "epic-001-*"`)
- [ ] Generate execution plan for dry-run mode

### Story 12: Autonomous Controller Agent

Create the autonomous-controller agent prompt.

**Acceptance Criteria:**
- [ ] Create `.claude/agents/autonomous-controller.md`
- [ ] Define controller responsibilities and decision logic
- [ ] Document STATUS signals protocol
- [ ] Define coordination patterns with child agents
- [ ] Include recovery strategy documentation
- [ ] Define escalation criteria

### Story 13: CLI Commands

Implement CLI commands for autonomous processing.

**Acceptance Criteria:**
- [ ] `orchestrate epic auto-process [--pattern <pattern>] [--max-agents <n>] [--model <model>] [--dry-run]`
- [ ] `orchestrate epic auto-status [--detailed]`
- [ ] `orchestrate epic auto-pause`
- [ ] `orchestrate epic auto-resume`
- [ ] `orchestrate epic auto-stop [--force]`
- [ ] `orchestrate epic stuck-agents`
- [ ] `orchestrate epic unblock <epic-id>`
- [ ] Output progress updates during execution
- [ ] Support verbose/quiet modes

### Story 14: Edge Case Handling

Handle common edge cases in autonomous processing.

**Acceptance Criteria:**
- [ ] Handle delayed CI reviews (GitHub Copilot async comments)
- [ ] Handle merge conflicts when multiple branches merge
- [ ] Handle flaky tests with retry logic
- [ ] Handle external service downtime (GitHub, CI)
- [ ] Handle story dependency failures
- [ ] Handle review ping-pong (repeated changes requested)
- [ ] Handle context overflow for large changes
- [ ] Log all edge cases for learning

### Story 15: REST API Endpoints

Add REST endpoints for autonomous processing.

**Acceptance Criteria:**
- [ ] `POST /api/epic/auto-process` - Start autonomous processing
- [ ] `GET /api/epic/auto-status` - Get current status
- [ ] `POST /api/epic/auto-pause` - Pause processing
- [ ] `POST /api/epic/auto-resume` - Resume processing
- [ ] `POST /api/epic/auto-stop` - Stop processing
- [ ] `GET /api/epic/stuck-agents` - List stuck agents
- [ ] `POST /api/epic/:id/unblock` - Unblock epic
- [ ] WebSocket events for real-time status updates

### Story 16: Dashboard Integration

Add autonomous processing UI to web dashboard.

**Acceptance Criteria:**
- [ ] Autonomous processing overview panel
- [ ] Real-time state machine visualization
- [ ] Work queue display with priorities
- [ ] Agent status cards with progress
- [ ] Stuck agent alerts with actions
- [ ] Review iteration tracking
- [ ] PR lifecycle visualization
- [ ] Metrics dashboard (completion rate, time per story)

## Definition of Done

- [ ] All stories completed and tested
- [ ] Full autonomous loop working end-to-end
- [ ] Model selection optimizing cost/performance
- [ ] Stuck detection preventing infinite loops
- [ ] Recovery strategies handling common failures
- [ ] PR workflow completing successfully
- [ ] Edge cases handled gracefully
- [ ] Integration tests covering full workflow
- [ ] Documentation complete
- [ ] Performance benchmarks established
