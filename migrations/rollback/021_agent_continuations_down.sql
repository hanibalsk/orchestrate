-- Rollback Agent Continuations Schema
-- Epic 016: Autonomous Epic Processing - Story 3
-- Reverses migration 021_agent_continuations.sql

-- Drop indexes first
DROP INDEX IF EXISTS idx_agent_continuations_created_at;
DROP INDEX IF EXISTS idx_agent_continuations_status;
DROP INDEX IF EXISTS idx_agent_continuations_agent_id;

-- Drop table
DROP TABLE IF EXISTS agent_continuations;
