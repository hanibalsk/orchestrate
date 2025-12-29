-- Feedback Tables
-- Migration: 012_feedback
--
-- Stores user feedback on agent outputs to enable closed-loop learning.
-- Feedback can be linked to specific agents and optionally to specific messages.

-- Feedback table
CREATE TABLE IF NOT EXISTS feedback (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT NOT NULL,                    -- UUID of the agent
    message_id INTEGER,                        -- Optional: specific message being rated
    rating TEXT NOT NULL,                      -- positive, negative, neutral
    comment TEXT,                              -- Optional comment explaining the rating
    source TEXT NOT NULL DEFAULT 'cli',        -- cli, web, slack, api, automated
    created_by TEXT NOT NULL,                  -- Who provided the feedback
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    CONSTRAINT valid_rating CHECK (rating IN ('positive', 'negative', 'neutral')),
    CONSTRAINT valid_source CHECK (source IN ('cli', 'web', 'slack', 'api', 'automated'))
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_feedback_agent_id ON feedback(agent_id);
CREATE INDEX IF NOT EXISTS idx_feedback_rating ON feedback(rating);
CREATE INDEX IF NOT EXISTS idx_feedback_source ON feedback(source);
CREATE INDEX IF NOT EXISTS idx_feedback_created_at ON feedback(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_feedback_message_id ON feedback(message_id);
