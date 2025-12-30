-- Recovery Attempts Schema
-- Epic 016: Autonomous Epic Processing - Story 7

-- Recovery attempts table - track recovery actions taken for stuck agents
CREATE TABLE IF NOT EXISTS recovery_attempts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    stuck_detection_id INTEGER,
    action_type TEXT NOT NULL CHECK(action_type IN (
        'pause_and_alert', 'model_escalation', 'spawn_fixer', 'fresh_retry',
        'escalate_to_parent', 'retry', 'wait', 'abort'
    )),
    outcome TEXT NOT NULL CHECK(outcome IN (
        'success', 'failed', 'in_progress', 'cancelled', 'skipped'
    )),
    details TEXT NOT NULL DEFAULT '{}',  -- JSON: action details
    attempt_number INTEGER NOT NULL DEFAULT 1,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    error_message TEXT,
    FOREIGN KEY (stuck_detection_id) REFERENCES stuck_agent_detections(id)
);

CREATE INDEX IF NOT EXISTS idx_recovery_attempts_agent_id ON recovery_attempts(agent_id);
CREATE INDEX IF NOT EXISTS idx_recovery_attempts_outcome ON recovery_attempts(outcome);
CREATE INDEX IF NOT EXISTS idx_recovery_attempts_action_type ON recovery_attempts(action_type);
CREATE INDEX IF NOT EXISTS idx_recovery_attempts_started_at ON recovery_attempts(started_at);
