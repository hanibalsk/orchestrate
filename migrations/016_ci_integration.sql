-- CI/CD Integration Schema

-- CI configurations
CREATE TABLE IF NOT EXISTS ci_configs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    provider TEXT NOT NULL,
    api_url TEXT,
    auth_type TEXT NOT NULL,
    token TEXT,
    custom_config TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT valid_provider CHECK (provider IN ('github_actions', 'gitlab_ci', 'circleci', 'jenkins', 'custom')),
    CONSTRAINT valid_auth_type CHECK (auth_type IN ('bearer', 'basic', 'api_key', 'none'))
);

-- CI runs
CREATE TABLE IF NOT EXISTS ci_runs (
    id TEXT PRIMARY KEY,
    provider TEXT NOT NULL,
    workflow_name TEXT NOT NULL,
    branch TEXT NOT NULL,
    commit_sha TEXT,
    status TEXT NOT NULL,
    conclusion TEXT,
    started_at TEXT,
    completed_at TEXT,
    duration_seconds INTEGER,
    url TEXT,
    triggered_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT valid_provider CHECK (provider IN ('github_actions', 'gitlab_ci', 'circleci', 'jenkins', 'custom')),
    CONSTRAINT valid_status CHECK (status IN ('queued', 'in_progress', 'completed', 'cancelled', 'skipped')),
    CONSTRAINT valid_conclusion CHECK (conclusion IS NULL OR conclusion IN ('success', 'failure', 'cancelled', 'skipped', 'timed_out', 'action_required', 'neutral'))
);

CREATE INDEX IF NOT EXISTS idx_ci_runs_provider ON ci_runs(provider);
CREATE INDEX IF NOT EXISTS idx_ci_runs_status ON ci_runs(status);
CREATE INDEX IF NOT EXISTS idx_ci_runs_branch ON ci_runs(branch);
CREATE INDEX IF NOT EXISTS idx_ci_runs_created_at ON ci_runs(created_at);

-- CI jobs within runs
CREATE TABLE IF NOT EXISTS ci_jobs (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    conclusion TEXT,
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (run_id) REFERENCES ci_runs(id) ON DELETE CASCADE,
    CONSTRAINT valid_status CHECK (status IN ('queued', 'in_progress', 'completed', 'cancelled', 'skipped')),
    CONSTRAINT valid_conclusion CHECK (conclusion IS NULL OR conclusion IN ('success', 'failure', 'cancelled', 'skipped', 'timed_out', 'action_required', 'neutral'))
);

CREATE INDEX IF NOT EXISTS idx_ci_jobs_run_id ON ci_jobs(run_id);
CREATE INDEX IF NOT EXISTS idx_ci_jobs_status ON ci_jobs(status);

-- CI steps within jobs
CREATE TABLE IF NOT EXISTS ci_steps (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    job_id TEXT NOT NULL,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    conclusion TEXT,
    step_number INTEGER NOT NULL,
    FOREIGN KEY (job_id) REFERENCES ci_jobs(id) ON DELETE CASCADE,
    CONSTRAINT valid_status CHECK (status IN ('queued', 'in_progress', 'completed', 'cancelled', 'skipped')),
    CONSTRAINT valid_conclusion CHECK (conclusion IS NULL OR conclusion IN ('success', 'failure', 'cancelled', 'skipped', 'timed_out', 'action_required', 'neutral'))
);

CREATE INDEX IF NOT EXISTS idx_ci_steps_job_id ON ci_steps(job_id);

-- CI artifacts
CREATE TABLE IF NOT EXISTS ci_artifacts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL,
    name TEXT NOT NULL,
    path TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    download_url TEXT,
    expires_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (run_id) REFERENCES ci_runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ci_artifacts_run_id ON ci_artifacts(run_id);

-- CI failure analysis
CREATE TABLE IF NOT EXISTS ci_failure_analysis (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id TEXT NOT NULL UNIQUE,
    is_flaky INTEGER NOT NULL DEFAULT 0,
    flaky_confidence REAL NOT NULL DEFAULT 0,
    should_auto_fix INTEGER NOT NULL DEFAULT 0,
    analyzed_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (run_id) REFERENCES ci_runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ci_failure_analysis_run_id ON ci_failure_analysis(run_id);

-- Failed jobs
CREATE TABLE IF NOT EXISTS ci_failed_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    analysis_id INTEGER NOT NULL,
    job_name TEXT NOT NULL,
    step_name TEXT,
    error_summary TEXT NOT NULL,
    log_url TEXT,
    FOREIGN KEY (analysis_id) REFERENCES ci_failure_analysis(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ci_failed_jobs_analysis_id ON ci_failed_jobs(analysis_id);

-- Failed tests
CREATE TABLE IF NOT EXISTS ci_failed_tests (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    analysis_id INTEGER NOT NULL,
    test_name TEXT NOT NULL,
    test_file TEXT,
    error_message TEXT NOT NULL,
    stack_trace TEXT,
    failure_count INTEGER NOT NULL DEFAULT 1,
    is_flaky INTEGER NOT NULL DEFAULT 0,
    FOREIGN KEY (analysis_id) REFERENCES ci_failure_analysis(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ci_failed_tests_analysis_id ON ci_failed_tests(analysis_id);
CREATE INDEX IF NOT EXISTS idx_ci_failed_tests_test_name ON ci_failed_tests(test_name);
CREATE INDEX IF NOT EXISTS idx_ci_failed_tests_is_flaky ON ci_failed_tests(is_flaky);

-- Error messages
CREATE TABLE IF NOT EXISTS ci_error_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    analysis_id INTEGER NOT NULL,
    message TEXT NOT NULL,
    FOREIGN KEY (analysis_id) REFERENCES ci_failure_analysis(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ci_error_messages_analysis_id ON ci_error_messages(analysis_id);

-- Recommendations
CREATE TABLE IF NOT EXISTS ci_recommendations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    analysis_id INTEGER NOT NULL,
    recommendation TEXT NOT NULL,
    FOREIGN KEY (analysis_id) REFERENCES ci_failure_analysis(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ci_recommendations_analysis_id ON ci_recommendations(analysis_id);

-- Flaky test history
CREATE TABLE IF NOT EXISTS ci_flaky_test_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    test_name TEXT NOT NULL,
    run_id TEXT NOT NULL,
    failed INTEGER NOT NULL,
    detected_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (run_id) REFERENCES ci_runs(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_ci_flaky_test_history_test_name ON ci_flaky_test_history(test_name);
CREATE INDEX IF NOT EXISTS idx_ci_flaky_test_history_run_id ON ci_flaky_test_history(run_id);
