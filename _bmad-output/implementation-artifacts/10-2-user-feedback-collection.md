# Story 10.2: User Feedback Collection

Status: ready-for-dev

## Story

As a **user**,
I want **to provide explicit feedback (thumbs up/down) on agent outputs**,
so that **the system can learn from my preferences and improve future responses**.

## Acceptance Criteria

1. **AC1**: REST API endpoint `POST /api/feedback` accepts feedback with agent_id, rating, and optional comment
2. **AC2**: CLI command `orchestrate feedback add --agent <id> --rating <pos|neg> --comment <text>` submits feedback
3. **AC3**: Feedback is stored with full agent context (agent_id, message_id, timestamp)
4. **AC4**: Feedback is linked to specific agent outputs via message_id
5. **AC5**: Support for positive, negative, and neutral ratings

## Tasks / Subtasks

- [ ] Task 1: Create Feedback data model (AC: 3, 5)
  - [ ] 1.1: Create Feedback struct with id, agent_id, message_id, rating, comment, created_at, created_by
  - [ ] 1.2: Create FeedbackRating enum (Positive, Negative, Neutral)
  - [ ] 1.3: Add database migration for feedback table
  - [ ] 1.4: Implement Database CRUD methods for feedback

- [ ] Task 2: Implement REST API endpoint (AC: 1)
  - [ ] 2.1: Add POST /api/feedback endpoint in orchestrate-web
  - [ ] 2.2: Add GET /api/feedback/stats endpoint for feedback statistics
  - [ ] 2.3: Add input validation and error handling
  - [ ] 2.4: Add authentication/authorization checks

- [ ] Task 3: Implement CLI command (AC: 2)
  - [ ] 3.1: Add `feedback` subcommand group to CLI
  - [ ] 3.2: Implement `feedback add` with required options
  - [ ] 3.3: Implement `feedback list` for viewing feedback
  - [ ] 3.4: Implement `feedback stats` for summary statistics

- [ ] Task 4: Link feedback to agent context (AC: 4)
  - [ ] 4.1: Store message_id reference with feedback
  - [ ] 4.2: Add method to retrieve agent context for feedback
  - [ ] 4.3: Support feedback on specific tool outputs

- [ ] Task 5: Write tests
  - [ ] 5.1: Unit tests for Feedback model
  - [ ] 5.2: Integration tests for REST endpoints
  - [ ] 5.3: CLI command tests

## Dev Notes

### Architecture Context

- Add new module `feedback.rs` in orchestrate-core
- REST endpoints go in orchestrate-web crate
- CLI commands in orchestrate-cli

### Database Schema

```sql
CREATE TABLE feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    message_id INTEGER,
    rating TEXT NOT NULL CHECK (rating IN ('positive', 'negative', 'neutral')),
    comment TEXT,
    created_at TEXT NOT NULL,
    created_by TEXT,
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);
```

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 2]
- [Source: crates/orchestrate-web/src/lib.rs] - API patterns

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
