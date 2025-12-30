-- Feature Flags Table
-- Migration: 017_feature_flags
--
-- Stores feature flags with support for:
-- - Global and environment-specific flags
-- - Gradual rollout via percentage
-- - Integration metadata for external providers

CREATE TABLE IF NOT EXISTS feature_flags (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    key TEXT NOT NULL,                         -- Flag key (unique per environment)
    name TEXT NOT NULL,                        -- Human-readable name
    description TEXT,                          -- Optional description
    status TEXT NOT NULL,                      -- enabled, disabled, conditional
    rollout_percentage INTEGER NOT NULL DEFAULT 100, -- 0-100 for gradual rollout
    environment TEXT,                          -- NULL for global, or environment name
    metadata TEXT,                             -- JSON metadata for external integrations
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT valid_status CHECK (status IN ('enabled', 'disabled', 'conditional')),
    CONSTRAINT valid_percentage CHECK (rollout_percentage >= 0 AND rollout_percentage <= 100),
    CONSTRAINT unique_flag UNIQUE (key, environment)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_feature_flags_key ON feature_flags(key);
CREATE INDEX IF NOT EXISTS idx_feature_flags_environment ON feature_flags(environment);
CREATE INDEX IF NOT EXISTS idx_feature_flags_status ON feature_flags(status);

-- Trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_feature_flags_updated_at
    AFTER UPDATE ON feature_flags
    FOR EACH ROW
BEGIN
    UPDATE feature_flags SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;
