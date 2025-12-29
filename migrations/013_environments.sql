-- Environment Configuration Tables
-- Migration: 013_environments
--
-- Stores deployment environment configurations with encrypted secrets.
-- Supports multiple environment types (dev/staging/production) with
-- environment-specific variables and secrets management.

-- Environments table
CREATE TABLE IF NOT EXISTS environments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,                 -- Environment name (e.g., "staging", "production")
    type TEXT NOT NULL,                        -- development, staging, production
    url TEXT,                                  -- Environment URL (optional)
    provider TEXT,                             -- Deployment provider (aws, k8s, etc)
    config TEXT NOT NULL DEFAULT '{}',         -- JSON configuration (non-sensitive)
    secrets TEXT NOT NULL DEFAULT '{}',        -- Encrypted JSON secrets
    requires_approval INTEGER NOT NULL DEFAULT 0, -- Boolean: require approval for deployments
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT valid_type CHECK (type IN ('development', 'staging', 'production'))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_environments_name ON environments(name);
CREATE INDEX IF NOT EXISTS idx_environments_type ON environments(type);
CREATE INDEX IF NOT EXISTS idx_environments_created_at ON environments(created_at DESC);

-- Trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_environments_updated_at
    AFTER UPDATE ON environments
    FOR EACH ROW
BEGIN
    UPDATE environments SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;
