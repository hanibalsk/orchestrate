# Story 5: Approval Gates - Implementation Summary

## Overview
Implemented human-in-the-loop approval workflow for critical pipeline stages, allowing pipelines to pause and wait for authorized approvers before proceeding with sensitive operations.

## Files Created

### Core Data Models
- **`crates/orchestrate-core/src/approval.rs`** (444 lines)
  - `ApprovalStatus` enum: Pending, Approved, Rejected, Delegated, TimedOut
  - `ApprovalRequest` struct: Manages approval requests with quorum support
  - `ApprovalDecision` struct: Records individual approval/rejection decisions
  - Comprehensive unit tests (13 tests)

### Service Layer
- **`crates/orchestrate-core/src/approval_service.rs`** (553 lines)
  - `ApprovalService`: High-level approval workflow management
  - Methods:
    - `create_approval()`: Create approval requests
    - `approve()`: Submit approval decision with validation
    - `reject()`: Submit rejection decision
    - `delegate()`: Transfer approval authority
    - `process_timeouts()`: Handle expired approvals
    - `list_pending()`: Query pending approvals
  - Comprehensive tests (10 tests)

### Database Layer
- **`migrations/008_approvals.sql`**
  - `approval_requests` table: Stores approval requests
  - `approval_decisions` table: Audit trail of decisions
  - Proper indexes for efficient querying
  
- **`crates/orchestrate-core/src/database_approval_tests.rs`** (323 lines)
  - Database operation tests (8 tests)
  - Integration tests with pipelines

### Pipeline Integration
- **Modified `crates/orchestrate-core/src/pipeline_executor.rs`**
  - Added approval service integration
  - Enhanced `execute_stage()` to handle approval gates
  - Pauses pipeline when stage requires approval
  - Creates approval request with default 24-hour timeout
  - Marks stage and run as `waiting_approval`

## Key Features Implemented

### 1. Approval Request Creation
```rust
let request = ApprovalRequest::new(
    stage_id,
    run_id,
    "user1@example.com,user2@example.com".to_string(),
    2,  // Quorum: need 2 approvals
    Some(3600),  // 1 hour timeout
    Some("reject".to_string()),  // Default action on timeout
);
```

### 2. Quorum Support
- Configurable required approval count
- Automatic approval when quorum reached
- Rejection quorum (too many rejections to proceed)
- Single approver optimization (immediate approval/rejection)

### 3. Decision Validation
- Verify approver is in authorized list
- Prevent duplicate decisions from same approver
- Prevent decisions on already-resolved approvals
- Maintain complete audit trail

### 4. Delegation Support
```rust
service.delegate(
    approval_id,
    "user1@example.com",  // From
    "user3@example.com",  // To
).await?;
```

### 5. Timeout Handling
- Configurable timeout duration
- Default action (approve/reject/timed_out)
- Automatic processing via `process_timeouts()`
- Proper status tracking

### 6. Pipeline Integration
```yaml
stages:
  - name: deploy-prod
    agent: deployer
    task: "Deploy to production"
    requires_approval: true
    approvers: [team-lead, devops]
```

## Database Schema

### approval_requests Table
```sql
CREATE TABLE approval_requests (
    id INTEGER PRIMARY KEY,
    stage_id INTEGER REFERENCES pipeline_stages(id),
    run_id INTEGER REFERENCES pipeline_runs(id),
    status TEXT CHECK (status IN ('pending', 'approved', 'rejected', 'delegated', 'timed_out')),
    required_approvers TEXT,  -- Comma-separated list
    required_count INTEGER,
    approval_count INTEGER,
    rejection_count INTEGER,
    timeout_seconds INTEGER,
    timeout_action TEXT CHECK (timeout_action IN ('approve', 'reject')),
    timeout_at TEXT,
    resolved_at TEXT,
    created_at TEXT
);
```

### approval_decisions Table (Audit Trail)
```sql
CREATE TABLE approval_decisions (
    id INTEGER PRIMARY KEY,
    approval_id INTEGER REFERENCES approval_requests(id),
    approver TEXT,
    decision INTEGER CHECK (decision IN (0, 1)),  -- 0=reject, 1=approve
    comment TEXT,
    created_at TEXT
);
```

## Test Coverage

### Unit Tests (13)
- ApprovalStatus parsing and serialization
- ApprovalRequest quorum logic
- Status transitions
- Timeout calculations

### Database Tests (8)
- CRUD operations
- Querying by stage/status
- Timeout queries
- Decision audit trail

### Service Tests (10)
- Single/multiple approver workflows
- Rejection quorum
- Unauthorized approver handling
- Duplicate decision prevention
- Delegation
- Timeout processing

### Integration Tests (3)
- Pipeline parser approval fields
- Pipeline executor approval gate handling
- End-to-end approval workflow

**Total: 34 tests, all passing**

## Acceptance Criteria Status

- [x] Pause pipeline at stages with `requires_approval: true`
- [x] Create approval request in database
- [ ] Notify approvers via configured channel (deferred - needs notification system)
- [x] Support multiple approvers with quorum
- [x] Implement approval timeout with default action
- [x] Support delegation of approval
- [x] Audit trail for approval decisions

## Usage Example

### 1. Pipeline Definition
```yaml
name: production-deployment
stages:
  - name: deploy-staging
    agent: deployer
    task: "Deploy to staging"
    
  - name: smoke-test
    agent: tester
    task: "Run smoke tests"
    depends_on: [deploy-staging]
    
  - name: deploy-production
    agent: deployer
    task: "Deploy to production"
    requires_approval: true
    approvers: [tech-lead, devops-lead]
    depends_on: [smoke-test]
```

### 2. Service Usage
```rust
// Create approval service
let service = ApprovalService::new(db);

// Create approval request
let request = service.create_approval(
    stage_id,
    run_id,
    vec!["tech-lead@company.com".to_string(), "devops@company.com".to_string()],
    2,  // Both must approve
    Some(24 * 3600),  // 24 hour timeout
    Some("reject".to_string()),
).await?;

// Approve
let updated = service.approve(
    request.id.unwrap(),
    "tech-lead@company.com".to_string(),
    Some("LGTM - approved for production".to_string()),
).await?;

// Check status
assert_eq!(updated.status, ApprovalStatus::Pending);  // Still need devops approval

// Second approval
service.approve(
    request.id.unwrap(),
    "devops@company.com".to_string(),
    Some("Infrastructure ready".to_string()),
).await?;

// Now approved - pipeline can proceed
```

### 3. Process Timeouts
```rust
// Run periodically (e.g., via cron job)
let timed_out = service.process_timeouts().await?;
for approval in timed_out {
    println!("Approval {} timed out with action: {:?}", 
             approval.id.unwrap(), 
             approval.timeout_action);
}
```

## Next Steps (Future Enhancements)

1. **Notification System Integration**
   - Email notifications to approvers
   - Slack/Teams integration
   - Webhook notifications

2. **CLI Commands** (from epic specification)
   - `orchestrate approval list --pending`
   - `orchestrate approval approve <id> --comment "LGTM"`
   - `orchestrate approval reject <id> --reason "Needs testing"`
   - `orchestrate approval delegate <id> --to user@example.com`

3. **Resume Execution After Approval**
   - Currently, pipelines halt at approval gates
   - Need mechanism to resume stage execution after approval
   - Possible approaches:
     - Background worker polling for approved stages
     - Event-driven resumption via webhooks
     - Manual resume command

4. **Advanced Features**
   - Custom quorum rules (e.g., "at least 2 of 5")
   - Approval expiration (auto-reject after X days)
   - Approval chains (sequential approvals)
   - Conditional approvals based on change scope

## Files Modified
- `crates/orchestrate-core/src/lib.rs`: Added approval module exports
- `crates/orchestrate-core/src/database.rs`: Added approval database methods (278 lines)
- `crates/orchestrate-core/src/pipeline_executor.rs`: Integrated approval gates

## Performance Considerations
- Database indexes on frequently-queried columns (status, timeout_at, stage_id)
- Efficient quorum checking without loading all decisions
- Batch timeout processing for multiple approvals
- Minimal overhead when approval not required

## Security Considerations
- Approver authorization check before accepting decisions
- Audit trail for all approval actions
- Immutable decision records (no updates, only inserts)
- Prevent decision tampering after approval resolved
