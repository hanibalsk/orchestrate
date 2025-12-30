-- Orchestrate Database Schema v1
-- Initial migration

-- Agents table
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    agent_type TEXT NOT NULL,
    state TEXT NOT NULL DEFAULT 'created',
    task TEXT NOT NULL,
    context TEXT DEFAULT '{}',
    session_id TEXT,
    parent_agent_id TEXT REFERENCES agents(id),
    worktree_id TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Agent messages
CREATE TABLE IF NOT EXISTS agent_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system', 'tool')),
    content TEXT NOT NULL,
    tool_calls TEXT,
    tool_results TEXT,
    input_tokens INTEGER DEFAULT 0,
    output_tokens INTEGER DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Sessions
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL REFERENCES agents(id) ON DELETE CASCADE,
    parent_id TEXT REFERENCES sessions(id),
    api_session_id TEXT,
    total_tokens INTEGER DEFAULT 0,
    is_forked INTEGER DEFAULT 0,
    forked_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    closed_at TEXT
);

-- PR Queue
CREATE TABLE IF NOT EXISTS pr_queue (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    epic_id TEXT,
    worktree_id TEXT,
    branch_name TEXT NOT NULL,
    title TEXT,
    body TEXT,
    pr_number INTEGER,
    status TEXT NOT NULL DEFAULT 'queued',
    merge_strategy TEXT DEFAULT 'squash',
    agent_id TEXT REFERENCES agents(id),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    merged_at TEXT
);

-- Worktrees
CREATE TABLE IF NOT EXISTS worktrees (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    path TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    base_branch TEXT NOT NULL DEFAULT 'main',
    status TEXT NOT NULL DEFAULT 'active',
    agent_id TEXT REFERENCES agents(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    removed_at TEXT
);

-- Epics (BMAD)
CREATE TABLE IF NOT EXISTS epics (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    source_file TEXT,
    pattern TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    current_phase TEXT,
    agent_id TEXT REFERENCES agents(id),
    pr_id INTEGER REFERENCES pr_queue(id),
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Stories (BMAD)
CREATE TABLE IF NOT EXISTS stories (
    id TEXT PRIMARY KEY,
    epic_id TEXT NOT NULL REFERENCES epics(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT,
    acceptance_criteria TEXT,
    status TEXT NOT NULL DEFAULT 'pending',
    agent_id TEXT REFERENCES agents(id),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT
);

-- Audit log is now created by migration 019_audit_log.sql
-- (Old audit_log table was replaced with enhanced version)

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_agents_state ON agents(state);
CREATE INDEX IF NOT EXISTS idx_agents_type ON agents(agent_type);
CREATE INDEX IF NOT EXISTS idx_agents_session ON agents(session_id);
CREATE INDEX IF NOT EXISTS idx_messages_agent ON agent_messages(agent_id);
CREATE INDEX IF NOT EXISTS idx_sessions_agent ON sessions(agent_id);
CREATE INDEX IF NOT EXISTS idx_sessions_parent ON sessions(parent_id);
CREATE INDEX IF NOT EXISTS idx_pr_queue_status ON pr_queue(status);
CREATE INDEX IF NOT EXISTS idx_pr_queue_pr_number ON pr_queue(pr_number);
CREATE INDEX IF NOT EXISTS idx_epics_status ON epics(status);
CREATE INDEX IF NOT EXISTS idx_stories_epic ON stories(epic_id);
CREATE INDEX IF NOT EXISTS idx_stories_status ON stories(status);
CREATE INDEX IF NOT EXISTS idx_worktrees_status ON worktrees(status);
CREATE INDEX IF NOT EXISTS idx_worktrees_agent ON worktrees(agent_id);
