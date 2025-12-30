-- Rollback Review Iterations Schema
-- Epic 016: Autonomous Epic Processing - Story 9
-- Reverses migration 025_review_iterations.sql

-- Drop review_requests indexes
DROP INDEX IF EXISTS idx_review_requests_status;
DROP INDEX IF EXISTS idx_review_requests_story_id;

-- Drop review_iterations indexes
DROP INDEX IF EXISTS idx_review_iterations_escalation;
DROP INDEX IF EXISTS idx_review_iterations_verdict;
DROP INDEX IF EXISTS idx_review_iterations_story_id;

-- Drop tables
DROP TABLE IF EXISTS review_requests;
DROP TABLE IF EXISTS review_iterations;
