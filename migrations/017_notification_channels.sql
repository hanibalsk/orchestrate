-- Notification channels for alerting

-- Channel configurations
CREATE TABLE IF NOT EXISTS notification_channels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    channel_type TEXT NOT NULL CHECK(channel_type IN ('slack', 'email', 'pagerduty', 'webhook')),
    enabled INTEGER NOT NULL DEFAULT 1,
    rate_limit_per_hour INTEGER NOT NULL DEFAULT 60,
    config TEXT NOT NULL, -- JSON configuration for the channel
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE INDEX IF NOT EXISTS idx_notification_channels_name ON notification_channels(name);
CREATE INDEX IF NOT EXISTS idx_notification_channels_enabled ON notification_channels(enabled);
CREATE INDEX IF NOT EXISTS idx_notification_channels_type ON notification_channels(channel_type);

-- Message templates for formatting notifications per channel
CREATE TABLE IF NOT EXISTS notification_templates (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    channel_type TEXT NOT NULL CHECK(channel_type IN ('slack', 'email', 'pagerduty', 'webhook')),
    severity TEXT NOT NULL CHECK(severity IN ('info', 'warning', 'critical')),
    template TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    UNIQUE(channel_type, severity)
);

CREATE INDEX IF NOT EXISTS idx_notification_templates_channel_severity
    ON notification_templates(channel_type, severity);

-- Notification log to track sent notifications
CREATE TABLE IF NOT EXISTS notification_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    alert_id INTEGER NOT NULL,
    channel_id INTEGER NOT NULL,
    status TEXT NOT NULL CHECK(status IN ('pending', 'sent', 'failed')),
    error_message TEXT,
    sent_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    FOREIGN KEY (alert_id) REFERENCES alerts(id) ON DELETE CASCADE,
    FOREIGN KEY (channel_id) REFERENCES notification_channels(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_notification_log_alert ON notification_log(alert_id);
CREATE INDEX IF NOT EXISTS idx_notification_log_channel ON notification_log(channel_id);
CREATE INDEX IF NOT EXISTS idx_notification_log_status ON notification_log(status);
CREATE INDEX IF NOT EXISTS idx_notification_log_sent_at ON notification_log(sent_at);
