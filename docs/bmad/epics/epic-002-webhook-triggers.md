# Epic 002: GitHub Webhook Triggers

Implement webhook receiver to automatically trigger agents based on GitHub events instead of manual polling.

**Priority:** Critical
**Effort:** Medium
**Use Cases:** UC-101

## Overview

Currently, orchestrate relies on manual commands or polling to detect GitHub events. This epic adds a webhook receiver that automatically spawns appropriate agents when GitHub events occur (PR opened, review submitted, CI failed, etc.).

## Stories

### Story 1: Webhook Receiver Endpoint

Implement HTTP endpoint to receive GitHub webhooks.

**Acceptance Criteria:**
- [x] Add `/webhooks/github` POST endpoint in orchestrate-web
- [x] Parse GitHub webhook payload with proper event type detection
- [x] Verify webhook signature using HMAC-SHA256
- [x] Return 200 OK quickly, process async
- [x] Log all received webhooks for debugging
- [x] Handle malformed payloads gracefully

**Technical Notes:**
- Use `X-Hub-Signature-256` header for verification
- Parse `X-GitHub-Event` header for event type
- Store webhook secret in config

**Implementation Details:**
- Created `webhook.rs` module in orchestrate-web
- Implemented HMAC-SHA256 signature verification
- Added comprehensive unit and integration tests (14 tests total)
- Webhook endpoint available at `/webhooks/github`
- Secret is optional (loaded from `GITHUB_WEBHOOK_SECRET` env or passed explicitly)
- Returns 200 OK with JSON response for valid webhooks
- Returns appropriate error codes (400, 401) for invalid requests
- Logs event type, delivery ID, and payload for debugging

### Story 2: Event Queue System

Implement reliable event queue for webhook processing.

**Acceptance Criteria:**
- [x] Create `webhook_events` table in database
- [x] Queue incoming webhooks before processing
- [x] Implement event processor that polls queue
- [x] Handle duplicate events (idempotency via delivery ID)
- [x] Implement dead-letter handling for failed events
- [x] Add retry logic with exponential backoff

**Technical Notes:**
- Use `X-GitHub-Delivery` as unique event ID
- Max 3 retries before dead-letter

**Implementation Details:**
- Created migration `006_webhook_events.sql` with webhook_events table
- Implemented WebhookEvent and WebhookEventStatus types in orchestrate-core
- Added database operations with idempotent insert via ON CONFLICT(delivery_id)
- Implemented exponential backoff retry logic (1s, 2s, 4s)
- Created WebhookProcessor that polls queue at configurable intervals
- Updated webhook handler to queue events immediately and return 200 OK
- Added 23 comprehensive tests covering all functionality
- Status flow: pending -> processing -> completed/failed/dead_letter
- Processor handles batch processing with configurable batch size
- Dead-letter queue after exceeding max_retries (default 3)
- Includes cleanup operation to delete old completed/dead-letter events

### Story 3: PR Opened Event Handler

Spawn pr-shepherd when PR is opened.

**Acceptance Criteria:**
- [ ] Detect `pull_request.opened` event
- [ ] Extract PR number, branch, repository info
- [ ] Spawn `pr-shepherd` agent for the PR
- [ ] Create worktree for the PR branch
- [ ] Update PR with comment indicating orchestrate is watching
- [ ] Skip if PR is from fork (security)

**Technical Notes:**
- Check `pull_request.head.repo.fork` field
- Use existing pr-shepherd agent type

### Story 4: PR Review Event Handler

Spawn issue-fixer when changes are requested.

**Acceptance Criteria:**
- [ ] Detect `pull_request_review.submitted` event
- [ ] Check if review state is `changes_requested`
- [ ] Parse review comments for actionable feedback
- [ ] Spawn `issue-fixer` agent with review context
- [ ] Link agent to existing PR shepherd if running

### Story 5: CI Status Event Handler

Spawn issue-fixer when CI fails.

**Acceptance Criteria:**
- [ ] Detect `check_run.completed` or `check_suite.completed` event
- [ ] Check if conclusion is `failure` or `timed_out`
- [ ] Extract failed check details and logs URL
- [ ] Spawn `issue-fixer` agent with CI failure context
- [ ] Avoid spawning duplicate fixers for same failure

### Story 6: Push to Main Event Handler

Spawn regression-tester on main branch pushes.

**Acceptance Criteria:**
- [ ] Detect `push` event to main/master branch
- [ ] Extract commit range and changed files
- [ ] Spawn `regression-tester` agent (new agent type)
- [ ] Run test suite and report results
- [ ] Create issue if regression detected

### Story 7: Issue Created Event Handler

Spawn issue-triager for new issues.

**Acceptance Criteria:**
- [ ] Detect `issues.opened` event
- [ ] Parse issue title and body
- [ ] Spawn `issue-triager` agent (new agent type)
- [ ] Agent adds labels, assigns priority
- [ ] Agent may create epic/story if appropriate

### Story 8: Webhook CLI Commands

Add CLI commands for webhook management.

**Acceptance Criteria:**
- [ ] `orchestrate webhook start --port 9000` - Start webhook server
- [ ] `orchestrate webhook list-events` - Show recent events
- [ ] `orchestrate webhook simulate <event-type>` - Test event handling
- [ ] `orchestrate webhook status` - Show webhook server status
- [ ] `orchestrate webhook secret rotate` - Generate new webhook secret

### Story 9: Webhook Configuration

Add configuration for webhook behavior.

**Acceptance Criteria:**
- [ ] Config file support for webhook settings
- [ ] Configure which events to handle
- [ ] Configure which branches to watch
- [ ] Configure agent spawning rules per event type
- [ ] Support event filtering by label, author, path

**Configuration Example:**
```yaml
webhooks:
  secret: ${GITHUB_WEBHOOK_SECRET}
  events:
    pull_request.opened:
      agent: pr-shepherd
      filter:
        base_branch: [main, develop]
        skip_forks: true
    check_run.completed:
      agent: issue-fixer
      filter:
        conclusion: [failure, timed_out]
```

## Definition of Done

- [ ] All stories completed and tested
- [ ] Integration tests for each event type
- [ ] Documentation for webhook setup
- [ ] GitHub webhook configuration guide
- [ ] Security review for signature verification
