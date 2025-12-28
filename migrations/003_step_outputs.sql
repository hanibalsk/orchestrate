-- Step Outputs Table
-- Migration: 003_step_outputs
--
-- Stores outputs from workflow steps that can be consumed by dependent agents.
-- Enables data flow between agents via database persistence.

-- Step outputs from agent skill executions
CREATE TABLE IF NOT EXISTS step_outputs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    output_type TEXT NOT NULL,          -- skill_result, state_transition, artifact, error
    data TEXT NOT NULL,                 -- JSON payload
    consumed INTEGER NOT NULL DEFAULT 0,
    consumed_by TEXT,                   -- Agent ID that consumed this output
    consumed_at TEXT,                   -- When the output was consumed
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Index for finding outputs by agent
CREATE INDEX IF NOT EXISTS idx_step_outputs_agent ON step_outputs(agent_id);

-- Index for finding unconsumed outputs (for dependency queries)
CREATE INDEX IF NOT EXISTS idx_step_outputs_unconsumed ON step_outputs(consumed, agent_id);

-- Index for finding outputs by skill
CREATE INDEX IF NOT EXISTS idx_step_outputs_skill ON step_outputs(skill_name);

-- Index for finding outputs by type
CREATE INDEX IF NOT EXISTS idx_step_outputs_type ON step_outputs(output_type);

-- Composite index for common query pattern: get unconsumed outputs for specific agents
CREATE INDEX IF NOT EXISTS idx_step_outputs_agent_unconsumed ON step_outputs(agent_id, consumed);
