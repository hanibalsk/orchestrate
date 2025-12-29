-- Dynamic Model Selection Schema

-- Model performance tracking
CREATE TABLE IF NOT EXISTS model_performance (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model TEXT NOT NULL,
    task_type TEXT NOT NULL,
    agent_type TEXT,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    total_tokens INTEGER NOT NULL DEFAULT 0,
    total_cost REAL NOT NULL DEFAULT 0.0,
    total_duration_secs REAL NOT NULL DEFAULT 0.0,
    sample_count INTEGER NOT NULL DEFAULT 0,
    last_used_at TEXT,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT unique_model_task UNIQUE (model, task_type, agent_type)
);

CREATE INDEX IF NOT EXISTS idx_model_perf_model ON model_performance(model);
CREATE INDEX IF NOT EXISTS idx_model_perf_task ON model_performance(task_type);
CREATE INDEX IF NOT EXISTS idx_model_perf_agent ON model_performance(agent_type);

-- Model selection rules
CREATE TABLE IF NOT EXISTS model_selection_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    task_type TEXT,
    agent_type TEXT,
    complexity TEXT,
    preferred_model TEXT NOT NULL,
    fallback_model TEXT,
    max_cost REAL,
    min_success_rate REAL,
    priority INTEGER NOT NULL DEFAULT 0,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT valid_complexity CHECK (complexity IS NULL OR complexity IN ('simple', 'medium', 'complex', 'very_complex'))
);

CREATE INDEX IF NOT EXISTS idx_model_rules_task ON model_selection_rules(task_type);
CREATE INDEX IF NOT EXISTS idx_model_rules_agent ON model_selection_rules(agent_type);
CREATE INDEX IF NOT EXISTS idx_model_rules_enabled ON model_selection_rules(enabled);

-- Model selection config (singleton table)
CREATE TABLE IF NOT EXISTS model_selection_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    optimization_goal TEXT NOT NULL DEFAULT 'balanced',
    max_cost_per_task REAL,
    min_success_rate REAL NOT NULL DEFAULT 0.6,
    min_samples_for_auto INTEGER NOT NULL DEFAULT 10,
    enabled INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT valid_goal CHECK (optimization_goal IN ('cost', 'quality', 'balanced'))
);

-- Insert default config
INSERT OR IGNORE INTO model_selection_config (id, optimization_goal, max_cost_per_task, min_success_rate, min_samples_for_auto, enabled)
VALUES (1, 'balanced', 0.50, 0.6, 10, 1);

-- Model selection history (for auditing and learning)
CREATE TABLE IF NOT EXISTS model_selection_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    task_type TEXT NOT NULL,
    complexity TEXT,
    selected_model TEXT NOT NULL,
    selection_reason TEXT,
    rule_id INTEGER,
    was_successful INTEGER,
    tokens_used INTEGER,
    cost REAL,
    duration_secs REAL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    FOREIGN KEY (rule_id) REFERENCES model_selection_rules(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_model_history_agent ON model_selection_history(agent_id);
CREATE INDEX IF NOT EXISTS idx_model_history_task ON model_selection_history(task_type);
CREATE INDEX IF NOT EXISTS idx_model_history_model ON model_selection_history(selected_model);
CREATE INDEX IF NOT EXISTS idx_model_history_created ON model_selection_history(created_at);
