-- Token Tracking Tables
-- Migration: 005_token_tracking
--
-- Adds token tracking for instructions and sessions.
-- Supports cache efficiency metrics and cost analysis.

-- Add token tracking columns to instruction_usage
ALTER TABLE instruction_usage ADD COLUMN input_tokens INTEGER DEFAULT 0;
ALTER TABLE instruction_usage ADD COLUMN output_tokens INTEGER DEFAULT 0;
ALTER TABLE instruction_usage ADD COLUMN cache_read_tokens INTEGER DEFAULT 0;
ALTER TABLE instruction_usage ADD COLUMN cache_write_tokens INTEGER DEFAULT 0;

-- Add total token tracking to instruction_effectiveness
ALTER TABLE instruction_effectiveness ADD COLUMN total_input_tokens INTEGER DEFAULT 0;
ALTER TABLE instruction_effectiveness ADD COLUMN total_output_tokens INTEGER DEFAULT 0;
ALTER TABLE instruction_effectiveness ADD COLUMN total_cache_read_tokens INTEGER DEFAULT 0;
ALTER TABLE instruction_effectiveness ADD COLUMN total_cache_write_tokens INTEGER DEFAULT 0;
ALTER TABLE instruction_effectiveness ADD COLUMN avg_tokens_per_run REAL;

-- Session cache statistics table
CREATE TABLE IF NOT EXISTS session_token_stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL,
    agent_id TEXT NOT NULL,
    turn_number INTEGER NOT NULL,
    input_tokens INTEGER NOT NULL DEFAULT 0,
    output_tokens INTEGER NOT NULL DEFAULT 0,
    cache_read_tokens INTEGER NOT NULL DEFAULT 0,
    cache_write_tokens INTEGER NOT NULL DEFAULT 0,
    context_window_used INTEGER NOT NULL DEFAULT 0,  -- Tokens used in context window
    messages_included INTEGER NOT NULL DEFAULT 0,    -- Number of messages in context
    messages_summarized INTEGER NOT NULL DEFAULT 0,  -- Messages that were summarized
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Daily token usage aggregation for cost tracking
CREATE TABLE IF NOT EXISTS daily_token_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL,  -- YYYY-MM-DD format
    model TEXT NOT NULL,
    total_input_tokens INTEGER NOT NULL DEFAULT 0,
    total_output_tokens INTEGER NOT NULL DEFAULT 0,
    total_cache_read_tokens INTEGER NOT NULL DEFAULT 0,
    total_cache_write_tokens INTEGER NOT NULL DEFAULT 0,
    request_count INTEGER NOT NULL DEFAULT 0,
    agent_count INTEGER NOT NULL DEFAULT 0,
    estimated_cost_usd REAL,  -- Estimated cost in USD
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(date, model)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_session_stats_session ON session_token_stats(session_id);
CREATE INDEX IF NOT EXISTS idx_session_stats_agent ON session_token_stats(agent_id);
CREATE INDEX IF NOT EXISTS idx_session_stats_turn ON session_token_stats(session_id, turn_number);
CREATE INDEX IF NOT EXISTS idx_daily_usage_date ON daily_token_usage(date);
CREATE INDEX IF NOT EXISTS idx_daily_usage_model ON daily_token_usage(model);
