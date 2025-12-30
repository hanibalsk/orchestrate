-- Review Iterations Schema
-- Epic 016: Autonomous Epic Processing - Story 9

-- Review iterations table - track each review iteration per story
CREATE TABLE IF NOT EXISTS review_iterations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    story_id TEXT NOT NULL,
    iteration INTEGER NOT NULL,
    reviewer_type TEXT NOT NULL CHECK(reviewer_type IN (
        'automated', 'human', 'copilot', 'external'
    )),
    reviewer TEXT,
    verdict TEXT NOT NULL CHECK(verdict IN (
        'approved', 'changes_requested', 'needs_discussion', 'pending'
    )) DEFAULT 'pending',
    issue_count INTEGER NOT NULL DEFAULT 0,
    blocking_issue_count INTEGER NOT NULL DEFAULT 0,
    escalation_level TEXT NOT NULL CHECK(escalation_level IN (
        'none', 'suggest_human', 'require_human', 'senior', 'block'
    )) DEFAULT 'none',
    duration_secs INTEGER,
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_review_iterations_story_id ON review_iterations(story_id);
CREATE INDEX IF NOT EXISTS idx_review_iterations_verdict ON review_iterations(verdict);
CREATE INDEX IF NOT EXISTS idx_review_iterations_escalation ON review_iterations(escalation_level);

-- Review requests table - track pending review requests
CREATE TABLE IF NOT EXISTS review_requests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    story_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    branch TEXT NOT NULL,
    base_branch TEXT NOT NULL,
    pr_number INTEGER,
    iteration INTEGER NOT NULL DEFAULT 1,
    changed_files TEXT NOT NULL DEFAULT '[]',  -- JSON array
    criteria TEXT NOT NULL DEFAULT '[]',  -- JSON array
    previous_issues TEXT NOT NULL DEFAULT '[]',  -- JSON array
    status TEXT NOT NULL CHECK(status IN (
        'pending', 'in_progress', 'completed', 'cancelled'
    )) DEFAULT 'pending',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_review_requests_story_id ON review_requests(story_id);
CREATE INDEX IF NOT EXISTS idx_review_requests_status ON review_requests(status);
