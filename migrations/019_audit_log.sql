-- Audit Log Enhancement
-- Migration: 019_audit_log
--
-- Enhances the existing audit_log table with comprehensive auditing capabilities
-- including actor tracking, IP addresses, and structured details.

-- Drop the old indexes first (they reference old columns)
DROP INDEX IF EXISTS idx_audit_entity;

-- Drop the old simple audit_log table if it exists
DROP TABLE IF EXISTS audit_log;

-- Create the comprehensive audit log table
CREATE TABLE IF NOT EXISTS audit_log (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    actor TEXT NOT NULL,
    actor_type TEXT NOT NULL CHECK(actor_type IN ('user', 'system', 'agent', 'api_key', 'webhook')),
    action TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    details TEXT NOT NULL,  -- JSON object with additional context
    ip_address TEXT,
    user_agent TEXT,
    success BOOLEAN NOT NULL DEFAULT 1,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_log_actor ON audit_log(actor);
CREATE INDEX IF NOT EXISTS idx_audit_log_actor_type ON audit_log(actor_type);
CREATE INDEX IF NOT EXISTS idx_audit_log_action ON audit_log(action);
CREATE INDEX IF NOT EXISTS idx_audit_log_resource ON audit_log(resource_type, resource_id);
CREATE INDEX IF NOT EXISTS idx_audit_log_success ON audit_log(success);
CREATE INDEX IF NOT EXISTS idx_audit_log_timestamp_action ON audit_log(timestamp, action);
CREATE INDEX IF NOT EXISTS idx_audit_log_actor_timestamp ON audit_log(actor, timestamp);
