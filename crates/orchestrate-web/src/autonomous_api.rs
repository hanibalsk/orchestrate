//! Autonomous Processing REST API
//!
//! Epic 016: Autonomous Epic Processing - Story 15
//!
//! Provides REST API endpoints for autonomous epic processing:
//! - POST /api/epic/auto-process - Start autonomous processing
//! - GET /api/epic/auto-status - Get current status
//! - POST /api/epic/auto-pause - Pause processing
//! - POST /api/epic/auto-resume - Resume processing
//! - POST /api/epic/auto-stop - Stop processing
//! - GET /api/epic/stuck-agents - List stuck agents
//! - POST /api/epic/:id/unblock - Unblock epic
//!
//! WebSocket events are handled in websocket.rs

use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use orchestrate_core::{
    AutonomousSession, AutonomousSessionState, EdgeCaseResolution, SessionConfig, StuckSeverity,
    StuckType,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::{ApiError, AppState};

// Input validation constants
const MAX_EPIC_PATTERN_LENGTH: usize = 256;
const MAX_AGENTS_LIMIT: u32 = 100;
const MAX_RETRIES_LIMIT: u32 = 10;

/// Create the autonomous processing API router
pub fn create_autonomous_router() -> Router<Arc<AppState>> {
    Router::new()
        // Autonomous processing control
        .route("/api/epic/auto-process", post(start_auto_process))
        .route("/api/epic/auto-status", get(get_auto_status))
        .route("/api/epic/auto-pause", post(pause_auto_process))
        .route("/api/epic/auto-resume", post(resume_auto_process))
        .route("/api/epic/auto-stop", post(stop_auto_process))
        // Stuck agent management
        .route("/api/epic/stuck-agents", get(list_stuck_agents))
        .route("/api/epic/:id/unblock", post(unblock_epic))
        // Edge case management
        .route("/api/epic/edge-cases", get(list_edge_cases))
        .route("/api/epic/edge-cases/:id/resolve", post(resolve_edge_case))
        // Session management
        .route("/api/epic/sessions", get(list_sessions))
        .route("/api/epic/sessions/:id", get(get_session))
        .route("/api/epic/sessions/:id/metrics", get(get_session_metrics))
}

// ==================== Request/Response Types ====================

/// Request to start autonomous processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartAutoProcessRequest {
    /// Epic ID or pattern to process
    pub epic_pattern: Option<String>,
    /// Optional custom configuration
    pub config: Option<AutoProcessConfig>,
}

/// Configuration for autonomous processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoProcessConfig {
    /// Maximum number of concurrent agents
    pub max_agents: Option<u32>,
    /// Maximum retries per story
    pub max_retries: Option<u32>,
    /// Whether this is a dry run
    pub dry_run: Option<bool>,
    /// Auto-merge approved PRs
    pub auto_merge: Option<bool>,
    /// Preferred model for execution
    pub model: Option<String>,
}

/// Response for auto process start
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartAutoProcessResponse {
    pub session_id: String,
    pub status: String,
    pub message: String,
}

/// Current autonomous processing status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoProcessStatus {
    /// Session ID if active
    pub session_id: Option<String>,
    /// Current state
    pub state: String,
    /// Epic being processed
    pub current_epic_id: Option<String>,
    /// Story being processed
    pub current_story_id: Option<String>,
    /// Stories completed
    pub stories_completed: u32,
    /// Stories failed
    pub stories_failed: u32,
    /// Agents spawned
    pub agents_spawned: u32,
    /// Tokens used
    pub tokens_used: u64,
    /// Number of stuck agents
    pub stuck_agents: u32,
    /// Queue depth
    pub queue_depth: u32,
    /// Success rate
    pub success_rate: f64,
}

/// Generic action response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    pub success: bool,
    pub message: String,
    pub session_id: Option<String>,
}

/// Stuck agent response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StuckAgentResponse {
    pub id: i64,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub stuck_type: String,
    pub severity: String,
    pub details: serde_json::Value,
    pub detected_at: String,
    pub resolved: bool,
    pub suggested_action: String,
}

/// Edge case response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCaseResponse {
    pub id: i64,
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub story_id: Option<String>,
    pub edge_case_type: String,
    pub resolution: String,
    pub action_taken: Option<String>,
    pub retry_count: u32,
    pub error_message: Option<String>,
    pub detected_at: String,
    pub resolved_at: Option<String>,
}

/// Session response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResponse {
    pub id: String,
    pub state: String,
    pub current_epic_id: Option<String>,
    pub current_story_id: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub completed_count: u32,
    pub failed_count: u32,
    pub stories_completed: u32,
    pub stories_failed: u32,
    pub tokens_used: u64,
}

/// Session metrics response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetricsResponse {
    pub session_id: String,
    pub stories_completed: u32,
    pub stories_failed: u32,
    pub reviews_passed: u32,
    pub reviews_failed: u32,
    pub total_iterations: u32,
    pub agents_spawned: u32,
    pub tokens_used: u64,
    pub success_rate: f64,
    pub review_pass_rate: f64,
    pub edge_cases_count: u32,
    pub stuck_detections_count: u32,
}

/// Query parameters for listing
#[derive(Debug, Clone, Deserialize)]
pub struct ListQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub status: Option<String>,
    pub session_id: Option<String>,
}

/// Resolve edge case request
#[derive(Debug, Clone, Deserialize)]
pub struct ResolveEdgeCaseRequest {
    pub resolution: String, // auto_resolved, manual_resolved, bypassed
    pub notes: Option<String>,
}

/// Unblock epic request
#[derive(Debug, Clone, Deserialize)]
pub struct UnblockRequest {
    pub action: String, // retry, skip, escalate
    pub notes: Option<String>,
}

// ==================== Validation ====================

/// Validate epic pattern input
fn validate_epic_pattern(pattern: &Option<String>) -> Result<(), ApiError> {
    if let Some(p) = pattern {
        // Check length
        if p.len() > MAX_EPIC_PATTERN_LENGTH {
            return Err(ApiError::bad_request(format!(
                "epic_pattern exceeds maximum length of {} characters",
                MAX_EPIC_PATTERN_LENGTH
            )));
        }

        // Check for path traversal attempts
        if p.contains("..") || p.contains("//") || p.starts_with('/') || p.starts_with('\\') {
            return Err(ApiError::bad_request(
                "epic_pattern contains invalid path characters",
            ));
        }

        // Check for null bytes
        if p.contains('\0') {
            return Err(ApiError::bad_request(
                "epic_pattern contains null bytes",
            ));
        }
    }
    Ok(())
}

/// Validate auto process configuration
fn validate_auto_process_config(config: &Option<AutoProcessConfig>) -> Result<(), ApiError> {
    if let Some(c) = config {
        if let Some(max_agents) = c.max_agents {
            if max_agents == 0 {
                return Err(ApiError::bad_request("max_agents must be at least 1"));
            }
            if max_agents > MAX_AGENTS_LIMIT {
                return Err(ApiError::bad_request(format!(
                    "max_agents exceeds maximum of {}",
                    MAX_AGENTS_LIMIT
                )));
            }
        }

        if let Some(max_retries) = c.max_retries {
            if max_retries > MAX_RETRIES_LIMIT {
                return Err(ApiError::bad_request(format!(
                    "max_retries exceeds maximum of {}",
                    MAX_RETRIES_LIMIT
                )));
            }
        }
    }
    Ok(())
}

// ==================== Handlers ====================

/// Start autonomous processing for an epic
async fn start_auto_process(
    State(state): State<Arc<AppState>>,
    Json(req): Json<StartAutoProcessRequest>,
) -> Result<Json<StartAutoProcessResponse>, ApiError> {
    // Validate inputs
    validate_epic_pattern(&req.epic_pattern)?;
    validate_auto_process_config(&req.config)?;

    // Check if there's already an active session
    let active_session = state
        .db
        .get_active_autonomous_session()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if active_session.is_some() {
        return Err(ApiError::conflict(
            "There is already an active autonomous processing session",
        ));
    }

    // Create session config from request
    let mut config = SessionConfig::default();
    if let Some(c) = req.config {
        if let Some(v) = c.max_agents {
            config.max_agents = v;
        }
        if let Some(v) = c.max_retries {
            config.max_retries = v;
        }
        if let Some(v) = c.dry_run {
            config.dry_run = v;
        }
        if let Some(v) = c.auto_merge {
            config.auto_merge = v;
        }
        config.model = c.model;
    }
    config.epic_pattern = req.epic_pattern;

    // Create new session
    let session = AutonomousSession::new().with_config(config);
    let session_id = session.id.clone();

    // Save to database
    state
        .db
        .create_autonomous_session(&session)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to create session: {}", e)))?;

    Ok(Json(StartAutoProcessResponse {
        session_id: session_id.clone(),
        status: "started".to_string(),
        message: format!("Autonomous processing started with session {}", session_id),
    }))
}

/// Get current autonomous processing status
async fn get_auto_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AutoProcessStatus>, ApiError> {
    // Get active session
    let active_session = state
        .db
        .get_active_autonomous_session()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let Some(session) = active_session else {
        return Ok(Json(AutoProcessStatus {
            session_id: None,
            state: "idle".to_string(),
            current_epic_id: None,
            current_story_id: None,
            stories_completed: 0,
            stories_failed: 0,
            agents_spawned: 0,
            tokens_used: 0,
            stuck_agents: 0,
            queue_depth: 0,
            success_rate: 0.0,
        }));
    };

    // Get stuck agents count
    let stuck_detections = state
        .db
        .get_all_unresolved_stuck_detections()
        .await
        .unwrap_or_default();

    Ok(Json(AutoProcessStatus {
        session_id: Some(session.id.clone()),
        state: session.state.as_str().to_string(),
        current_epic_id: session.current_epic_id.clone(),
        current_story_id: session.current_story_id.clone(),
        stories_completed: session.metrics.stories_completed,
        stories_failed: session.metrics.stories_failed,
        agents_spawned: session.metrics.agents_spawned,
        tokens_used: session.metrics.tokens_used,
        stuck_agents: stuck_detections.len() as u32,
        queue_depth: session.work_queue.len() as u32,
        success_rate: session.metrics.success_rate(),
    }))
}

/// Pause autonomous processing
async fn pause_auto_process(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ActionResponse>, ApiError> {
    let active_session = state
        .db
        .get_active_autonomous_session()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let Some(mut session) = active_session else {
        return Err(ApiError::not_found(
            "No active autonomous processing session",
        ));
    };

    let session_id = session.id.clone();

    // Pause the session
    session
        .pause("Paused via API")
        .map_err(|e| ApiError::conflict(format!("Cannot pause session: {}", e)))?;

    state
        .db
        .update_autonomous_session(&session)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to pause session: {}", e)))?;

    Ok(Json(ActionResponse {
        success: true,
        message: "Autonomous processing paused".to_string(),
        session_id: Some(session_id),
    }))
}

/// Resume autonomous processing
async fn resume_auto_process(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ActionResponse>, ApiError> {
    // Get paused session
    let sessions = state
        .db
        .get_sessions_by_state(AutonomousSessionState::Paused)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let Some(mut session) = sessions.into_iter().next() else {
        return Err(ApiError::not_found(
            "No paused autonomous processing session",
        ));
    };

    let session_id = session.id.clone();

    // Resume the session
    session
        .resume()
        .map_err(|e| ApiError::conflict(format!("Cannot resume session: {}", e)))?;

    state
        .db
        .update_autonomous_session(&session)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to resume session: {}", e)))?;

    Ok(Json(ActionResponse {
        success: true,
        message: "Autonomous processing resumed".to_string(),
        session_id: Some(session_id),
    }))
}

/// Stop autonomous processing
async fn stop_auto_process(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ActionResponse>, ApiError> {
    let active_session = state
        .db
        .get_active_autonomous_session()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let Some(mut session) = active_session else {
        return Ok(Json(ActionResponse {
            success: true,
            message: "No active session to stop".to_string(),
            session_id: None,
        }));
    };

    let session_id = session.id.clone();

    // Stop the session - transition to Done
    session
        .transition_to(AutonomousSessionState::Done)
        .map_err(|e| ApiError::conflict(format!("Cannot stop session: {}", e)))?;

    state
        .db
        .update_autonomous_session(&session)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to stop session: {}", e)))?;

    Ok(Json(ActionResponse {
        success: true,
        message: "Autonomous processing stopped".to_string(),
        session_id: Some(session_id),
    }))
}

/// List stuck agents
async fn list_stuck_agents(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<StuckAgentResponse>>, ApiError> {
    let detections = if let Some(session_id) = &query.session_id {
        state
            .db
            .get_stuck_detections_for_session(session_id)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    } else {
        state
            .db
            .get_all_unresolved_stuck_detections()
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    };

    let responses: Vec<StuckAgentResponse> = detections
        .into_iter()
        .map(|d| StuckAgentResponse {
            id: d.id,
            agent_id: d.agent_id.clone(),
            session_id: d.session_id.clone(),
            stuck_type: d.detection_type.as_str().to_string(),
            severity: d.severity.as_str().to_string(),
            details: d.details.clone(),
            detected_at: d.detected_at.to_rfc3339(),
            resolved: d.resolved,
            suggested_action: get_suggested_action(&d.detection_type, &d.severity),
        })
        .collect();

    Ok(Json(responses))
}

/// Unblock an epic/session
async fn unblock_epic(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UnblockRequest>,
) -> Result<Json<ActionResponse>, ApiError> {
    // Try to find session by ID first
    let session = state
        .db
        .get_autonomous_session(&id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let Some(mut session) = session else {
        return Err(ApiError::not_found("Session"));
    };

    if session.state != AutonomousSessionState::Blocked {
        return Err(ApiError::bad_request("Session is not blocked"));
    }

    // Handle based on action
    match req.action.as_str() {
        "retry" => {
            // Unblock and resume
            session.unblock().map_err(|e| {
                ApiError::internal(format!("Failed to unblock: {}", e))
            })?;
        }
        "skip" => {
            // Skip current work and continue
            session.unblock().map_err(|e| {
                ApiError::internal(format!("Failed to unblock: {}", e))
            })?;
            // Could remove current work item from queue here
        }
        "escalate" => {
            // Log escalation
            tracing::warn!("Session {} escalated: {:?}", id, req.notes);
            // Keep blocked for manual intervention
            return Ok(Json(ActionResponse {
                success: true,
                message: "Session escalated for manual intervention".to_string(),
                session_id: Some(session.id),
            }));
        }
        _ => {
            return Err(ApiError::bad_request(format!(
                "Unknown action: {}",
                req.action
            )));
        }
    }

    state
        .db
        .update_autonomous_session(&session)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update session: {}", e)))?;

    Ok(Json(ActionResponse {
        success: true,
        message: format!("Session {} unblocked with action: {}", id, req.action),
        session_id: Some(session.id),
    }))
}

/// List edge cases
async fn list_edge_cases(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<EdgeCaseResponse>>, ApiError> {
    let events = if let Some(session_id) = &query.session_id {
        state
            .db
            .get_edge_case_events_for_session(session_id)
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    } else if query.status.as_deref() == Some("unresolved") {
        state
            .db
            .get_unresolved_edge_case_events()
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    } else {
        // Return all recent edge cases (default to unresolved)
        state
            .db
            .get_unresolved_edge_case_events()
            .await
            .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
    };

    let responses: Vec<EdgeCaseResponse> = events
        .into_iter()
        .map(|e| EdgeCaseResponse {
            id: e.id,
            session_id: e.session_id,
            agent_id: e.agent_id,
            story_id: e.story_id,
            edge_case_type: e.edge_case_type.as_str().to_string(),
            resolution: e.resolution.as_str().to_string(),
            action_taken: e.action_taken,
            retry_count: e.retry_count,
            error_message: e.error_message,
            detected_at: e.detected_at.to_rfc3339(),
            resolved_at: e.resolved_at.map(|dt| dt.to_rfc3339()),
        })
        .collect();

    Ok(Json(responses))
}

/// Resolve an edge case
async fn resolve_edge_case(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<ResolveEdgeCaseRequest>,
) -> Result<Json<ActionResponse>, ApiError> {
    let mut event = state
        .db
        .get_edge_case_event(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Edge case event"))?;

    let resolution = match req.resolution.as_str() {
        "auto_resolved" => EdgeCaseResolution::AutoResolved,
        "manual_resolved" => EdgeCaseResolution::ManualResolved,
        "bypassed" => EdgeCaseResolution::Bypassed,
        _ => {
            return Err(ApiError::bad_request(format!(
                "Invalid resolution: {}. Must be: auto_resolved, manual_resolved, or bypassed",
                req.resolution
            )));
        }
    };

    event.resolve(resolution, req.notes);
    state
        .db
        .update_edge_case_event(&event)
        .await
        .map_err(|e| ApiError::internal(format!("Failed to update edge case: {}", e)))?;

    Ok(Json(ActionResponse {
        success: true,
        message: format!("Edge case {} resolved", id),
        session_id: None,
    }))
}

/// List autonomous sessions
async fn list_sessions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<SessionResponse>>, ApiError> {
    let limit = query.limit.unwrap_or(50) as i64;

    let sessions = state
        .db
        .list_autonomous_sessions(query.status.as_deref(), Some(limit))
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let responses: Vec<SessionResponse> = sessions
        .into_iter()
        .map(|s| SessionResponse {
            id: s.id,
            state: s.state.as_str().to_string(),
            current_epic_id: s.current_epic_id,
            current_story_id: s.current_story_id,
            started_at: s.started_at.to_rfc3339(),
            completed_at: s.completed_at.map(|dt| dt.to_rfc3339()),
            completed_count: s.completed_items.len() as u32,
            failed_count: s.completed_items.iter().filter(|i| !i.success).count() as u32,
            stories_completed: s.metrics.stories_completed,
            stories_failed: s.metrics.stories_failed,
            tokens_used: s.metrics.tokens_used,
        })
        .collect();

    Ok(Json(responses))
}

/// Get session details
async fn get_session(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionResponse>, ApiError> {
    let session = state
        .db
        .get_autonomous_session(&id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Session"))?;

    Ok(Json(SessionResponse {
        id: session.id,
        state: session.state.as_str().to_string(),
        current_epic_id: session.current_epic_id,
        current_story_id: session.current_story_id,
        started_at: session.started_at.to_rfc3339(),
        completed_at: session.completed_at.map(|dt| dt.to_rfc3339()),
        completed_count: session.completed_items.len() as u32,
        failed_count: session
            .completed_items
            .iter()
            .filter(|i| !i.success)
            .count() as u32,
        stories_completed: session.metrics.stories_completed,
        stories_failed: session.metrics.stories_failed,
        tokens_used: session.metrics.tokens_used,
    }))
}

/// Get session metrics
async fn get_session_metrics(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<SessionMetricsResponse>, ApiError> {
    let session = state
        .db
        .get_autonomous_session(&id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Session"))?;

    // Get edge case stats for this session
    let edge_cases = state
        .db
        .get_edge_case_events_for_session(&id)
        .await
        .unwrap_or_default();

    // Get stuck detection stats
    let stuck_detections = state
        .db
        .get_stuck_detections_for_session(&id)
        .await
        .unwrap_or_default();

    Ok(Json(SessionMetricsResponse {
        session_id: session.id,
        stories_completed: session.metrics.stories_completed,
        stories_failed: session.metrics.stories_failed,
        reviews_passed: session.metrics.reviews_passed,
        reviews_failed: session.metrics.reviews_failed,
        total_iterations: session.metrics.total_iterations,
        agents_spawned: session.metrics.agents_spawned,
        tokens_used: session.metrics.tokens_used,
        success_rate: session.metrics.success_rate(),
        review_pass_rate: session.metrics.review_pass_rate(),
        edge_cases_count: edge_cases.len() as u32,
        stuck_detections_count: stuck_detections.len() as u32,
    }))
}

// ==================== Helper Functions ====================

fn get_suggested_action(stuck_type: &StuckType, severity: &StuckSeverity) -> String {
    match (stuck_type, severity) {
        (StuckType::TurnLimit, _) => "Increase max_turns or use model escalation".to_string(),
        (StuckType::NoProgress, StuckSeverity::Critical) => {
            "Manual intervention required".to_string()
        }
        (StuckType::NoProgress, _) => "Retry with fresh context".to_string(),
        (StuckType::CiTimeout, _) => "Check CI system status, consider retry".to_string(),
        (StuckType::ReviewDelay, _) => "Wait for review or escalate".to_string(),
        (StuckType::MergeConflict, _) => "Spawn conflict resolver agent".to_string(),
        (StuckType::RateLimit, _) => "Wait for rate limit reset".to_string(),
        (StuckType::ContextLimit, _) => "Summarize context and retry".to_string(),
        (StuckType::ErrorLoop, _) => "Fresh retry with different approach".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggested_action_for_stuck_types() {
        let action = get_suggested_action(&StuckType::TurnLimit, &StuckSeverity::Medium);
        assert!(action.contains("max_turns"));

        let action = get_suggested_action(&StuckType::MergeConflict, &StuckSeverity::High);
        assert!(action.contains("conflict resolver"));

        let action = get_suggested_action(&StuckType::NoProgress, &StuckSeverity::Critical);
        assert!(action.contains("Manual"));
    }

    #[test]
    fn test_auto_process_config_defaults() {
        let config = AutoProcessConfig {
            max_agents: None,
            max_retries: None,
            dry_run: None,
            auto_merge: None,
            model: None,
        };
        // Just verify it creates without panic
        assert!(config.max_agents.is_none());
    }

    // ==================== Validation Tests ====================

    #[test]
    fn test_validate_epic_pattern_valid() {
        assert!(validate_epic_pattern(&None).is_ok());
        assert!(validate_epic_pattern(&Some("epic-016".to_string())).is_ok());
        assert!(validate_epic_pattern(&Some("epic-*".to_string())).is_ok());
        assert!(validate_epic_pattern(&Some("epic-016-story-1".to_string())).is_ok());
    }

    #[test]
    fn test_validate_epic_pattern_too_long() {
        let long_pattern = "a".repeat(MAX_EPIC_PATTERN_LENGTH + 1);
        let result = validate_epic_pattern(&Some(long_pattern));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_epic_pattern_path_traversal() {
        assert!(validate_epic_pattern(&Some("../etc/passwd".to_string())).is_err());
        assert!(validate_epic_pattern(&Some("epic/../secret".to_string())).is_err());
        assert!(validate_epic_pattern(&Some("/etc/passwd".to_string())).is_err());
        assert!(validate_epic_pattern(&Some("\\windows\\system32".to_string())).is_err());
        assert!(validate_epic_pattern(&Some("epic//double".to_string())).is_err());
    }

    #[test]
    fn test_validate_epic_pattern_null_bytes() {
        assert!(validate_epic_pattern(&Some("epic\0-016".to_string())).is_err());
    }

    #[test]
    fn test_validate_config_valid() {
        assert!(validate_auto_process_config(&None).is_ok());
        assert!(validate_auto_process_config(&Some(AutoProcessConfig {
            max_agents: Some(5),
            max_retries: Some(3),
            dry_run: None,
            auto_merge: None,
            model: None,
        })).is_ok());
    }

    #[test]
    fn test_validate_config_max_agents_zero() {
        let result = validate_auto_process_config(&Some(AutoProcessConfig {
            max_agents: Some(0),
            max_retries: None,
            dry_run: None,
            auto_merge: None,
            model: None,
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_max_agents_exceeds_limit() {
        let result = validate_auto_process_config(&Some(AutoProcessConfig {
            max_agents: Some(MAX_AGENTS_LIMIT + 1),
            max_retries: None,
            dry_run: None,
            auto_merge: None,
            model: None,
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_config_max_retries_exceeds_limit() {
        let result = validate_auto_process_config(&Some(AutoProcessConfig {
            max_agents: None,
            max_retries: Some(MAX_RETRIES_LIMIT + 1),
            dry_run: None,
            auto_merge: None,
            model: None,
        }));
        assert!(result.is_err());
    }
}
