-- Prompt Optimization Schema

-- Prompt versions table
CREATE TABLE IF NOT EXISTS prompt_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_type TEXT NOT NULL,
    version INTEGER NOT NULL,
    content TEXT NOT NULL,
    description TEXT,
    parent_version_id INTEGER,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    activated_at TEXT,
    deactivated_at TEXT,
    FOREIGN KEY (parent_version_id) REFERENCES prompt_versions(id) ON DELETE SET NULL,
    CONSTRAINT unique_agent_version UNIQUE (agent_type, version)
);

CREATE INDEX IF NOT EXISTS idx_prompt_versions_agent ON prompt_versions(agent_type);
CREATE INDEX IF NOT EXISTS idx_prompt_versions_active ON prompt_versions(is_active);

-- Prompt effectiveness tracking
CREATE TABLE IF NOT EXISTS prompt_effectiveness (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prompt_version_id INTEGER NOT NULL UNIQUE,
    usage_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    total_duration_secs REAL NOT NULL DEFAULT 0,
    total_feedback_score REAL NOT NULL DEFAULT 0,
    feedback_count INTEGER NOT NULL DEFAULT 0,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (prompt_version_id) REFERENCES prompt_versions(id) ON DELETE CASCADE
);

-- Prompt suggestions table
CREATE TABLE IF NOT EXISTS prompt_suggestions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_type TEXT NOT NULL,
    section TEXT NOT NULL,
    current_content TEXT,
    suggested_content TEXT NOT NULL,
    reasoning TEXT NOT NULL,
    expected_improvement REAL NOT NULL DEFAULT 0,
    confidence REAL NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'pending',
    experiment_id INTEGER,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    applied_at TEXT,
    CONSTRAINT valid_section CHECK (section IN ('system_instructions', 'role_definition', 'task_description', 'constraints', 'output_format', 'examples', 'context', 'custom')),
    CONSTRAINT valid_status CHECK (status IN ('pending', 'approved', 'rejected', 'testing', 'applied', 'rolled_back')),
    FOREIGN KEY (experiment_id) REFERENCES experiments(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_prompt_suggestions_agent ON prompt_suggestions(agent_type);
CREATE INDEX IF NOT EXISTS idx_prompt_suggestions_status ON prompt_suggestions(status);

-- Prompt optimization config (singleton)
CREATE TABLE IF NOT EXISTS prompt_optimization_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    enabled INTEGER NOT NULL DEFAULT 1,
    auto_suggest INTEGER NOT NULL DEFAULT 1,
    auto_test INTEGER NOT NULL DEFAULT 0,
    min_samples_for_analysis INTEGER NOT NULL DEFAULT 20,
    min_improvement_threshold REAL NOT NULL DEFAULT 0.05,
    confidence_threshold REAL NOT NULL DEFAULT 0.7,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Insert default config
INSERT OR IGNORE INTO prompt_optimization_config (id, enabled, auto_suggest, auto_test, min_samples_for_analysis, min_improvement_threshold, confidence_threshold)
VALUES (1, 1, 1, 0, 20, 0.05, 0.7);
