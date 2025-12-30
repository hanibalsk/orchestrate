-- Rollback Database Optimization Migration
-- Epic 016: Autonomous Epic Processing
-- Reverses migration 027_add_foreign_keys_and_indexes.sql

-- =============================================================================
-- PART 1: DROP TRIGGERS
-- =============================================================================

DROP TRIGGER IF EXISTS update_edge_case_learnings_updated_at;
DROP TRIGGER IF EXISTS update_review_requests_updated_at;
DROP TRIGGER IF EXISTS update_autonomous_sessions_updated_at;

-- =============================================================================
-- PART 2: DROP COMPOSITE INDEXES
-- =============================================================================

-- Edge case learnings indexes
DROP INDEX IF EXISTS idx_edge_case_learnings_type_success;

-- Edge case events indexes
DROP INDEX IF EXISTS idx_edge_case_events_type_resolution;
DROP INDEX IF EXISTS idx_edge_case_events_session_resolution;

-- Review requests indexes
DROP INDEX IF EXISTS idx_review_requests_story_status;

-- Review iterations indexes
DROP INDEX IF EXISTS idx_review_iterations_story_iteration;

-- CI check results indexes
DROP INDEX IF EXISTS idx_ci_check_results_story_status;

-- Code review results indexes
DROP INDEX IF EXISTS idx_code_review_results_story_verdict;

-- Story evaluations indexes
DROP INDEX IF EXISTS idx_story_evaluations_session_story;
DROP INDEX IF EXISTS idx_story_evaluations_story_status;

-- Recovery attempts indexes
DROP INDEX IF EXISTS idx_recovery_attempts_session_outcome;
DROP INDEX IF EXISTS idx_recovery_attempts_stuck_detection;

-- Stuck agent detections indexes
DROP INDEX IF EXISTS idx_stuck_agent_detections_session_severity;
DROP INDEX IF EXISTS idx_stuck_agent_detections_agent_resolved;

-- Work evaluations indexes
DROP INDEX IF EXISTS idx_work_evaluations_story_type;
DROP INDEX IF EXISTS idx_work_evaluations_session_status;

-- Agent continuations indexes
DROP INDEX IF EXISTS idx_agent_continuations_session_status;
DROP INDEX IF EXISTS idx_agent_continuations_agent_status;

-- Autonomous sessions indexes
DROP INDEX IF EXISTS idx_autonomous_sessions_state_story;
DROP INDEX IF EXISTS idx_autonomous_sessions_state_epic;

-- Note: JSON validation and FK documentation are comments only and don't need rollback
