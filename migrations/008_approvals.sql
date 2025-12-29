-- Approval Gates System
-- Migration for Epic 004 Story 5: Approval Gates

-- Approval requests table - stores approval requests for pipeline stages
CREATE TABLE IF NOT EXISTS approval_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    stage_id INTEGER NOT NULL REFERENCES pipeline_stages(id) ON DELETE CASCADE,
    run_id INTEGER NOT NULL REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'approved', 'rejected', 'delegated', 'timed_out')),
    required_approvers TEXT NOT NULL,  -- Comma-separated list of approver identifiers
    required_count INTEGER NOT NULL DEFAULT 1,  -- Number of approvals needed (quorum)
    approval_count INTEGER NOT NULL DEFAULT 0,  -- Current number of approvals
    rejection_count INTEGER NOT NULL DEFAULT 0,  -- Current number of rejections
    timeout_seconds INTEGER,  -- Timeout in seconds (NULL means no timeout)
    timeout_action TEXT CHECK (timeout_action IN ('approve', 'reject') OR timeout_action IS NULL),  -- Action on timeout
    timeout_at TEXT,  -- When the approval times out
    resolved_at TEXT,  -- When the approval was resolved
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Approval decisions table - stores individual approval/rejection decisions (audit trail)
CREATE TABLE IF NOT EXISTS approval_decisions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    approval_id INTEGER NOT NULL REFERENCES approval_requests(id) ON DELETE CASCADE,
    approver TEXT NOT NULL,  -- Approver identifier (email, username, etc.)
    decision INTEGER NOT NULL CHECK (decision IN (0, 1)),  -- 0=reject, 1=approve
    comment TEXT,  -- Optional comment with the decision
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_approval_requests_stage_id ON approval_requests(stage_id);
CREATE INDEX IF NOT EXISTS idx_approval_requests_run_id ON approval_requests(run_id);
CREATE INDEX IF NOT EXISTS idx_approval_requests_status ON approval_requests(status);
CREATE INDEX IF NOT EXISTS idx_approval_requests_timeout_at ON approval_requests(timeout_at);
CREATE INDEX IF NOT EXISTS idx_approval_decisions_approval_id ON approval_decisions(approval_id);
CREATE INDEX IF NOT EXISTS idx_approval_decisions_approver ON approval_decisions(approver);
CREATE INDEX IF NOT EXISTS idx_approval_decisions_created_at ON approval_decisions(created_at);
