-- Rollback Work Evaluations Schema
-- Epic 016: Autonomous Epic Processing - Story 6 & 8
-- Reverses migration 022_work_evaluations.sql

-- Drop stuck_agent_detections indexes
DROP INDEX IF EXISTS idx_stuck_agent_detections_detected_at;
DROP INDEX IF EXISTS idx_stuck_agent_detections_severity;
DROP INDEX IF EXISTS idx_stuck_agent_detections_resolved;
DROP INDEX IF EXISTS idx_stuck_agent_detections_agent_id;

-- Drop work_evaluations indexes
DROP INDEX IF EXISTS idx_work_evaluations_created_at;
DROP INDEX IF EXISTS idx_work_evaluations_status;
DROP INDEX IF EXISTS idx_work_evaluations_session_id;
DROP INDEX IF EXISTS idx_work_evaluations_agent_id;

-- Drop tables
DROP TABLE IF EXISTS stuck_agent_detections;
DROP TABLE IF EXISTS work_evaluations;
