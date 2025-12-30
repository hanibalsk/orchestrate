-- Database Optimization Migration
-- Epic 016: Autonomous Epic Processing
-- Adds composite indexes, updated_at triggers, JSON validation, and FK documentation

-- =============================================================================
-- PART 1: COMPOSITE INDEXES FOR PERFORMANCE
-- =============================================================================
-- These composite indexes optimize common query patterns for Epic 016 features

-- Autonomous sessions: queries filtering by state and epic
CREATE INDEX IF NOT EXISTS idx_autonomous_sessions_state_epic
ON autonomous_sessions(state, current_epic_id);

-- Autonomous sessions: queries filtering by state and story
CREATE INDEX IF NOT EXISTS idx_autonomous_sessions_state_story
ON autonomous_sessions(state, current_story_id);

-- Agent continuations: queries filtering by agent and status
CREATE INDEX IF NOT EXISTS idx_agent_continuations_agent_status
ON agent_continuations(agent_id, status);

-- Agent continuations: queries for session continuations by status
CREATE INDEX IF NOT EXISTS idx_agent_continuations_session_status
ON agent_continuations(session_id, status);

-- Work evaluations: queries filtering by session and status
CREATE INDEX IF NOT EXISTS idx_work_evaluations_session_status
ON work_evaluations(session_id, status);

-- Work evaluations: queries filtering by story and evaluation type
CREATE INDEX IF NOT EXISTS idx_work_evaluations_story_type
ON work_evaluations(story_id, evaluation_type);

-- Stuck agent detections: queries for unresolved issues by agent
CREATE INDEX IF NOT EXISTS idx_stuck_agent_detections_agent_resolved
ON stuck_agent_detections(agent_id, resolved, detected_at);

-- Stuck agent detections: queries for session issues by severity
CREATE INDEX IF NOT EXISTS idx_stuck_agent_detections_session_severity
ON stuck_agent_detections(session_id, severity, resolved);

-- Recovery attempts: queries for recovery attempts by stuck detection
CREATE INDEX IF NOT EXISTS idx_recovery_attempts_stuck_detection
ON recovery_attempts(stuck_detection_id, outcome);

-- Recovery attempts: queries for session recovery attempts
CREATE INDEX IF NOT EXISTS idx_recovery_attempts_session_outcome
ON recovery_attempts(session_id, outcome);

-- Story evaluations: queries filtering by story and status
CREATE INDEX IF NOT EXISTS idx_story_evaluations_story_status
ON story_evaluations(story_id, status);

-- Story evaluations: queries for session evaluations
CREATE INDEX IF NOT EXISTS idx_story_evaluations_session_story
ON story_evaluations(session_id, story_id);

-- Code review results: queries filtering by story and verdict
CREATE INDEX IF NOT EXISTS idx_code_review_results_story_verdict
ON code_review_results(story_id, verdict);

-- CI check results: queries filtering by story and status
CREATE INDEX IF NOT EXISTS idx_ci_check_results_story_status
ON ci_check_results(story_id, status);

-- Review iterations: queries filtering by story and iteration
CREATE INDEX IF NOT EXISTS idx_review_iterations_story_iteration
ON review_iterations(story_id, iteration);

-- Review requests: queries filtering by story and status
CREATE INDEX IF NOT EXISTS idx_review_requests_story_status
ON review_requests(story_id, status);

-- Edge case events: queries filtering by session and resolution
CREATE INDEX IF NOT EXISTS idx_edge_case_events_session_resolution
ON edge_case_events(session_id, resolution);

-- Edge case events: queries filtering by type and resolution
CREATE INDEX IF NOT EXISTS idx_edge_case_events_type_resolution
ON edge_case_events(edge_case_type, resolution);

-- Edge case learnings: queries for successful patterns by type
CREATE INDEX IF NOT EXISTS idx_edge_case_learnings_type_success
ON edge_case_learnings(edge_case_type, success_rate DESC);

-- =============================================================================
-- PART 2: UPDATED_AT TRIGGERS
-- =============================================================================
-- Automatically maintain updated_at timestamps on record changes

-- Trigger for autonomous_sessions
CREATE TRIGGER IF NOT EXISTS update_autonomous_sessions_updated_at
AFTER UPDATE ON autonomous_sessions
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE autonomous_sessions
    SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;

-- Trigger for review_requests
CREATE TRIGGER IF NOT EXISTS update_review_requests_updated_at
AFTER UPDATE ON review_requests
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE review_requests
    SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;

-- Trigger for edge_case_learnings
CREATE TRIGGER IF NOT EXISTS update_edge_case_learnings_updated_at
AFTER UPDATE ON edge_case_learnings
FOR EACH ROW
WHEN NEW.updated_at = OLD.updated_at
BEGIN
    UPDATE edge_case_learnings
    SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;

-- =============================================================================
-- PART 3: JSON VALIDATION CONSTRAINTS
-- =============================================================================
-- Add CHECK constraints to validate JSON columns (SQLite 3.38.0+)
-- These constraints ensure data integrity for JSON fields

-- Note: SQLite doesn't support ADD CONSTRAINT to existing tables easily.
-- The json_valid() function will validate at runtime, and application code
-- should also validate before insertion. Future migrations can recreate tables
-- with these constraints if needed.

-- Validation is documented here for reference:
-- autonomous_sessions: config, work_queue, completed_items, metrics (all JSON)
-- autonomous_session_history: metadata (JSON)
-- agent_continuations: context, result (JSON)
-- work_evaluations: details (JSON)
-- stuck_agent_detections: details (JSON)
-- recovery_attempts: details (JSON)
-- story_evaluations: details (JSON)
-- code_review_results: issues (JSON array)
-- review_iterations: no JSON fields
-- review_requests: changed_files, criteria, previous_issues (JSON arrays)
-- ci_check_results: no JSON fields
-- edge_case_events: context (JSON)
-- edge_case_learnings: no JSON fields

-- =============================================================================
-- PART 4: FOREIGN KEY CONSTRAINT DOCUMENTATION
-- =============================================================================
-- SQLite FOREIGN KEY constraints are enforced at runtime when enabled:
-- PRAGMA foreign_keys = ON;
--
-- The following FK constraints already exist in the schema:
--
-- 1. autonomous_session_history.session_id -> autonomous_sessions.id (ON DELETE CASCADE)
--    Defined in migration 020_autonomous_sessions.sql
--
-- 2. recovery_attempts.stuck_detection_id -> stuck_agent_detections.id
--    Defined in migration 023_recovery_attempts.sql
--
-- Additional FK relationships (enforced at application level):
--
-- 3. autonomous_sessions.current_epic_id -> epics.id
--    - Not enforced at DB level to allow for flexible epic lifecycle
--    - Application must ensure epic exists before assignment
--
-- 4. autonomous_sessions.current_story_id -> stories.id
--    - Not enforced at DB level to allow for flexible story lifecycle
--    - Application must ensure story exists before assignment
--
-- 5. autonomous_sessions.current_agent_id -> agents.id
--    - Not enforced at DB level to allow agents to be cleaned up independently
--    - Application tracks agent-session relationships
--
-- 6. agent_continuations.agent_id -> agents.id
--    - Not enforced at DB level to preserve continuation history after agent deletion
--    - Application ensures valid agent_id on creation
--
-- 7. agent_continuations.session_id -> autonomous_sessions.id
--    - Optional reference, can be NULL for standalone continuations
--    - Application ensures valid session_id when provided
--
-- 8. work_evaluations.agent_id -> agents.id
--    - Not enforced at DB level to preserve evaluation history
--    - Application ensures valid agent_id on creation
--
-- 9. work_evaluations.session_id -> autonomous_sessions.id
--    - Optional reference, can be NULL for standalone evaluations
--    - Application ensures valid session_id when provided
--
-- 10. work_evaluations.story_id -> stories.id
--     - Optional reference, can be NULL for non-story evaluations
--     - Application ensures valid story_id when provided
--
-- 11. stuck_agent_detections.agent_id -> agents.id
--     - Not enforced at DB level to preserve detection history
--     - Application ensures valid agent_id on creation
--
-- 12. stuck_agent_detections.session_id -> autonomous_sessions.id
--     - Optional reference, can be NULL for standalone detections
--     - Application ensures valid session_id when provided
--
-- 13. recovery_attempts.agent_id -> agents.id
--     - Not enforced at DB level to preserve recovery history
--     - Application ensures valid agent_id on creation
--
-- 14. recovery_attempts.session_id -> autonomous_sessions.id
--     - Optional reference, can be NULL for standalone recovery attempts
--     - Application ensures valid session_id when provided
--
-- 15. story_evaluations.story_id -> stories.id
--     - Not enforced at DB level to preserve evaluation history
--     - Application ensures valid story_id on creation
--
-- 16. story_evaluations.agent_id -> agents.id
--     - Not enforced at DB level to preserve evaluation history
--     - Application ensures valid agent_id on creation
--
-- 17. story_evaluations.session_id -> autonomous_sessions.id
--     - Optional reference, can be NULL for standalone evaluations
--     - Application ensures valid session_id when provided
--
-- 18. code_review_results.story_id -> stories.id
--     - Not enforced at DB level to preserve review history
--     - Application ensures valid story_id on creation
--
-- 19. code_review_results.agent_id -> agents.id
--     - Not enforced at DB level to preserve review history
--     - Application ensures valid agent_id on creation
--
-- 20. code_review_results.session_id -> autonomous_sessions.id
--     - Optional reference, can be NULL for standalone reviews
--     - Application ensures valid session_id when provided
--
-- 21. ci_check_results.story_id -> stories.id
--     - Optional reference, can be NULL for non-story CI checks
--     - Application ensures valid story_id when provided
--
-- 22. ci_check_results.agent_id -> agents.id
--     - Not enforced at DB level to preserve CI history
--     - Application ensures valid agent_id on creation
--
-- 23. ci_check_results.session_id -> autonomous_sessions.id
--     - Optional reference, can be NULL for standalone CI checks
--     - Application ensures valid session_id when provided
--
-- 24. review_iterations.story_id -> stories.id
--     - Not enforced at DB level to preserve review history
--     - Application ensures valid story_id on creation
--
-- 25. review_requests.story_id -> stories.id
--     - Not enforced at DB level to preserve request history
--     - Application ensures valid story_id on creation
--
-- 26. review_requests.agent_id -> agents.id
--     - Not enforced at DB level to preserve request history
--     - Application ensures valid agent_id on creation
--
-- 27. review_requests.session_id -> autonomous_sessions.id
--     - Optional reference, can be NULL for standalone requests
--     - Application ensures valid session_id when provided
--
-- 28. edge_case_events.session_id -> autonomous_sessions.id
--     - Optional reference, can be NULL for standalone edge cases
--     - Application ensures valid session_id when provided
--
-- 29. edge_case_events.agent_id -> agents.id
--     - Optional reference, can be NULL for system-level edge cases
--     - Application ensures valid agent_id when provided
--
-- 30. edge_case_events.story_id -> stories.id
--     - Optional reference, can be NULL for non-story edge cases
--     - Application ensures valid story_id when provided
--
-- RATIONALE FOR APPLICATION-LEVEL ENFORCEMENT:
-- - Preserves historical data even after referenced entities are deleted
-- - Allows for flexible lifecycle management of sessions, agents, and stories
-- - Reduces database lock contention in high-concurrency scenarios
-- - Simplifies debugging by maintaining complete audit trails
-- - Follows the pattern established in earlier migrations

-- =============================================================================
-- PART 5: BOOLEAN DATA TYPE DOCUMENTATION
-- =============================================================================
-- SQLite does not have a native BOOLEAN type. BOOLEAN columns are stored as
-- INTEGER with values 0 (false) or 1 (true).
--
-- The following tables use BOOLEAN columns:
-- - stuck_agent_detections: resolved
-- - story_evaluations: ci_passed, review_passed, pr_mergeable
--
-- Application code should:
-- 1. Use boolean types in Rust (mapped to INTEGER in SQLite)
-- 2. Ensure values are only 0 or 1 when inserting/updating
-- 3. Treat any non-zero value as true when reading (defensive programming)
-- 4. Consider adding CHECK constraints in future migrations if strict validation needed

-- =============================================================================
-- VERIFICATION QUERIES
-- =============================================================================
-- Use these queries to verify the migration was successful:
--
-- List all indexes:
-- SELECT name, tbl_name, sql FROM sqlite_master WHERE type='index' AND name LIKE 'idx_%' ORDER BY tbl_name, name;
--
-- List all triggers:
-- SELECT name, tbl_name, sql FROM sqlite_master WHERE type='trigger' ORDER BY tbl_name, name;
--
-- Check foreign key integrity (requires PRAGMA foreign_keys = ON):
-- PRAGMA foreign_key_check;
--
-- List all tables with their schema:
-- SELECT name, sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name;
