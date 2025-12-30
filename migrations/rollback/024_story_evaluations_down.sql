-- Rollback Story Evaluations Schema
-- Epic 016: Autonomous Epic Processing - Story 8
-- Reverses migration 024_story_evaluations.sql

-- Drop ci_check_results indexes
DROP INDEX IF EXISTS idx_ci_check_results_checked_at;
DROP INDEX IF EXISTS idx_ci_check_results_status;
DROP INDEX IF EXISTS idx_ci_check_results_agent_id;
DROP INDEX IF EXISTS idx_ci_check_results_story_id;

-- Drop code_review_results indexes
DROP INDEX IF EXISTS idx_code_review_results_reviewed_at;
DROP INDEX IF EXISTS idx_code_review_results_verdict;
DROP INDEX IF EXISTS idx_code_review_results_story_id;

-- Drop story_evaluations indexes
DROP INDEX IF EXISTS idx_story_evaluations_evaluated_at;
DROP INDEX IF EXISTS idx_story_evaluations_status;
DROP INDEX IF EXISTS idx_story_evaluations_session_id;
DROP INDEX IF EXISTS idx_story_evaluations_agent_id;
DROP INDEX IF EXISTS idx_story_evaluations_story_id;

-- Drop tables
DROP TABLE IF EXISTS ci_check_results;
DROP TABLE IF EXISTS code_review_results;
DROP TABLE IF EXISTS story_evaluations;
