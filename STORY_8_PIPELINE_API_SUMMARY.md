# Story 8: Pipeline REST API - Implementation Summary

**Epic:** 004 - Event-Driven Pipelines
**Story:** 8 - Pipeline REST API
**Status:** Completed
**Date:** 2025-12-29

## Overview

Implemented comprehensive REST API endpoints for pipeline management, pipeline runs, and approval workflows. All endpoints follow established patterns from existing API code and include full test coverage.

## Acceptance Criteria - All Met

### Pipeline Management
- [x] `GET /api/pipelines` - List all pipelines
- [x] `POST /api/pipelines` - Create new pipeline
- [x] `GET /api/pipelines/:name` - Get pipeline by name
- [x] `PUT /api/pipelines/:name` - Update pipeline definition/enabled
- [x] `DELETE /api/pipelines/:name` - Delete pipeline
- [x] `POST /api/pipelines/:name/run` - Trigger manual run
- [x] `GET /api/pipelines/:name/runs` - List runs for pipeline

### Pipeline Run Management
- [x] `GET /api/pipeline-runs/:id` - Get run details
- [x] `POST /api/pipeline-runs/:id/cancel` - Cancel run (pending/running/waiting only)

### Approval Management
- [x] `GET /api/approvals` - List pending approvals
- [x] `POST /api/approvals/:id/approve` - Submit approval decision
- [x] `POST /api/approvals/:id/reject` - Submit rejection decision

## Implementation Details

### File Modified
- `/crates/orchestrate-web/src/api.rs` (+1321 lines)

### Request/Response Types

**Pipeline Types:**
```rust
CreatePipelineRequest {
    name: String,
    definition: String,
    enabled: Option<bool>,
}

UpdatePipelineRequest {
    definition: Option<String>,
    enabled: Option<bool>,
}

PipelineResponse {
    id: i64,
    name: String,
    definition: String,
    enabled: bool,
    created_at: String,
}
```

**Pipeline Run Types:**
```rust
TriggerRunRequest {
    trigger_event: Option<String>,
}

PipelineRunResponse {
    id: i64,
    pipeline_id: i64,
    status: String,
    trigger_event: Option<String>,
    started_at: Option<String>,
    completed_at: Option<String>,
    created_at: String,
}
```

**Approval Types:**
```rust
ApprovalDecisionRequest {
    approver: String,
    comment: Option<String>,
}

ApprovalResponse {
    id: i64,
    stage_id: i64,
    run_id: i64,
    status: String,
    required_approvers: String,
    required_count: i32,
    approval_count: i32,
    rejection_count: i32,
    timeout_seconds: Option<i64>,
    timeout_action: Option<String>,
    timeout_at: Option<String>,
    resolved_at: Option<String>,
    created_at: String,
}
```

### Key Features

1. **Request Validation**
   - Pipeline name: non-empty, max 255 characters
   - Pipeline definition: non-empty
   - Approver: non-empty string

2. **Error Handling**
   - 404 Not Found: Pipeline/run/approval doesn't exist
   - 400 Bad Request: Validation errors, unauthorized approver
   - 409 Conflict: Invalid state transition (e.g., cancel completed run), approval already resolved
   - 500 Internal Server Error: Database/unexpected errors

3. **Integration with Services**
   - Uses `ApprovalService` for approval logic (quorum, authorization)
   - Validates pipeline run status before cancellation
   - Ensures foreign key relationships (pipeline -> run -> stage -> approval)

## Test Coverage

**Total Tests: 59 passing**

### Pipeline CRUD Tests (10)
- List empty pipelines
- Create pipeline with validation (success, empty name, empty definition, long name)
- Get pipeline (success, not found)
- Update pipeline (definition and enabled flag)
- Delete pipeline (success, not found)

### Pipeline Run Tests (9)
- Trigger run (success, nonexistent pipeline)
- List runs for pipeline
- Get run details (success, not found)
- Cancel run (pending, running, already completed - conflict)

### Approval Tests (10)
- List pending approvals (empty, with data)
- Approve approval (success, empty approver, unauthorized approver)
- Reject approval (success)
- Not found errors for approve/reject

### Validation Tests (2)
- CreatePipelineRequest validation
- ApprovalDecisionRequest validation

### Response Conversion Tests (3)
- PipelineResponse from Pipeline
- PipelineRunResponse from PipelineRun
- ApprovalResponse from ApprovalRequest

## Example Usage

### Create Pipeline
```bash
curl -X POST http://localhost:3000/api/pipelines \
  -H "Content-Type: application/json" \
  -d '{
    "name": "deploy-prod",
    "definition": "name: deploy-prod\nstages:\n  - name: deploy\n    agent: deployer"
  }'
```

### Trigger Run
```bash
curl -X POST http://localhost:3000/api/pipelines/deploy-prod/run \
  -H "Content-Type: application/json" \
  -d '{"trigger_event": "manual"}'
```

### List Pending Approvals
```bash
curl http://localhost:3000/api/approvals
```

### Approve
```bash
curl -X POST http://localhost:3000/api/approvals/1/approve \
  -H "Content-Type: application/json" \
  -d '{
    "approver": "user@example.com",
    "comment": "LGTM - looks good to deploy"
  }'
```

### Cancel Run
```bash
curl -X POST http://localhost:3000/api/pipeline-runs/1/cancel
```

## Testing Approach (TDD)

1. **Red Phase**: Wrote comprehensive tests covering all endpoints and edge cases
2. **Green Phase**: Implemented handlers to pass tests
3. **Refactor**: Cleaned up error handling patterns, ensured consistency

## Integration Points

- **Database Layer**: Uses existing `Database` methods for pipelines, runs, stages, approvals
- **ApprovalService**: Delegates approval business logic (quorum, authorization, decisions)
- **Authentication**: Protected routes use existing auth middleware
- **Existing Patterns**: Follows same patterns as agent/instruction/pattern endpoints

## Next Steps

Story 8 is complete. The REST API provides full CRUD operations for pipelines and runs, plus approval workflow management. This enables:

- Frontend integration (Story 9: Pipeline Dashboard UI)
- Manual pipeline triggering via API
- Programmatic approval/rejection workflows
- Pipeline monitoring and management

## Files Changed

1. `/crates/orchestrate-web/src/api.rs`
   - Added 12 new endpoint handlers
   - Added 6 request/response types
   - Added 40+ comprehensive tests
   - Total: +1321 lines

## Commit

```
feat: Add Pipeline REST API endpoints

Implement comprehensive REST API for pipeline management with full test coverage

Implements: Story 8 - Pipeline REST API (Epic 004)
```

## Notes

- All endpoints are protected by authentication middleware
- Foreign key constraints ensure data integrity
- Cancel operation only works on active runs (pending/running/waiting)
- Approval endpoints integrate with ApprovalService for business logic
- Tests use in-memory database for isolation
- Error messages provide clear feedback for debugging
