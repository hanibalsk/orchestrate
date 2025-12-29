-- A/B Testing Experiments Schema

-- Experiments table
CREATE TABLE IF NOT EXISTS experiments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    hypothesis TEXT,
    experiment_type TEXT NOT NULL DEFAULT 'prompt',
    metric TEXT NOT NULL DEFAULT 'success_rate',
    agent_type TEXT,
    status TEXT NOT NULL DEFAULT 'draft',
    min_samples INTEGER NOT NULL DEFAULT 100,
    confidence_level REAL NOT NULL DEFAULT 0.95,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    started_at TEXT,
    completed_at TEXT,
    winner_variant_id INTEGER,
    CONSTRAINT valid_experiment_type CHECK (experiment_type IN ('prompt', 'model', 'instruction', 'context', 'custom')),
    CONSTRAINT valid_metric CHECK (metric IN ('success_rate', 'completion_time', 'token_usage', 'cost', 'feedback_score', 'custom')),
    CONSTRAINT valid_status CHECK (status IN ('draft', 'running', 'paused', 'completed', 'cancelled')),
    CONSTRAINT valid_confidence CHECK (confidence_level > 0 AND confidence_level < 1),
    CONSTRAINT valid_min_samples CHECK (min_samples > 0)
);

CREATE INDEX IF NOT EXISTS idx_experiments_status ON experiments(status);
CREATE INDEX IF NOT EXISTS idx_experiments_agent_type ON experiments(agent_type);

-- Experiment variants table
CREATE TABLE IF NOT EXISTS experiment_variants (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experiment_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    description TEXT,
    is_control INTEGER NOT NULL DEFAULT 0,
    weight INTEGER NOT NULL DEFAULT 50,
    config TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (experiment_id) REFERENCES experiments(id) ON DELETE CASCADE,
    CONSTRAINT valid_weight CHECK (weight > 0 AND weight <= 100),
    CONSTRAINT unique_variant_name UNIQUE (experiment_id, name)
);

CREATE INDEX IF NOT EXISTS idx_variants_experiment ON experiment_variants(experiment_id);

-- Experiment assignments (agent -> variant mapping)
CREATE TABLE IF NOT EXISTS experiment_assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experiment_id INTEGER NOT NULL,
    variant_id INTEGER NOT NULL,
    agent_id TEXT NOT NULL,
    assigned_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (experiment_id) REFERENCES experiments(id) ON DELETE CASCADE,
    FOREIGN KEY (variant_id) REFERENCES experiment_variants(id) ON DELETE CASCADE,
    CONSTRAINT unique_agent_experiment UNIQUE (experiment_id, agent_id)
);

CREATE INDEX IF NOT EXISTS idx_assignments_experiment ON experiment_assignments(experiment_id);
CREATE INDEX IF NOT EXISTS idx_assignments_variant ON experiment_assignments(variant_id);
CREATE INDEX IF NOT EXISTS idx_assignments_agent ON experiment_assignments(agent_id);

-- Experiment observations (metric values per assignment)
CREATE TABLE IF NOT EXISTS experiment_observations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    assignment_id INTEGER NOT NULL,
    metric_name TEXT NOT NULL,
    metric_value REAL NOT NULL,
    recorded_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (assignment_id) REFERENCES experiment_assignments(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_observations_assignment ON experiment_observations(assignment_id);
CREATE INDEX IF NOT EXISTS idx_observations_metric ON experiment_observations(metric_name);
