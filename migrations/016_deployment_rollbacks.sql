-- Deployment Rollback Events
-- Migration: 016_deployment_rollbacks
--
-- Stores deployment rollback history and events.
-- Supports rollback to previous version, specific version, and fast blue-green switches.

-- Deployment rollback events table
CREATE TABLE IF NOT EXISTS deployment_rollback_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id INTEGER NOT NULL,            -- Deployment being rolled back from
    target_version TEXT NOT NULL,              -- Version to rollback to
    rollback_type TEXT NOT NULL,               -- previous, specific, blue_green_switch, automatic
    status TEXT NOT NULL DEFAULT 'pending',    -- pending, in_progress, completed, failed
    error_message TEXT,                        -- Error message if rollback failed
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    notification_sent INTEGER NOT NULL DEFAULT 0, -- 0 = false, 1 = true
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (deployment_id) REFERENCES deployments(id) ON DELETE CASCADE,
    CONSTRAINT valid_rollback_type CHECK (rollback_type IN ('previous', 'specific', 'blue_green_switch', 'automatic')),
    CONSTRAINT valid_status CHECK (status IN ('pending', 'in_progress', 'completed', 'failed')),
    CONSTRAINT valid_notification_sent CHECK (notification_sent IN (0, 1))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_deployment_rollback_events_deployment_id ON deployment_rollback_events(deployment_id);
CREATE INDEX IF NOT EXISTS idx_deployment_rollback_events_status ON deployment_rollback_events(status);
CREATE INDEX IF NOT EXISTS idx_deployment_rollback_events_started_at ON deployment_rollback_events(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_deployment_rollback_events_notification_sent ON deployment_rollback_events(notification_sent);

-- Trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_deployment_rollback_events_updated_at
    AFTER UPDATE ON deployment_rollback_events
    FOR EACH ROW
BEGIN
    UPDATE deployment_rollback_events SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;
