-- Work Evaluations Schema
-- Epic 016: Autonomous Epic Processing - Story 6 & 8

-- Work evaluations table - track agent progress evaluations
CREATE TABLE IF NOT EXISTS work_evaluations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    story_id TEXT,
    evaluation_type TEXT NOT NULL CHECK(evaluation_type IN (
        'progress', 'completion', 'stuck_check', 'review_outcome', 'ci_status'
    )),
    status TEXT NOT NULL CHECK(status IN (
        'healthy', 'warning', 'stuck', 'failed', 'complete'
    )),
    details TEXT NOT NULL DEFAULT '{}',  -- JSON: evaluation details
    turn_count INTEGER,
    max_turns INTEGER,
    token_count INTEGER,
    max_tokens INTEGER,
    duration_secs INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_work_evaluations_agent_id ON work_evaluations(agent_id);
CREATE INDEX IF NOT EXISTS idx_work_evaluations_session_id ON work_evaluations(session_id);
CREATE INDEX IF NOT EXISTS idx_work_evaluations_status ON work_evaluations(status);
CREATE INDEX IF NOT EXISTS idx_work_evaluations_created_at ON work_evaluations(created_at);

-- Stuck agent detections table - track stuck agent incidents
CREATE TABLE IF NOT EXISTS stuck_agent_detections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    detection_type TEXT NOT NULL CHECK(detection_type IN (
        'turn_limit', 'no_progress', 'ci_timeout', 'review_delay',
        'merge_conflict', 'rate_limit', 'context_limit', 'error_loop'
    )),
    severity TEXT NOT NULL CHECK(severity IN ('low', 'medium', 'high', 'critical')),
    details TEXT NOT NULL DEFAULT '{}',  -- JSON: detection details
    resolved BOOLEAN NOT NULL DEFAULT FALSE,
    resolution_action TEXT,  -- What action was taken
    detected_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_stuck_agent_detections_agent_id ON stuck_agent_detections(agent_id);
CREATE INDEX IF NOT EXISTS idx_stuck_agent_detections_resolved ON stuck_agent_detections(resolved);
CREATE INDEX IF NOT EXISTS idx_stuck_agent_detections_severity ON stuck_agent_detections(severity);
CREATE INDEX IF NOT EXISTS idx_stuck_agent_detections_detected_at ON stuck_agent_detections(detected_at);
