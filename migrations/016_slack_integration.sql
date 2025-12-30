-- Slack Integration Tables
-- Migration 016: Epic 008 Stories 1-4 - Slack Integration
-- Implements:
--   Story 1: Slack App Configuration (OAuth, credentials, connection)
--   Story 2: Notification Service (channels, DMs, threads, templates)
--   Story 3: Agent Lifecycle Notifications
--   Story 4: PR Notifications

-- Story 1: Slack workspace connections and OAuth configuration
CREATE TABLE IF NOT EXISTS slack_connections (
    id TEXT PRIMARY KEY,
    team_id TEXT NOT NULL UNIQUE,
    team_name TEXT NOT NULL,
    bot_token TEXT NOT NULL,
    bot_user_id TEXT NOT NULL,
    app_id TEXT NOT NULL,
    connected_at TEXT NOT NULL DEFAULT (datetime('now')),
    connected_by TEXT NOT NULL,
    is_active INTEGER NOT NULL DEFAULT 1,
    scopes TEXT NOT NULL DEFAULT '[]',
    CONSTRAINT check_is_active CHECK (is_active IN (0, 1))
);

-- Story 2: Channel configurations for notification routing
CREATE TABLE IF NOT EXISTS slack_channel_configs (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL REFERENCES slack_connections(id) ON DELETE CASCADE,
    default_channel TEXT NOT NULL,
    channel_mappings TEXT NOT NULL DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Story 2: GitHub to Slack user mappings for direct messages
CREATE TABLE IF NOT EXISTS slack_user_mappings (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL REFERENCES slack_connections(id) ON DELETE CASCADE,
    github_username TEXT NOT NULL,
    slack_user_id TEXT NOT NULL,
    slack_username TEXT NOT NULL,
    notify_on_pr INTEGER NOT NULL DEFAULT 1,
    notify_on_mention INTEGER NOT NULL DEFAULT 1,
    notify_on_failure INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT check_notify_on_pr CHECK (notify_on_pr IN (0, 1)),
    CONSTRAINT check_notify_on_mention CHECK (notify_on_mention IN (0, 1)),
    CONSTRAINT check_notify_on_failure CHECK (notify_on_failure IN (0, 1)),
    UNIQUE (connection_id, github_username)
);

-- Story 4: PR thread tracking for threaded conversations
CREATE TABLE IF NOT EXISTS slack_pr_threads (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL REFERENCES slack_connections(id) ON DELETE CASCADE,
    pr_number INTEGER NOT NULL,
    channel_id TEXT NOT NULL,
    thread_ts TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    last_updated TEXT NOT NULL DEFAULT (datetime('now')),
    is_archived INTEGER NOT NULL DEFAULT 0,
    CONSTRAINT check_is_archived CHECK (is_archived IN (0, 1)),
    UNIQUE (connection_id, pr_number)
);

-- Story 2: User notification settings for verbosity control
CREATE TABLE IF NOT EXISTS slack_notification_settings (
    id TEXT PRIMARY KEY,
    user_mapping_id TEXT NOT NULL REFERENCES slack_user_mappings(id) ON DELETE CASCADE,
    enabled_types TEXT NOT NULL DEFAULT '[]',
    muted_until TEXT,
    dm_for_urgent INTEGER NOT NULL DEFAULT 1,
    digest_mode TEXT NOT NULL DEFAULT 'instant',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT check_dm_for_urgent CHECK (dm_for_urgent IN (0, 1)),
    CONSTRAINT check_digest_mode CHECK (digest_mode IN ('instant', 'hourly', 'daily')),
    UNIQUE (user_mapping_id)
);

-- Story 2: Notification templates for rich message formatting
CREATE TABLE IF NOT EXISTS slack_notification_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    notification_type TEXT NOT NULL,
    template_blocks TEXT NOT NULL,
    fallback_text TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (notification_type)
);

-- Story 2: Sent message tracking for rate limiting and threading
CREATE TABLE IF NOT EXISTS slack_sent_messages (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL REFERENCES slack_connections(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL,
    message_ts TEXT NOT NULL,
    thread_ts TEXT,
    notification_type TEXT,
    agent_id TEXT REFERENCES agents(id),
    pr_number INTEGER,
    sent_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (connection_id, channel_id, message_ts)
);

-- Slack approval requests tracking
CREATE TABLE IF NOT EXISTS slack_approval_requests (
    id TEXT PRIMARY KEY,
    approval_id INTEGER NOT NULL REFERENCES approval_requests(id) ON DELETE CASCADE,
    connection_id TEXT NOT NULL REFERENCES slack_connections(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL,
    message_ts TEXT NOT NULL,
    requester_slack_id TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    description TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    responded_at TEXT,
    responder_slack_id TEXT,
    decision TEXT,
    comment TEXT,
    CONSTRAINT check_decision CHECK (decision IS NULL OR decision IN ('approved', 'rejected', 'requested_changes'))
);

-- Story 2: Rate limiting tracking to prevent spam
CREATE TABLE IF NOT EXISTS slack_rate_limits (
    id TEXT PRIMARY KEY,
    connection_id TEXT NOT NULL REFERENCES slack_connections(id) ON DELETE CASCADE,
    channel_id TEXT NOT NULL,
    notification_type TEXT NOT NULL,
    message_count INTEGER NOT NULL DEFAULT 0,
    window_start TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (connection_id, channel_id, notification_type)
);

-- Story 1: Slack slash command audit log
CREATE TABLE IF NOT EXISTS slack_command_audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    connection_id TEXT NOT NULL REFERENCES slack_connections(id) ON DELETE CASCADE,
    command TEXT NOT NULL,
    user_id TEXT NOT NULL,
    user_name TEXT NOT NULL,
    channel_id TEXT NOT NULL,
    text TEXT,
    response_type TEXT NOT NULL,
    success INTEGER NOT NULL DEFAULT 1,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Story 8: Code owners for file patterns (user mention support)
CREATE TABLE IF NOT EXISTS slack_code_owners (
    id TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    github_username TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (pattern, github_username)
);

-- Indices for performance
CREATE INDEX IF NOT EXISTS idx_slack_connections_active ON slack_connections(is_active);
CREATE INDEX IF NOT EXISTS idx_slack_connections_team ON slack_connections(team_id);
CREATE INDEX IF NOT EXISTS idx_slack_channel_configs_connection ON slack_channel_configs(connection_id);
CREATE INDEX IF NOT EXISTS idx_slack_user_mappings_connection ON slack_user_mappings(connection_id);
CREATE INDEX IF NOT EXISTS idx_slack_user_mappings_github ON slack_user_mappings(github_username);
CREATE INDEX IF NOT EXISTS idx_slack_pr_threads_connection ON slack_pr_threads(connection_id);
CREATE INDEX IF NOT EXISTS idx_slack_pr_threads_pr ON slack_pr_threads(pr_number);
CREATE INDEX IF NOT EXISTS idx_slack_pr_threads_archived ON slack_pr_threads(is_archived);
CREATE INDEX IF NOT EXISTS idx_slack_notification_settings_user ON slack_notification_settings(user_mapping_id);
CREATE INDEX IF NOT EXISTS idx_slack_sent_messages_connection ON slack_sent_messages(connection_id);
CREATE INDEX IF NOT EXISTS idx_slack_sent_messages_agent ON slack_sent_messages(agent_id);
CREATE INDEX IF NOT EXISTS idx_slack_sent_messages_pr ON slack_sent_messages(pr_number);
CREATE INDEX IF NOT EXISTS idx_slack_approval_requests_approval ON slack_approval_requests(approval_id);
CREATE INDEX IF NOT EXISTS idx_slack_approval_requests_connection ON slack_approval_requests(connection_id);
CREATE INDEX IF NOT EXISTS idx_slack_approval_requests_decision ON slack_approval_requests(decision);
CREATE INDEX IF NOT EXISTS idx_slack_rate_limits_connection ON slack_rate_limits(connection_id);
CREATE INDEX IF NOT EXISTS idx_slack_command_audit_connection ON slack_command_audit(connection_id);
CREATE INDEX IF NOT EXISTS idx_slack_command_audit_user ON slack_command_audit(user_id);
CREATE INDEX IF NOT EXISTS idx_slack_command_audit_created ON slack_command_audit(created_at);
CREATE INDEX IF NOT EXISTS idx_slack_code_owners_pattern ON slack_code_owners(pattern);
CREATE INDEX IF NOT EXISTS idx_slack_code_owners_github ON slack_code_owners(github_username);
