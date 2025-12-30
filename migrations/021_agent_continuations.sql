-- Agent Continuations Schema
-- Epic 016: Autonomous Epic Processing - Story 3

-- Agent continuations table - track requests to continue completed agents
CREATE TABLE IF NOT EXISTS agent_continuations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    reason TEXT NOT NULL CHECK(reason IN (
        'review_feedback', 'test_failures', 'incomplete_criteria',
        'additional_task', 'fix_request', 'retry'
    )),
    message TEXT NOT NULL,           -- The continuation message to send
    context TEXT NOT NULL DEFAULT '{}',  -- JSON: additional context
    status TEXT NOT NULL DEFAULT 'pending' CHECK(status IN (
        'pending', 'executing', 'completed', 'failed', 'cancelled'
    )),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT,
    result TEXT,                     -- JSON: result details
    error_message TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_continuations_agent_id ON agent_continuations(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_continuations_status ON agent_continuations(status);
CREATE INDEX IF NOT EXISTS idx_agent_continuations_created_at ON agent_continuations(created_at);
