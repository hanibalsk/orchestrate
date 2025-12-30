-- Autonomous Session Management Schema
-- Epic 016: Autonomous Epic Processing - Story 1

-- Autonomous sessions table - track autonomous processing sessions
CREATE TABLE IF NOT EXISTS autonomous_sessions (
    id TEXT PRIMARY KEY,
    state TEXT NOT NULL CHECK(state IN (
        'idle', 'analyzing', 'discovering', 'planning', 'executing',
        'reviewing', 'pr_creation', 'pr_monitoring', 'pr_fixing',
        'pr_merging', 'completing', 'done', 'blocked', 'paused'
    )),
    started_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    completed_at TEXT,
    current_epic_id TEXT,
    current_story_id TEXT,
    current_agent_id TEXT,
    config TEXT NOT NULL DEFAULT '{}',           -- JSON: session configuration
    work_queue TEXT NOT NULL DEFAULT '[]',       -- JSON: pending work items
    completed_items TEXT NOT NULL DEFAULT '[]',  -- JSON: completed work items
    metrics TEXT NOT NULL DEFAULT '{}',          -- JSON: SessionMetrics
    error_message TEXT,
    blocked_reason TEXT,
    pause_reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_autonomous_sessions_state ON autonomous_sessions(state);
CREATE INDEX IF NOT EXISTS idx_autonomous_sessions_current_epic_id ON autonomous_sessions(current_epic_id);
CREATE INDEX IF NOT EXISTS idx_autonomous_sessions_started_at ON autonomous_sessions(started_at);

-- Session state history - track state transitions
CREATE TABLE IF NOT EXISTS autonomous_session_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    reason TEXT,
    transitioned_at TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',  -- JSON: additional context
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES autonomous_sessions(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_autonomous_session_history_session_id ON autonomous_session_history(session_id);
CREATE INDEX IF NOT EXISTS idx_autonomous_session_history_transitioned_at ON autonomous_session_history(transitioned_at);
