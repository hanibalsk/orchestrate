-- Success Patterns Tables
-- Migration: 011_success_patterns
--
-- Stores patterns learned from successful agent runs to identify
-- effective strategies for future agents. This complements the
-- learning_patterns table which focuses on error patterns.

-- Success patterns table
CREATE TABLE IF NOT EXISTS success_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern_type TEXT NOT NULL,              -- tool_sequence, prompt_structure, context_size, model_choice, timing
    agent_type TEXT,                         -- NULL for all types, or specific agent type
    task_type TEXT,                          -- Optional categorization of task
    pattern_signature TEXT NOT NULL UNIQUE,  -- Hash for deduplication
    pattern_data TEXT NOT NULL,              -- JSON with pattern details
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    avg_completion_time_ms INTEGER,          -- Average completion time in milliseconds
    avg_token_usage INTEGER,                 -- Average token usage
    success_rate REAL NOT NULL DEFAULT 1.0,  -- Success rate (0.0-1.0)
    first_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT valid_pattern_type CHECK (pattern_type IN ('tool_sequence', 'prompt_structure', 'context_size', 'model_choice', 'timing'))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_success_patterns_type ON success_patterns(pattern_type);
CREATE INDEX IF NOT EXISTS idx_success_patterns_agent ON success_patterns(agent_type);
CREATE INDEX IF NOT EXISTS idx_success_patterns_task ON success_patterns(task_type);
CREATE INDEX IF NOT EXISTS idx_success_patterns_signature ON success_patterns(pattern_signature);
CREATE INDEX IF NOT EXISTS idx_success_patterns_occurrence ON success_patterns(occurrence_count DESC);
