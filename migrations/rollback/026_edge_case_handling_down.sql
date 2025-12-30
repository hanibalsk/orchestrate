-- Rollback Edge Case Handling Schema
-- Epic 016: Autonomous Epic Processing - Story 14
-- Reverses migration 026_edge_case_handling.sql

-- Drop edge_case_learnings indexes
DROP INDEX IF EXISTS idx_edge_case_learnings_occurrence;
DROP INDEX IF EXISTS idx_edge_case_learnings_success_rate;
DROP INDEX IF EXISTS idx_edge_case_learnings_type;

-- Drop edge_case_events indexes
DROP INDEX IF EXISTS idx_edge_case_events_detected_at;
DROP INDEX IF EXISTS idx_edge_case_events_resolution;
DROP INDEX IF EXISTS idx_edge_case_events_type;
DROP INDEX IF EXISTS idx_edge_case_events_story_id;
DROP INDEX IF EXISTS idx_edge_case_events_agent_id;
DROP INDEX IF EXISTS idx_edge_case_events_session_id;

-- Drop tables
DROP TABLE IF EXISTS edge_case_learnings;
DROP TABLE IF EXISTS edge_case_events;
