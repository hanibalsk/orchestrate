-- Rollback Autonomous Session Management Schema
-- Epic 016: Autonomous Epic Processing - Story 1
-- Reverses migration 020_autonomous_sessions.sql

-- Drop indexes first
DROP INDEX IF EXISTS idx_autonomous_session_history_transitioned_at;
DROP INDEX IF EXISTS idx_autonomous_session_history_session_id;
DROP INDEX IF EXISTS idx_autonomous_sessions_started_at;
DROP INDEX IF EXISTS idx_autonomous_sessions_current_epic_id;
DROP INDEX IF EXISTS idx_autonomous_sessions_state;

-- Drop tables in reverse order (child tables first)
DROP TABLE IF EXISTS autonomous_session_history;
DROP TABLE IF EXISTS autonomous_sessions;
