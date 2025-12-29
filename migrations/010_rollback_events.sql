-- Rollback Events
-- Migration for Epic 004 Story 6: Rollback Support

-- Rollback events table - stores rollback execution history
CREATE TABLE IF NOT EXISTS rollback_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    failed_stage_name TEXT NOT NULL,
    rollback_to_stage TEXT NOT NULL,
    trigger_type TEXT NOT NULL CHECK (trigger_type IN ('automatic', 'manual')),
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'succeeded', 'failed')),
    error_message TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_rollback_events_run_id ON rollback_events(run_id);
CREATE INDEX IF NOT EXISTS idx_rollback_events_status ON rollback_events(status);
CREATE INDEX IF NOT EXISTS idx_rollback_events_created_at ON rollback_events(created_at);
