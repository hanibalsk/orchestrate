-- Alerting Rules Engine
-- Migration for Epic 007 Story 3: Alerting Rules Engine

-- Alert rules table - defines alerting conditions and thresholds
CREATE TABLE IF NOT EXISTS alert_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    condition TEXT NOT NULL,  -- Alert condition expression (e.g., "rate(orchestrate_agent_failures_total[5m]) > 0.2")
    severity TEXT NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    channels TEXT NOT NULL,  -- JSON array of notification channels (e.g., ["slack", "email"])
    enabled INTEGER NOT NULL DEFAULT 1,  -- 1=enabled, 0=disabled
    evaluation_interval_seconds INTEGER NOT NULL DEFAULT 60,  -- How often to evaluate the rule
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Alerts table - tracks triggered alerts
CREATE TABLE IF NOT EXISTS alerts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id INTEGER NOT NULL REFERENCES alert_rules(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'acknowledged', 'resolved')),
    triggered_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    resolved_at TEXT,
    acknowledged_at TEXT,
    acknowledged_by TEXT,
    trigger_value TEXT,  -- JSON with the metric value that triggered the alert
    metadata TEXT,  -- JSON with additional context about the alert
    fingerprint TEXT NOT NULL,  -- Deduplication key (hash of rule + conditions)
    last_notified_at TEXT,  -- When was the last notification sent
    notification_count INTEGER NOT NULL DEFAULT 0  -- How many times notifications were sent
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_alert_rules_name ON alert_rules(name);
CREATE INDEX IF NOT EXISTS idx_alert_rules_enabled ON alert_rules(enabled);
CREATE INDEX IF NOT EXISTS idx_alerts_rule_id ON alerts(rule_id);
CREATE INDEX IF NOT EXISTS idx_alerts_status ON alerts(status);
CREATE INDEX IF NOT EXISTS idx_alerts_fingerprint ON alerts(fingerprint);
CREATE INDEX IF NOT EXISTS idx_alerts_triggered_at ON alerts(triggered_at);
CREATE INDEX IF NOT EXISTS idx_alerts_resolved_at ON alerts(resolved_at);
