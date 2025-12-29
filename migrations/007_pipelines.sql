-- Pipelines System
-- Migration for Epic 004 Story 1: Pipeline Data Model

-- Pipelines table - stores pipeline definitions
CREATE TABLE IF NOT EXISTS pipelines (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    definition TEXT NOT NULL,  -- YAML pipeline definition
    enabled INTEGER NOT NULL DEFAULT 1,  -- Boolean: 1=enabled, 0=disabled
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Pipeline runs table - stores execution instances
CREATE TABLE IF NOT EXISTS pipeline_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pipeline_id INTEGER NOT NULL REFERENCES pipelines(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'waiting_approval', 'succeeded', 'failed', 'cancelled')),
    trigger_event TEXT,  -- Event that triggered this run (e.g., "pull_request.merged")
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Pipeline stages table - stores individual stage executions within a run
CREATE TABLE IF NOT EXISTS pipeline_stages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    run_id INTEGER NOT NULL REFERENCES pipeline_runs(id) ON DELETE CASCADE,
    stage_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'waiting_approval', 'succeeded', 'failed', 'skipped', 'cancelled')),
    agent_id TEXT REFERENCES agents(id),
    started_at TEXT,
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_pipelines_name ON pipelines(name);
CREATE INDEX IF NOT EXISTS idx_pipelines_enabled ON pipelines(enabled);
CREATE INDEX IF NOT EXISTS idx_pipeline_runs_pipeline_id ON pipeline_runs(pipeline_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_runs_status ON pipeline_runs(status);
CREATE INDEX IF NOT EXISTS idx_pipeline_runs_created_at ON pipeline_runs(created_at);
CREATE INDEX IF NOT EXISTS idx_pipeline_stages_run_id ON pipeline_stages(run_id);
CREATE INDEX IF NOT EXISTS idx_pipeline_stages_status ON pipeline_stages(status);
CREATE INDEX IF NOT EXISTS idx_pipeline_stages_agent_id ON pipeline_stages(agent_id);
