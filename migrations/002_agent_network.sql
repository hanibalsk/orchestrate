-- Agent Network Tables
-- Migration: 002_agent_network
--
-- Adds tables for agent network relationships, state transitions,
-- and network health monitoring.

-- Agent dependencies
-- Tracks which agents depend on which other agents
CREATE TABLE IF NOT EXISTS agent_dependencies (
    agent_id TEXT NOT NULL,
    depends_on_id TEXT NOT NULL,
    dependency_type TEXT NOT NULL DEFAULT 'required',  -- required, optional
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (agent_id, depends_on_id),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE,
    FOREIGN KEY (depends_on_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Index for reverse lookups (who depends on this agent?)
CREATE INDEX IF NOT EXISTS idx_agent_deps_depends_on ON agent_dependencies(depends_on_id);

-- State transition log
-- Tracks all state changes for auditing and debugging
CREATE TABLE IF NOT EXISTS state_transitions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    from_state TEXT NOT NULL,
    to_state TEXT NOT NULL,
    trigger TEXT,                       -- What triggered this transition
    dependency_states TEXT,             -- JSON snapshot of dependency states at transition time
    success INTEGER NOT NULL DEFAULT 1, -- Whether transition succeeded
    error_message TEXT,                 -- Error message if failed
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Index for querying transitions by agent
CREATE INDEX IF NOT EXISTS idx_state_transitions_agent ON state_transitions(agent_id);

-- Index for querying transitions by time
CREATE INDEX IF NOT EXISTS idx_state_transitions_time ON state_transitions(created_at);

-- Skill executions
-- Tracks skill invocations and their results
CREATE TABLE IF NOT EXISTS skill_executions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,
    skill_name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending', -- pending, executing, completed, failed, cancelled, timed_out
    started_at TEXT,
    ended_at TEXT,
    result TEXT,                        -- Result or error message
    dependency_snapshot TEXT,           -- JSON snapshot of dependency states at start
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Index for querying executions by agent
CREATE INDEX IF NOT EXISTS idx_skill_executions_agent ON skill_executions(agent_id);

-- Index for querying active executions
CREATE INDEX IF NOT EXISTS idx_skill_executions_status ON skill_executions(status);

-- Network health snapshots
-- Periodic snapshots of network state for monitoring
CREATE TABLE IF NOT EXISTS network_snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    snapshot_time TEXT NOT NULL DEFAULT (datetime('now')),
    total_agents INTEGER NOT NULL DEFAULT 0,
    agents_by_state TEXT NOT NULL,      -- JSON: {"running": 5, "paused": 2, ...}
    agents_by_type TEXT NOT NULL,       -- JSON: {"story_developer": 3, ...}
    validation_result TEXT NOT NULL,    -- valid, invalid
    error_count INTEGER NOT NULL DEFAULT 0,
    warning_count INTEGER NOT NULL DEFAULT 0,
    issues TEXT,                        -- JSON array of validation issues
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for querying snapshots by time
CREATE INDEX IF NOT EXISTS idx_network_snapshots_time ON network_snapshots(snapshot_time);

-- Recovery actions
-- Tracks self-healing actions taken by the network coordinator
CREATE TABLE IF NOT EXISTS recovery_actions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT,
    action_type TEXT NOT NULL,          -- restart, pause, terminate, spawn_dependency, retry
    reason TEXT NOT NULL,
    target_agent_type TEXT,             -- For spawn_dependency
    target_state TEXT,                  -- For retry
    success INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (agent_id) REFERENCES agents(id) ON DELETE SET NULL
);

-- Index for querying recovery actions by agent
CREATE INDEX IF NOT EXISTS idx_recovery_actions_agent ON recovery_actions(agent_id);

-- Index for querying recovery actions by time
CREATE INDEX IF NOT EXISTS idx_recovery_actions_time ON recovery_actions(created_at);

-- Propagation events
-- Tracks state propagation through the network
CREATE TABLE IF NOT EXISTS propagation_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_agent_id TEXT NOT NULL,
    target_agent_id TEXT NOT NULL,
    event_type TEXT NOT NULL,           -- dependency_ready, dependency_completed, dependency_failed, etc.
    source_state TEXT NOT NULL,
    processed INTEGER NOT NULL DEFAULT 0,
    processed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (source_agent_id) REFERENCES agents(id) ON DELETE CASCADE,
    FOREIGN KEY (target_agent_id) REFERENCES agents(id) ON DELETE CASCADE
);

-- Index for querying unprocessed events
CREATE INDEX IF NOT EXISTS idx_propagation_unprocessed ON propagation_events(processed, target_agent_id);

-- Index for querying events by source
CREATE INDEX IF NOT EXISTS idx_propagation_source ON propagation_events(source_agent_id);
