-- Custom Instructions Tables
-- Migration: 004_custom_instructions
--
-- Stores custom instructions that can be injected into agent system prompts.
-- Supports both global instructions and per-agent-type instructions.
-- Includes learning patterns for automatic instruction generation.
-- Tracks effectiveness metrics including penalty scoring.

-- Custom instructions table
CREATE TABLE IF NOT EXISTS custom_instructions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,               -- Human-readable identifier
    content TEXT NOT NULL,                   -- The actual instruction text
    scope TEXT NOT NULL DEFAULT 'global',    -- global, agent_type
    agent_type TEXT,                         -- NULL for global, agent type for scoped
    priority INTEGER NOT NULL DEFAULT 100,   -- Higher = injected earlier in prompt
    enabled INTEGER NOT NULL DEFAULT 1,      -- 0 = disabled, 1 = enabled
    source TEXT NOT NULL DEFAULT 'manual',   -- manual, learned, imported
    confidence REAL NOT NULL DEFAULT 1.0,    -- 0.0-1.0 for learned instructions
    tags TEXT,                               -- JSON array of tags for organization
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    created_by TEXT,                         -- User or agent that created this
    CONSTRAINT valid_scope CHECK (scope IN ('global', 'agent_type')),
    CONSTRAINT valid_source CHECK (source IN ('manual', 'learned', 'imported')),
    CONSTRAINT agent_type_required CHECK (
        (scope = 'global' AND agent_type IS NULL) OR
        (scope = 'agent_type' AND agent_type IS NOT NULL)
    )
);

-- Instruction usage tracking
CREATE TABLE IF NOT EXISTS instruction_usage (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    instruction_id INTEGER NOT NULL,
    agent_id TEXT NOT NULL,
    session_id TEXT,
    applied_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (instruction_id) REFERENCES custom_instructions(id) ON DELETE CASCADE,
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Instruction effectiveness metrics with penalty scoring
CREATE TABLE IF NOT EXISTS instruction_effectiveness (
    instruction_id INTEGER PRIMARY KEY,
    usage_count INTEGER NOT NULL DEFAULT 0,
    success_count INTEGER NOT NULL DEFAULT 0,
    failure_count INTEGER NOT NULL DEFAULT 0,
    penalty_score REAL NOT NULL DEFAULT 0.0,  -- Accumulated penalty (0.0-2.0)
    avg_completion_time REAL,                  -- Average time to completion in seconds
    last_success_at TEXT,
    last_failure_at TEXT,
    last_penalty_at TEXT,                      -- When last penalty was applied
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (instruction_id) REFERENCES custom_instructions(id) ON DELETE CASCADE
);

-- Learning patterns for automatic instruction generation
CREATE TABLE IF NOT EXISTS learning_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern_type TEXT NOT NULL,              -- error_pattern, tool_usage_pattern, behavior_pattern
    agent_type TEXT,                         -- NULL for all types
    pattern_signature TEXT NOT NULL UNIQUE,  -- Hash or normalized form for deduplication
    pattern_data TEXT NOT NULL,              -- JSON with pattern details
    occurrence_count INTEGER NOT NULL DEFAULT 1,
    first_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_seen_at TEXT NOT NULL DEFAULT (datetime('now')),
    instruction_id INTEGER,                  -- NULL until instruction generated
    status TEXT NOT NULL DEFAULT 'observed', -- observed, pending_review, approved, rejected
    CONSTRAINT valid_status CHECK (status IN ('observed', 'pending_review', 'approved', 'rejected')),
    FOREIGN KEY (instruction_id) REFERENCES custom_instructions(id) ON DELETE SET NULL
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_instructions_scope ON custom_instructions(scope, agent_type);
CREATE INDEX IF NOT EXISTS idx_instructions_enabled ON custom_instructions(enabled, priority DESC);
CREATE INDEX IF NOT EXISTS idx_instructions_source ON custom_instructions(source);
CREATE INDEX IF NOT EXISTS idx_usage_instruction ON instruction_usage(instruction_id);
CREATE INDEX IF NOT EXISTS idx_usage_agent ON instruction_usage(agent_id);
CREATE INDEX IF NOT EXISTS idx_usage_applied ON instruction_usage(applied_at);
CREATE INDEX IF NOT EXISTS idx_effectiveness_penalty ON instruction_effectiveness(penalty_score);
CREATE INDEX IF NOT EXISTS idx_patterns_type ON learning_patterns(pattern_type, agent_type);
CREATE INDEX IF NOT EXISTS idx_patterns_status ON learning_patterns(status);
CREATE INDEX IF NOT EXISTS idx_patterns_signature ON learning_patterns(pattern_signature);
