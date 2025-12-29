-- Webhook Events Queue
-- Migration for Epic 002 Story 2: Event Queue System

-- Webhook events table for queueing incoming webhooks
CREATE TABLE IF NOT EXISTS webhook_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    delivery_id TEXT NOT NULL UNIQUE,  -- GitHub X-GitHub-Delivery header for idempotency
    event_type TEXT NOT NULL,          -- GitHub event type (pull_request, check_run, etc.)
    payload TEXT NOT NULL,             -- JSON payload from GitHub
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'processing', 'completed', 'failed', 'dead_letter')),
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 3,
    error_message TEXT,
    next_retry_at TEXT,                -- Next retry attempt timestamp
    received_at TEXT NOT NULL DEFAULT (datetime('now')),
    processed_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_webhook_events_delivery_id ON webhook_events(delivery_id);
CREATE INDEX IF NOT EXISTS idx_webhook_events_status ON webhook_events(status);
CREATE INDEX IF NOT EXISTS idx_webhook_events_event_type ON webhook_events(event_type);
CREATE INDEX IF NOT EXISTS idx_webhook_events_next_retry ON webhook_events(next_retry_at) WHERE status = 'pending';
CREATE INDEX IF NOT EXISTS idx_webhook_events_received_at ON webhook_events(received_at);
