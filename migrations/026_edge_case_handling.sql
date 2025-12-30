-- Edge Case Handling Schema
-- Epic 016: Autonomous Epic Processing - Story 14

-- Edge case events table - track detected edge cases and their handling
CREATE TABLE IF NOT EXISTS edge_case_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT,
    agent_id TEXT,
    story_id TEXT,
    edge_case_type TEXT NOT NULL CHECK(edge_case_type IN (
        'delayed_ci_review', 'merge_conflict', 'flaky_test', 'service_downtime',
        'dependency_failure', 'review_ping_pong', 'context_overflow', 'rate_limit',
        'timeout', 'auth_error', 'network_error', 'unknown'
    )),
    resolution TEXT NOT NULL DEFAULT 'pending' CHECK(resolution IN (
        'pending', 'auto_resolved', 'manual_resolved', 'bypassed', 'failed', 'retrying', 'waiting'
    )),
    action_taken TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    context TEXT NOT NULL DEFAULT '{}',  -- JSON: additional context
    detected_at TEXT NOT NULL DEFAULT (datetime('now')),
    resolved_at TEXT,
    resolution_notes TEXT
);

CREATE INDEX IF NOT EXISTS idx_edge_case_events_session_id ON edge_case_events(session_id);
CREATE INDEX IF NOT EXISTS idx_edge_case_events_agent_id ON edge_case_events(agent_id);
CREATE INDEX IF NOT EXISTS idx_edge_case_events_story_id ON edge_case_events(story_id);
CREATE INDEX IF NOT EXISTS idx_edge_case_events_type ON edge_case_events(edge_case_type);
CREATE INDEX IF NOT EXISTS idx_edge_case_events_resolution ON edge_case_events(resolution);
CREATE INDEX IF NOT EXISTS idx_edge_case_events_detected_at ON edge_case_events(detected_at);

-- Edge case learnings table - track patterns for automatic handling
CREATE TABLE IF NOT EXISTS edge_case_learnings (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    edge_case_type TEXT NOT NULL CHECK(edge_case_type IN (
        'delayed_ci_review', 'merge_conflict', 'flaky_test', 'service_downtime',
        'dependency_failure', 'review_ping_pong', 'context_overflow', 'rate_limit',
        'timeout', 'auth_error', 'network_error', 'unknown'
    )),
    pattern TEXT NOT NULL,  -- Error pattern or context pattern
    success_rate REAL NOT NULL DEFAULT 0.0,
    avg_resolution_time_seconds REAL,
    recommended_action TEXT NOT NULL,
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    last_occurrence TEXT NOT NULL DEFAULT (datetime('now')),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(edge_case_type, pattern)
);

CREATE INDEX IF NOT EXISTS idx_edge_case_learnings_type ON edge_case_learnings(edge_case_type);
CREATE INDEX IF NOT EXISTS idx_edge_case_learnings_success_rate ON edge_case_learnings(success_rate);
CREATE INDEX IF NOT EXISTS idx_edge_case_learnings_occurrence ON edge_case_learnings(occurrence_count);
