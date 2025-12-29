-- Deployment Verification Tables
-- Migration: 015_deployment_verification
--
-- Stores post-deployment verification results including:
-- - Smoke test results
-- - Health endpoint checks
-- - Version verification
-- - Log error analysis
-- - Error rate monitoring
-- Supports rollback triggers based on verification failures

-- Deployment verification results table
CREATE TABLE IF NOT EXISTS deployment_verifications (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id INTEGER NOT NULL,
    overall_status TEXT NOT NULL,                  -- pending, running, passed, failed, skipped
    should_rollback INTEGER NOT NULL DEFAULT 0,    -- Boolean: 1 = rollback recommended, 0 = no rollback
    started_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (deployment_id) REFERENCES deployments(id) ON DELETE CASCADE,
    CONSTRAINT valid_status CHECK (overall_status IN ('pending', 'running', 'passed', 'failed', 'skipped'))
);

-- Verification checks table
CREATE TABLE IF NOT EXISTS verification_checks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    verification_id INTEGER NOT NULL,
    check_type TEXT NOT NULL,                      -- smoke_test, health_endpoint, version_check, log_error_check, error_rate_monitoring
    status TEXT NOT NULL,                          -- pending, running, passed, failed, skipped
    message TEXT NOT NULL,
    details TEXT,                                  -- Optional JSON details
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (verification_id) REFERENCES deployment_verifications(id) ON DELETE CASCADE,
    CONSTRAINT valid_check_type CHECK (check_type IN ('smoke_test', 'health_endpoint', 'version_check', 'log_error_check', 'error_rate_monitoring')),
    CONSTRAINT valid_check_status CHECK (status IN ('pending', 'running', 'passed', 'failed', 'skipped'))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_deployment_verifications_deployment_id ON deployment_verifications(deployment_id);
CREATE INDEX IF NOT EXISTS idx_deployment_verifications_status ON deployment_verifications(overall_status);
CREATE INDEX IF NOT EXISTS idx_deployment_verifications_started_at ON deployment_verifications(started_at DESC);

CREATE INDEX IF NOT EXISTS idx_verification_checks_verification_id ON verification_checks(verification_id);
CREATE INDEX IF NOT EXISTS idx_verification_checks_check_type ON verification_checks(check_type);
CREATE INDEX IF NOT EXISTS idx_verification_checks_status ON verification_checks(status);

-- Trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_deployment_verifications_updated_at
    AFTER UPDATE ON deployment_verifications
    FOR EACH ROW
BEGIN
    UPDATE deployment_verifications SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;
