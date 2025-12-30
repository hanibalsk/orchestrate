-- Incident Response Schema
-- Epic 015: Autonomous Incident Response

-- Incidents table - track incidents through lifecycle
CREATE TABLE IF NOT EXISTS incidents (
    id TEXT PRIMARY KEY,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    severity TEXT NOT NULL CHECK(severity IN ('critical', 'high', 'medium', 'low')),
    status TEXT NOT NULL CHECK(status IN ('detected', 'investigating', 'mitigating', 'resolved', 'post_mortem')),
    detected_at TEXT NOT NULL,
    acknowledged_at TEXT,
    resolved_at TEXT,
    affected_services TEXT NOT NULL DEFAULT '[]',  -- JSON array
    related_incidents TEXT NOT NULL DEFAULT '[]',  -- JSON array
    tags TEXT NOT NULL DEFAULT '[]',               -- JSON array
    metadata TEXT NOT NULL DEFAULT '{}',           -- JSON object
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_incidents_status ON incidents(status);
CREATE INDEX IF NOT EXISTS idx_incidents_severity ON incidents(severity);
CREATE INDEX IF NOT EXISTS idx_incidents_detected_at ON incidents(detected_at);

-- Incident timeline - track events during incident lifecycle
CREATE TABLE IF NOT EXISTS incident_timeline (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    incident_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK(event_type IN (
        'detected', 'acknowledged', 'investigation_started', 'root_cause_identified',
        'mitigation_started', 'playbook_executed', 'escalated', 'resolved',
        'post_mortem_created', 'comment'
    )),
    description TEXT NOT NULL,
    actor TEXT,  -- Agent or user who triggered the event
    metadata TEXT NOT NULL DEFAULT '{}',  -- JSON object
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_incident_timeline_incident_id ON incident_timeline(incident_id);
CREATE INDEX IF NOT EXISTS idx_incident_timeline_timestamp ON incident_timeline(timestamp);

-- Root cause analyses
CREATE TABLE IF NOT EXISTS root_cause_analyses (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    incident_id TEXT NOT NULL UNIQUE,
    primary_cause TEXT NOT NULL DEFAULT '',
    evidence TEXT NOT NULL DEFAULT '[]',          -- JSON array of Evidence
    contributing_factors TEXT NOT NULL DEFAULT '[]',  -- JSON array
    hypotheses TEXT NOT NULL DEFAULT '[]',        -- JSON array of Hypothesis
    related_events TEXT NOT NULL DEFAULT '[]',    -- JSON array of RelatedEvent
    analyzed_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_rca_incident_id ON root_cause_analyses(incident_id);

-- Playbooks - remediation action templates
CREATE TABLE IF NOT EXISTS playbooks (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    triggers TEXT NOT NULL DEFAULT '[]',  -- JSON array of PlaybookTrigger
    actions TEXT NOT NULL DEFAULT '[]',   -- JSON array of PlaybookAction
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_playbooks_name ON playbooks(name);

-- Playbook executions
CREATE TABLE IF NOT EXISTS playbook_executions (
    id TEXT PRIMARY KEY,
    playbook_id TEXT NOT NULL,
    incident_id TEXT,
    status TEXT NOT NULL CHECK(status IN ('running', 'waiting_approval', 'completed', 'failed', 'cancelled')),
    started_at TEXT NOT NULL,
    completed_at TEXT,
    action_results TEXT NOT NULL DEFAULT '[]',  -- JSON array of ActionResult
    triggered_by TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (playbook_id) REFERENCES playbooks(id) ON DELETE CASCADE,
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_playbook_executions_playbook_id ON playbook_executions(playbook_id);
CREATE INDEX IF NOT EXISTS idx_playbook_executions_incident_id ON playbook_executions(incident_id);
CREATE INDEX IF NOT EXISTS idx_playbook_executions_status ON playbook_executions(status);

-- Escalation rules
CREATE TABLE IF NOT EXISTS escalation_rules (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    condition TEXT NOT NULL,  -- JSON EscalationCondition
    targets TEXT NOT NULL DEFAULT '[]',  -- JSON array of EscalationTarget
    delay_seconds INTEGER NOT NULL DEFAULT 0,
    repeat_interval_seconds INTEGER,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_escalation_rules_enabled ON escalation_rules(enabled);

-- Escalation events
CREATE TABLE IF NOT EXISTS escalation_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    incident_id TEXT NOT NULL,
    rule_id INTEGER NOT NULL,
    target_type TEXT NOT NULL CHECK(target_type IN ('slack', 'pagerduty', 'email', 'webhook')),
    destination TEXT NOT NULL,
    sent_at TEXT NOT NULL,
    acknowledged_at TEXT,
    metadata TEXT NOT NULL DEFAULT '{}',  -- JSON object
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE,
    FOREIGN KEY (rule_id) REFERENCES escalation_rules(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_escalation_events_incident_id ON escalation_events(incident_id);
CREATE INDEX IF NOT EXISTS idx_escalation_events_rule_id ON escalation_events(rule_id);

-- Post-mortems
CREATE TABLE IF NOT EXISTS post_mortems (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    incident_id TEXT NOT NULL UNIQUE,
    title TEXT NOT NULL,
    summary TEXT NOT NULL DEFAULT '',
    impact TEXT NOT NULL DEFAULT '{}',  -- JSON IncidentImpact
    root_cause TEXT NOT NULL DEFAULT '',
    contributing_factors TEXT NOT NULL DEFAULT '[]',  -- JSON array
    resolution TEXT NOT NULL DEFAULT '',
    action_items TEXT NOT NULL DEFAULT '[]',  -- JSON array of ActionItem
    lessons_learned TEXT NOT NULL DEFAULT '[]',  -- JSON array
    authors TEXT NOT NULL DEFAULT '[]',  -- JSON array
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_post_mortems_incident_id ON post_mortems(incident_id);

-- Anomaly detection metrics (for tracking anomalies over time)
CREATE TABLE IF NOT EXISTS anomaly_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    current_value REAL NOT NULL,
    baseline_value REAL NOT NULL,
    threshold REAL NOT NULL,
    deviation_percent REAL NOT NULL,
    is_anomaly INTEGER NOT NULL,
    incident_id TEXT,  -- NULL if no incident created
    timestamp TEXT NOT NULL,
    metadata TEXT NOT NULL DEFAULT '{}',  -- JSON object
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (incident_id) REFERENCES incidents(id) ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_anomaly_metrics_name ON anomaly_metrics(name);
CREATE INDEX IF NOT EXISTS idx_anomaly_metrics_timestamp ON anomaly_metrics(timestamp);
CREATE INDEX IF NOT EXISTS idx_anomaly_metrics_is_anomaly ON anomaly_metrics(is_anomaly);
