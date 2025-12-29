-- Pipeline System Tables
-- Migration: 006_pipelines
--
-- Adds tables for event-driven pipelines with multi-stage workflows,
-- conditional execution, approval gates, and rollback support.

-- Pipelines table
-- Stores pipeline definitions with YAML configuration
CREATE TABLE IF NOT EXISTS pipelines (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    definition TEXT NOT NULL,           -- YAML pipeline definition
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT
);

-- Pipeline runs table
-- Tracks execution instances of pipelines
CREATE TABLE IF NOT EXISTS pipeline_runs (
    id TEXT PRIMARY KEY,
    pipeline_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, running, waiting_approval, succeeded, failed, cancelled
    trigger_event TEXT,                      -- Event that triggered this run (e.g., 'pull_request.merged')
    trigger_data TEXT,                       -- JSON data about the trigger
    variables TEXT,                          -- JSON variables for this run
    started_at TEXT,
    completed_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (pipeline_id) REFERENCES pipelines(id) ON DELETE CASCADE
);

-- Pipeline stages table
-- Tracks individual stage executions within a pipeline run
CREATE TABLE IF NOT EXISTS pipeline_stages (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    stage_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, running, succeeded, failed, skipped, cancelled
    agent_id TEXT,                           -- Agent executing this stage
    stage_config TEXT,                       -- JSON configuration for this stage
    output TEXT,                             -- Stage output/result
    error_message TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (run_id) REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE SET NULL
);

-- Pipeline approvals table
-- Tracks approval requests for pipeline stages
CREATE TABLE IF NOT EXISTS pipeline_approvals (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    stage_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, approved, rejected, expired, delegated
    approvers TEXT NOT NULL,                 -- JSON array of approver identities
    required_approvals INTEGER DEFAULT 1,    -- Number of approvals needed (quorum)
    timeout_minutes INTEGER,                 -- Auto-reject after timeout
    default_action TEXT,                     -- Action on timeout (approve/reject)
    delegated_to TEXT,                       -- Identity if delegated
    approved_by TEXT,                        -- JSON array of who approved
    rejected_by TEXT,                        -- Who rejected
    comment TEXT,
    reason TEXT,                            -- Rejection reason
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT,
    expires_at TEXT,
    FOREIGN KEY (run_id) REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    FOREIGN KEY (stage_id) REFERENCES pipeline_stages(id) ON DELETE CASCADE
);

-- Pipeline stage dependencies table
-- Tracks dependencies between stages in a run
CREATE TABLE IF NOT EXISTS pipeline_stage_dependencies (
    stage_id TEXT NOT NULL,
    depends_on_stage_id TEXT NOT NULL,
    dependency_type TEXT DEFAULT 'required',  -- required, optional
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (stage_id, depends_on_stage_id),
    FOREIGN KEY (stage_id) REFERENCES pipeline_stages(id) ON DELETE CASCADE,
    FOREIGN KEY (depends_on_stage_id) REFERENCES pipeline_stages(id) ON DELETE CASCADE
);

-- Pipeline rollback history table
-- Tracks rollback executions
CREATE TABLE IF NOT EXISTS pipeline_rollbacks (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    stage_id TEXT NOT NULL,                  -- Stage that failed and triggered rollback
    rollback_target_stage_id TEXT,           -- Stage to roll back to
    status TEXT NOT NULL DEFAULT 'pending',  -- pending, running, succeeded, failed
    trigger_reason TEXT NOT NULL,
    agent_id TEXT,                           -- Agent executing rollback
    error_message TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (run_id) REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    FOREIGN KEY (stage_id) REFERENCES pipeline_stages(id) ON DELETE CASCADE,
    FOREIGN KEY (rollback_target_stage_id) REFERENCES pipeline_stages(id) ON DELETE SET NULL,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE SET NULL
);

-- Pipeline audit log
-- Tracks all pipeline state changes and actions
CREATE TABLE IF NOT EXISTS pipeline_audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,               -- pipeline, run, stage, approval, rollback
    entity_id TEXT NOT NULL,
    action TEXT NOT NULL,                    -- created, updated, deleted, triggered, approved, rejected, etc.
    old_value TEXT,                          -- JSON snapshot before
    new_value TEXT,                          -- JSON snapshot after
    user_identity TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_pipelines_name ON pipelines(name);
CREATE INDEX IF NOT EXISTS idx_pipelines_enabled ON pipelines(enabled);
CREATE INDEX IF NOT EXISTS idx_pipeline_runs_pipeline ON pipeline_runs(pipeline_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_runs_status ON pipeline_runs(status);
CREATE INDEX IF NOT EXISTS idx_pipeline_runs_created ON pipeline_runs(created_at);
CREATE INDEX IF NOT EXISTS idx_pipeline_stages_run ON pipeline_stages(run_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_stages_status ON pipeline_stages(status);
CREATE INDEX IF NOT EXISTS idx_pipeline_stages_agent ON pipeline_stages(agent_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_approvals_run ON pipeline_approvals(run_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_approvals_status ON pipeline_approvals(status);
CREATE INDEX IF NOT EXISTS idx_pipeline_rollbacks_run ON pipeline_rollbacks(run_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_audit_entity ON pipeline_audit_log(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_audit_created ON pipeline_audit_log(created_at);
