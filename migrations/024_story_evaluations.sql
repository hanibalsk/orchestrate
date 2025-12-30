-- Story Evaluations Schema
-- Epic 016: Autonomous Epic Processing - Story 8

-- Story evaluation records - track comprehensive work evaluations per story
CREATE TABLE IF NOT EXISTS story_evaluations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    story_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    status TEXT NOT NULL CHECK(status IN (
        'complete', 'in_progress', 'blocked', 'failed',
        'needs_review', 'needs_review_fixes', 'needs_ci_fixes',
        'needs_pr_approval', 'ready_to_merge'
    )),
    criteria_met_count INTEGER NOT NULL DEFAULT 0,
    criteria_total_count INTEGER NOT NULL DEFAULT 0,
    ci_passed BOOLEAN NOT NULL DEFAULT FALSE,
    review_passed BOOLEAN NOT NULL DEFAULT FALSE,
    review_iteration INTEGER NOT NULL DEFAULT 0,
    pr_mergeable BOOLEAN NOT NULL DEFAULT FALSE,
    feedback TEXT,
    details TEXT NOT NULL DEFAULT '{}',  -- JSON: full evaluation result
    evaluated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_story_evaluations_story_id ON story_evaluations(story_id);
CREATE INDEX IF NOT EXISTS idx_story_evaluations_agent_id ON story_evaluations(agent_id);
CREATE INDEX IF NOT EXISTS idx_story_evaluations_session_id ON story_evaluations(session_id);
CREATE INDEX IF NOT EXISTS idx_story_evaluations_status ON story_evaluations(status);
CREATE INDEX IF NOT EXISTS idx_story_evaluations_evaluated_at ON story_evaluations(evaluated_at);

-- Code review results table - track review iterations
CREATE TABLE IF NOT EXISTS code_review_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    story_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    verdict TEXT NOT NULL CHECK(verdict IN (
        'approved', 'changes_requested', 'needs_discussion', 'pending'
    )),
    reviewer TEXT,
    iteration INTEGER NOT NULL DEFAULT 1,
    issue_count INTEGER NOT NULL DEFAULT 0,
    blocking_issue_count INTEGER NOT NULL DEFAULT 0,
    issues TEXT NOT NULL DEFAULT '[]',  -- JSON array of issues
    raw_output TEXT,
    reviewed_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_code_review_results_story_id ON code_review_results(story_id);
CREATE INDEX IF NOT EXISTS idx_code_review_results_verdict ON code_review_results(verdict);
CREATE INDEX IF NOT EXISTS idx_code_review_results_reviewed_at ON code_review_results(reviewed_at);

-- CI check results table - track CI/build status
CREATE TABLE IF NOT EXISTS ci_check_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    story_id TEXT,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    check_name TEXT NOT NULL,
    status TEXT NOT NULL CHECK(status IN (
        'running', 'passed', 'failed', 'cancelled', 'timeout', 'pending'
    )),
    url TEXT,
    failure_details TEXT,
    duration_secs INTEGER,
    checked_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_ci_check_results_story_id ON ci_check_results(story_id);
CREATE INDEX IF NOT EXISTS idx_ci_check_results_agent_id ON ci_check_results(agent_id);
CREATE INDEX IF NOT EXISTS idx_ci_check_results_status ON ci_check_results(status);
CREATE INDEX IF NOT EXISTS idx_ci_check_results_checked_at ON ci_check_results(checked_at);
