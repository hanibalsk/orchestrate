-- Cost Analytics Tables
-- Migration: 018_cost_analytics
--
-- Adds cost tracking, budgets, and analytics for multi-agent operations.

-- Cost budgets table
CREATE TABLE IF NOT EXISTS cost_budgets (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    period_type TEXT NOT NULL CHECK(period_type IN ('daily', 'weekly', 'monthly')),
    amount_usd REAL NOT NULL CHECK(amount_usd >= 0),
    alert_threshold_percent INTEGER NOT NULL DEFAULT 80 CHECK(alert_threshold_percent > 0 AND alert_threshold_percent <= 100),
    start_date TEXT NOT NULL,  -- When this budget takes effect (YYYY-MM-DD)
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Cost aggregations by agent
CREATE TABLE IF NOT EXISTS cost_by_agent (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,  -- YYYY-MM-DD format
    agent_id TEXT NOT NULL,
    model TEXT NOT NULL,
    total_input_tokens INTEGER NOT NULL DEFAULT 0,
    total_output_tokens INTEGER NOT NULL DEFAULT 0,
    total_cache_read_tokens INTEGER NOT NULL DEFAULT 0,
    total_cache_write_tokens INTEGER NOT NULL DEFAULT 0,
    request_count INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(date, agent_id, model),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Cost aggregations by epic
CREATE TABLE IF NOT EXISTS cost_by_epic (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,  -- YYYY-MM-DD format
    epic_id TEXT NOT NULL,
    model TEXT NOT NULL,
    total_input_tokens INTEGER NOT NULL DEFAULT 0,
    total_output_tokens INTEGER NOT NULL DEFAULT 0,
    total_cache_read_tokens INTEGER NOT NULL DEFAULT 0,
    total_cache_write_tokens INTEGER NOT NULL DEFAULT 0,
    request_count INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(date, epic_id, model),
    FOREIGN KEY (epic_id) REFERENCES epics(id) ON DELETE CASCADE
);

-- Cost aggregations by story
CREATE TABLE IF NOT EXISTS cost_by_story (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,  -- YYYY-MM-DD format
    story_id TEXT NOT NULL,
    model TEXT NOT NULL,
    total_input_tokens INTEGER NOT NULL DEFAULT 0,
    total_output_tokens INTEGER NOT NULL DEFAULT 0,
    total_cache_read_tokens INTEGER NOT NULL DEFAULT 0,
    total_cache_write_tokens INTEGER NOT NULL DEFAULT 0,
    request_count INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(date, story_id, model),
    FOREIGN KEY (story_id) REFERENCES stories(id) ON DELETE CASCADE
);

-- Cost optimization recommendations
CREATE TABLE IF NOT EXISTS cost_recommendations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    recommendation_type TEXT NOT NULL CHECK(recommendation_type IN (
        'model_downgrade',
        'cache_optimization',
        'prompt_optimization',
        'batch_processing',
        'rate_limiting'
    )),
    entity_type TEXT NOT NULL CHECK(entity_type IN ('agent', 'epic', 'story', 'global')),
    entity_id TEXT,  -- NULL for global recommendations
    description TEXT NOT NULL,
    potential_savings_usd REAL NOT NULL,
    confidence_score REAL NOT NULL CHECK(confidence_score >= 0 AND confidence_score <= 1),
    applied BOOLEAN NOT NULL DEFAULT 0,
    applied_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for performance
CREATE INDEX IF NOT EXISTS idx_cost_agent_date ON cost_by_agent(date);
CREATE INDEX IF NOT EXISTS idx_cost_agent_agent ON cost_by_agent(agent_id);
CREATE INDEX IF NOT EXISTS idx_cost_epic_date ON cost_by_epic(date);
CREATE INDEX IF NOT EXISTS idx_cost_epic_epic ON cost_by_epic(epic_id);
CREATE INDEX IF NOT EXISTS idx_cost_story_date ON cost_by_story(date);
CREATE INDEX IF NOT EXISTS idx_cost_story_story ON cost_by_story(story_id);
CREATE INDEX IF NOT EXISTS idx_cost_recommendations_type ON cost_recommendations(recommendation_type);
CREATE INDEX IF NOT EXISTS idx_cost_recommendations_entity ON cost_recommendations(entity_type, entity_id);
CREATE INDEX IF NOT EXISTS idx_cost_recommendations_applied ON cost_recommendations(applied);
