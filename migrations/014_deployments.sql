-- Deployment Tracking Tables
-- Migration: 014_deployments
--
-- Stores deployment history, progress tracking, and deployment records.
-- Supports multiple providers (Docker, ECS, Lambda, K8s, Vercel, etc.)
-- with timeout handling and validation results.

-- Deployments table
CREATE TABLE IF NOT EXISTS deployments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    environment_id INTEGER NOT NULL,
    environment_name TEXT NOT NULL,
    version TEXT NOT NULL,                     -- Version being deployed
    provider TEXT NOT NULL,                    -- docker, aws_ecs, aws_lambda, kubernetes, etc.
    strategy TEXT,                             -- JSON deployment strategy configuration
    status TEXT NOT NULL,                      -- pending, validating, in_progress, completed, failed, rolled_back, timed_out
    error_message TEXT,                        -- Error message if deployment failed
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    timeout_seconds INTEGER NOT NULL DEFAULT 1800, -- Deployment timeout (default 30 minutes)
    validation_result TEXT,                    -- JSON validation result
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (environment_id) REFERENCES environments(id) ON DELETE CASCADE,
    CONSTRAINT valid_status CHECK (status IN ('pending', 'validating', 'in_progress', 'completed', 'failed', 'rolled_back', 'timed_out'))
);

-- Deployment progress events table
CREATE TABLE IF NOT EXISTS deployment_progress (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id INTEGER NOT NULL,
    status TEXT NOT NULL,                      -- Current status
    message TEXT NOT NULL,                     -- Progress message
    progress_percent INTEGER NOT NULL DEFAULT 0, -- Progress percentage (0-100)
    details TEXT,                              -- Optional JSON details
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (deployment_id) REFERENCES deployments(id) ON DELETE CASCADE,
    CONSTRAINT valid_progress CHECK (progress_percent >= 0 AND progress_percent <= 100)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_deployments_environment_id ON deployments(environment_id);
CREATE INDEX IF NOT EXISTS idx_deployments_environment_name ON deployments(environment_name);
CREATE INDEX IF NOT EXISTS idx_deployments_status ON deployments(status);
CREATE INDEX IF NOT EXISTS idx_deployments_started_at ON deployments(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_deployments_version ON deployments(version);

CREATE INDEX IF NOT EXISTS idx_deployment_progress_deployment_id ON deployment_progress(deployment_id);
CREATE INDEX IF NOT EXISTS idx_deployment_progress_created_at ON deployment_progress(created_at DESC);

-- Trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_deployments_updated_at
    AFTER UPDATE ON deployments
    FOR EACH ROW
BEGIN
    UPDATE deployments SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;
