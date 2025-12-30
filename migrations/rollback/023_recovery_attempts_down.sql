-- Rollback Recovery Attempts Schema
-- Epic 016: Autonomous Epic Processing - Story 7
-- Reverses migration 023_recovery_attempts.sql

-- Drop indexes first
DROP INDEX IF EXISTS idx_recovery_attempts_started_at;
DROP INDEX IF EXISTS idx_recovery_attempts_action_type;
DROP INDEX IF EXISTS idx_recovery_attempts_outcome;
DROP INDEX IF EXISTS idx_recovery_attempts_agent_id;

-- Drop table
DROP TABLE IF EXISTS recovery_attempts;
