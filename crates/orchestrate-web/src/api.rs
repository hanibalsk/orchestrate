//! REST API endpoints with authentication and validation

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use orchestrate_core::{
    Agent, AgentState, AgentType, ApprovalDecision, ApprovalRequest, ApprovalService,
    ApprovalStatus, CustomInstruction, Database, Feedback, FeedbackRating, FeedbackSource,
    FeedbackStats, InstructionEffectiveness, InstructionScope, InstructionSource, LearningEngine,
    LearningPattern, PatternStatus, Pipeline, PipelineRun, PipelineRunStatus, PipelineStage,
    Schedule, ScheduleRun,
};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Maximum task length
const MAX_TASK_LENGTH: usize = 10_000;

/// API error response
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub error: String,
    pub code: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = match self.code.as_str() {
            "unauthorized" => StatusCode::UNAUTHORIZED,
            "forbidden" => StatusCode::FORBIDDEN,
            "not_found" => StatusCode::NOT_FOUND,
            "bad_request" | "validation_error" => StatusCode::BAD_REQUEST,
            "conflict" => StatusCode::CONFLICT,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self)).into_response()
    }
}

impl ApiError {
    fn unauthorized() -> Self {
        Self {
            error: "Invalid or missing API key".to_string(),
            code: "unauthorized".to_string(),
        }
    }

    fn not_found(entity: &str) -> Self {
        Self {
            error: format!("{} not found", entity),
            code: "not_found".to_string(),
        }
    }

    fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "bad_request".to_string(),
        }
    }

    fn validation(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "validation_error".to_string(),
        }
    }

    fn internal(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "internal_error".to_string(),
        }
    }

    fn conflict(msg: impl Into<String>) -> Self {
        Self {
            error: msg.into(),
            code: "conflict".to_string(),
        }
    }
}

/// Application state
#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub api_key: Option<SecretString>,
}

impl AppState {
    /// Create new app state with optional API key authentication
    pub fn new(db: Database, api_key: Option<String>) -> Self {
        Self {
            db,
            api_key: api_key.map(SecretString::new),
        }
    }
}

/// Authentication middleware
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    // If no API key configured, allow all requests
    let Some(ref expected_key) = state.api_key else {
        return Ok(next.run(request).await);
    };

    // Check for API key in headers
    let headers = request.headers();
    let provided_key = headers
        .get("x-api-key")
        .or_else(|| headers.get("authorization"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s));

    match provided_key {
        Some(key) if key == expected_key.expose_secret() => Ok(next.run(request).await),
        _ => Err(ApiError::unauthorized()),
    }
}

/// Create the API router (API endpoints only)
pub fn create_api_router(state: Arc<AppState>) -> Router {
    // Routes that require authentication
    // Note: axum 0.7 uses :param syntax, axum 0.8+ uses {param}
    let protected_routes = Router::new()
        // Agent routes
        .route("/api/agents", get(list_agents).post(create_agent))
        .route("/api/agents/:id", get(get_agent))
        .route("/api/agents/:id/pause", post(pause_agent))
        .route("/api/agents/:id/resume", post(resume_agent))
        .route("/api/agents/:id/terminate", post(terminate_agent))
        .route("/api/agents/:id/messages", get(get_messages))
        .route("/api/status", get(system_status))
        // Instruction routes
        .route(
            "/api/instructions",
            get(list_instructions).post(create_instruction),
        )
        .route(
            "/api/instructions/:id",
            get(get_instruction)
                .put(update_instruction)
                .delete(delete_instruction),
        )
        .route("/api/instructions/:id/enable", post(enable_instruction))
        .route("/api/instructions/:id/disable", post(disable_instruction))
        .route(
            "/api/instructions/:id/effectiveness",
            get(get_instruction_effectiveness),
        )
        // Learning pattern routes
        .route("/api/patterns", get(list_patterns))
        .route("/api/patterns/:id", get(get_pattern))
        .route("/api/patterns/:id/approve", post(approve_pattern))
        .route("/api/patterns/:id/reject", post(reject_pattern))
        .route("/api/learning/process", post(process_patterns))
        .route("/api/learning/cleanup", post(cleanup_instructions))
        // Pipeline routes
        .route(
            "/api/pipelines",
            get(list_pipelines).post(create_pipeline),
        )
        .route(
            "/api/pipelines/:name",
            get(get_pipeline)
                .put(update_pipeline)
                .delete(delete_pipeline),
        )
        .route("/api/pipelines/:name/run", post(trigger_pipeline_run))
        .route("/api/pipelines/:name/runs", get(list_pipeline_runs))
        .route("/api/pipeline-runs/:id", get(get_pipeline_run))
        .route("/api/pipeline-runs/:id/cancel", post(cancel_pipeline_run))
        .route("/api/pipeline-runs/:id/stages", get(list_pipeline_stages))
        // Approval routes
        .route("/api/approvals", get(list_pending_approvals))
        .route("/api/approvals/:id/approve", post(approve_approval))
        .route("/api/approvals/:id/reject", post(reject_approval))
        // Schedule routes
        .route("/api/schedules", get(list_schedules).post(create_schedule))
        .route(
            "/api/schedules/:id",
            get(get_schedule).put(update_schedule).delete(delete_schedule),
        )
        .route("/api/schedules/:id/pause", post(pause_schedule))
        .route("/api/schedules/:id/resume", post(resume_schedule))
        .route("/api/schedules/:id/run", post(run_schedule))
        .route("/api/schedules/:id/runs", get(get_schedule_runs))
        // Feedback routes
        .route("/api/feedback", get(list_feedback).post(create_feedback))
        .route("/api/feedback/:id", get(get_feedback).delete(delete_feedback))
        .route("/api/feedback/stats", get(get_feedback_stats))
        // Learning analytics routes
        .route("/api/learning/effectiveness", get(get_learning_effectiveness))
        .route("/api/learning/suggestions", get(get_learning_suggestions))
        .route("/api/learning/analyze", post(trigger_learning_analysis))
        // Experiment routes
        .route("/api/experiments", get(list_experiments).post(create_experiment))
        .route("/api/experiments/:id", get(get_experiment))
        .route("/api/experiments/:id/results", get(get_experiment_results))
        .route("/api/experiments/:id/promote", post(promote_experiment))
        // Prediction routes
        .route("/api/predictions", post(get_prediction))
        // Documentation routes
        .route("/api/docs/generate", post(generate_documentation))
        .route("/api/docs/validate", post(validate_documentation))
        .route("/api/docs/adrs", get(list_adrs).post(create_adr))
        .route("/api/docs/adrs/:number", get(get_adr).put(update_adr))
        .route("/api/docs/changelog", post(generate_changelog))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    // Public routes (no auth required)
    let public_routes = Router::new().route("/api/health", get(health_check));

    Router::new()
        .merge(protected_routes)
        .merge(public_routes)
        .with_state(state)
}

/// Create the full router with both API and UI routes
pub fn create_router(state: Arc<AppState>) -> Router {
    create_router_with_webhook(state, None)
}

/// Create the full router with both API, UI routes, and optional webhook endpoint
pub fn create_router_with_webhook(state: Arc<AppState>, webhook_secret: Option<String>) -> Router {
    let api_router = create_api_router(state.clone());
    let ui_router = crate::ui::create_ui_router().with_state(state.clone());

    // Create WebSocket state with the same database
    let ws_state = Arc::new(crate::websocket::WsState::new(state.db.clone()));

    let mut router = Router::new()
        .merge(api_router)
        .merge(ui_router)
        .route(
            "/ws",
            axum::routing::get(crate::websocket::ws_handler).with_state(ws_state),
        );

    // Add webhook endpoint (always available, secret is optional for signature verification)
    let secret = webhook_secret.or_else(|| std::env::var("GITHUB_WEBHOOK_SECRET").ok());
    let webhook_config = crate::webhook::WebhookConfig::new(secret);
    let webhook_state = Arc::new(crate::webhook::WebhookState::new(webhook_config, state.db.clone()));

    router = router.route(
        "/webhooks/github",
        post(crate::webhook::github_webhook_handler).with_state(webhook_state),
    );

    router
}

// ==================== Handlers ====================

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn list_agents(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<AgentResponse>>, ApiError> {
    let agents = state
        .db
        .list_agents()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(agents.into_iter().map(Into::into).collect()))
}

async fn get_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AgentResponse>, ApiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("Invalid UUID format"))?;

    let agent = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Agent"))?;

    Ok(Json(agent.into()))
}

async fn create_agent(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateAgentRequest>,
) -> Result<Json<AgentResponse>, ApiError> {
    // Validate request
    req.validate()?;

    let mut agent = Agent::new(req.agent_type, req.task);

    // Set worktree if provided
    if let Some(worktree_id) = req.worktree_id {
        agent = agent.with_worktree(worktree_id);
    }

    state
        .db
        .insert_agent(&agent)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(agent.into()))
}

async fn pause_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AgentResponse>, ApiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("Invalid UUID format"))?;

    let mut agent = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Agent"))?;

    let original_updated_at = agent.updated_at.to_rfc3339();

    agent.transition_to(AgentState::Paused).map_err(|_| {
        ApiError::conflict(format!("Cannot pause agent in state {:?}", agent.state))
    })?;

    // Use optimistic locking to prevent race conditions
    let updated = state
        .db
        .update_agent_with_version(&agent, &original_updated_at)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !updated {
        return Err(ApiError::conflict("Agent was modified by another request"));
    }

    Ok(Json(agent.into()))
}

async fn resume_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AgentResponse>, ApiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("Invalid UUID format"))?;

    let mut agent = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Agent"))?;

    let original_updated_at = agent.updated_at.to_rfc3339();

    agent.transition_to(AgentState::Running).map_err(|_| {
        ApiError::conflict(format!("Cannot resume agent in state {:?}", agent.state))
    })?;

    let updated = state
        .db
        .update_agent_with_version(&agent, &original_updated_at)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !updated {
        return Err(ApiError::conflict("Agent was modified by another request"));
    }

    Ok(Json(agent.into()))
}

async fn terminate_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<AgentResponse>, ApiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("Invalid UUID format"))?;

    let mut agent = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Agent"))?;

    let original_updated_at = agent.updated_at.to_rfc3339();

    agent.transition_to(AgentState::Terminated).map_err(|_| {
        ApiError::conflict(format!("Cannot terminate agent in state {:?}", agent.state))
    })?;

    let updated = state
        .db
        .update_agent_with_version(&agent, &original_updated_at)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !updated {
        return Err(ApiError::conflict("Agent was modified by another request"));
    }

    Ok(Json(agent.into()))
}

async fn get_messages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<Vec<MessageResponse>>, ApiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| ApiError::bad_request("Invalid UUID format"))?;

    // Verify agent exists
    let _ = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Agent"))?;

    let messages = state
        .db
        .get_messages(uuid)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(messages.into_iter().map(Into::into).collect()))
}

async fn system_status(State(state): State<Arc<AppState>>) -> Result<Json<SystemStatus>, ApiError> {
    let agents = state
        .db
        .list_agents()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let running = agents
        .iter()
        .filter(|a| a.state == AgentState::Running)
        .count();
    let paused = agents
        .iter()
        .filter(|a| a.state == AgentState::Paused)
        .count();
    let completed = agents
        .iter()
        .filter(|a| a.state == AgentState::Completed)
        .count();

    Ok(Json(SystemStatus {
        total_agents: agents.len(),
        running_agents: running,
        paused_agents: paused,
        completed_agents: completed,
    }))
}

// ==================== Instruction Handlers ====================

async fn list_instructions(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListInstructionsParams>,
) -> Result<Json<Vec<InstructionResponse>>, ApiError> {
    let scope = params
        .scope
        .as_deref()
        .map(|s| InstructionScope::from_str(s))
        .transpose()
        .map_err(|e| ApiError::bad_request(format!("Invalid scope: {}", e)))?;

    let source = params
        .source
        .as_deref()
        .map(|s| InstructionSource::from_str(s))
        .transpose()
        .map_err(|e| ApiError::bad_request(format!("Invalid source: {}", e)))?;

    let instructions = state
        .db
        .list_instructions(params.enabled_only.unwrap_or(false), scope, source)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(instructions.into_iter().map(Into::into).collect()))
}

async fn get_instruction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<InstructionResponse>, ApiError> {
    let instruction = state
        .db
        .get_instruction(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Instruction"))?;

    Ok(Json(instruction.into()))
}

async fn create_instruction(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateInstructionRequest>,
) -> Result<Json<InstructionResponse>, ApiError> {
    req.validate()?;

    let mut instruction = if req.scope == Some("agent_type".to_string()) {
        let agent_type = req
            .agent_type
            .as_ref()
            .ok_or_else(|| ApiError::validation("agent_type required for agent_type scope"))?;
        let agent_type = AgentType::from_str(agent_type)
            .map_err(|_| ApiError::bad_request(format!("Invalid agent_type: {}", agent_type)))?;
        CustomInstruction::for_agent_type(&req.name, &req.content, agent_type)
    } else {
        CustomInstruction::global(&req.name, &req.content)
    };

    if let Some(priority) = req.priority {
        instruction = instruction.with_priority(priority);
    }
    if let Some(tags) = req.tags {
        instruction = instruction.with_tags(tags);
    }
    if let Some(created_by) = req.created_by {
        instruction = instruction.with_created_by(created_by);
    }

    let id = state
        .db
        .insert_instruction(&instruction)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    instruction.id = id;
    Ok(Json(instruction.into()))
}

async fn update_instruction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateInstructionRequest>,
) -> Result<Json<InstructionResponse>, ApiError> {
    let mut instruction = state
        .db
        .get_instruction(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Instruction"))?;

    if let Some(name) = req.name {
        instruction.name = name;
    }
    if let Some(content) = req.content {
        instruction.content = content;
    }
    if let Some(priority) = req.priority {
        instruction.priority = priority;
    }
    if let Some(enabled) = req.enabled {
        instruction.enabled = enabled;
    }
    if let Some(tags) = req.tags {
        instruction.tags = tags;
    }

    state
        .db
        .update_instruction(&instruction)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(instruction.into()))
}

async fn delete_instruction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, ApiError> {
    // Verify instruction exists
    let _ = state
        .db
        .get_instruction(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Instruction"))?;

    state
        .db
        .delete_instruction(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(serde_json::json!({"deleted": true})))
}

async fn enable_instruction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<InstructionResponse>, ApiError> {
    let mut instruction = state
        .db
        .get_instruction(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Instruction"))?;

    state
        .db
        .set_instruction_enabled(id, true)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    instruction.enabled = true;
    Ok(Json(instruction.into()))
}

async fn disable_instruction(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<InstructionResponse>, ApiError> {
    let mut instruction = state
        .db
        .get_instruction(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Instruction"))?;

    state
        .db
        .set_instruction_enabled(id, false)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    instruction.enabled = false;
    Ok(Json(instruction.into()))
}

async fn get_instruction_effectiveness(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<EffectivenessResponse>, ApiError> {
    // Verify instruction exists
    let _ = state
        .db
        .get_instruction(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Instruction"))?;

    let effectiveness = state
        .db
        .get_instruction_effectiveness(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Effectiveness metrics"))?;

    Ok(Json(effectiveness.into()))
}

// ==================== Pattern Handlers ====================

async fn list_patterns(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListPatternsParams>,
) -> Result<Json<Vec<PatternResponse>>, ApiError> {
    let status = params
        .status
        .as_deref()
        .map(|s| PatternStatus::from_str(s))
        .transpose()
        .map_err(|e| ApiError::bad_request(format!("Invalid status: {}", e)))?;

    let patterns = state
        .db
        .list_patterns(status)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(patterns.into_iter().map(Into::into).collect()))
}

async fn get_pattern(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<PatternResponse>, ApiError> {
    let pattern = state
        .db
        .get_pattern(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pattern"))?;

    Ok(Json(pattern.into()))
}

async fn approve_pattern(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<PatternResponse>, ApiError> {
    let pattern = state
        .db
        .get_pattern(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pattern"))?;

    // Generate instruction from pattern
    let engine = LearningEngine::new();
    let instruction = engine
        .generate_instruction_from_pattern(&pattern)
        .ok_or_else(|| ApiError::bad_request("Could not generate instruction from pattern"))?;

    let instruction_id = state
        .db
        .insert_instruction(&instruction)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    state
        .db
        .update_pattern_status(id, PatternStatus::Approved, Some(instruction_id))
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Reload pattern to get updated state
    let updated_pattern = state
        .db
        .get_pattern(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pattern"))?;

    Ok(Json(updated_pattern.into()))
}

async fn reject_pattern(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<PatternResponse>, ApiError> {
    let _ = state
        .db
        .get_pattern(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pattern"))?;

    state
        .db
        .update_pattern_status(id, PatternStatus::Rejected, None)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let updated_pattern = state
        .db
        .get_pattern(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pattern"))?;

    Ok(Json(updated_pattern.into()))
}

async fn process_patterns(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ProcessPatternsResponse>, ApiError> {
    let engine = LearningEngine::new();

    let created = engine
        .process_patterns(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Processing error: {}", e)))?;

    Ok(Json(ProcessPatternsResponse {
        created_count: created.len(),
        instruction_ids: created.iter().map(|i| i.id).collect(),
    }))
}

async fn cleanup_instructions(
    State(state): State<Arc<AppState>>,
) -> Result<Json<CleanupResponse>, ApiError> {
    let engine = LearningEngine::new();

    let result = engine
        .cleanup(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Cleanup error: {}", e)))?;

    Ok(Json(CleanupResponse {
        disabled_count: result.disabled_count,
        deleted_names: result.deleted_names,
    }))
}

// ==================== Pipeline Handlers ====================

async fn list_pipelines(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<PipelineResponse>>, ApiError> {
    let pipelines = state
        .db
        .list_pipelines()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(pipelines.into_iter().map(Into::into).collect()))
}

async fn get_pipeline(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<PipelineResponse>, ApiError> {
    let pipeline = state
        .db
        .get_pipeline_by_name(&name)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pipeline"))?;

    Ok(Json(pipeline.into()))
}

async fn create_pipeline(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePipelineRequest>,
) -> Result<Json<PipelineResponse>, ApiError> {
    req.validate()?;

    let mut pipeline = Pipeline::new(req.name, req.definition);
    if let Some(enabled) = req.enabled {
        pipeline.enabled = enabled;
    }

    let id = state
        .db
        .insert_pipeline(&pipeline)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    pipeline.id = Some(id);
    Ok(Json(pipeline.into()))
}

async fn update_pipeline(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<UpdatePipelineRequest>,
) -> Result<Json<PipelineResponse>, ApiError> {
    let mut pipeline = state
        .db
        .get_pipeline_by_name(&name)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pipeline"))?;

    if let Some(definition) = req.definition {
        pipeline.definition = definition;
    }
    if let Some(enabled) = req.enabled {
        pipeline.enabled = enabled;
    }

    state
        .db
        .update_pipeline(&pipeline)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(pipeline.into()))
}

async fn delete_pipeline(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let pipeline = state
        .db
        .get_pipeline_by_name(&name)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pipeline"))?;

    let id = pipeline
        .id
        .ok_or_else(|| ApiError::internal("Pipeline missing ID"))?;

    state
        .db
        .delete_pipeline(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(serde_json::json!({"deleted": true})))
}

async fn trigger_pipeline_run(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(req): Json<TriggerRunRequest>,
) -> Result<Json<PipelineRunResponse>, ApiError> {
    let pipeline = state
        .db
        .get_pipeline_by_name(&name)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pipeline"))?;

    let pipeline_id = pipeline
        .id
        .ok_or_else(|| ApiError::internal("Pipeline missing ID"))?;

    let mut run = PipelineRun::new(pipeline_id, req.trigger_event);
    let run_id = state
        .db
        .insert_pipeline_run(&run)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    run.id = Some(run_id);
    Ok(Json(run.into()))
}

async fn list_pipeline_runs(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<Vec<PipelineRunResponse>>, ApiError> {
    let pipeline = state
        .db
        .get_pipeline_by_name(&name)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pipeline"))?;

    let pipeline_id = pipeline
        .id
        .ok_or_else(|| ApiError::internal("Pipeline missing ID"))?;

    let runs = state
        .db
        .list_pipeline_runs(pipeline_id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(runs.into_iter().map(Into::into).collect()))
}

async fn get_pipeline_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<PipelineRunResponse>, ApiError> {
    let run = state
        .db
        .get_pipeline_run(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pipeline run"))?;

    Ok(Json(run.into()))
}

async fn cancel_pipeline_run(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<PipelineRunResponse>, ApiError> {
    let mut run = state
        .db
        .get_pipeline_run(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Pipeline run"))?;

    // Only allow cancelling runs that are pending, running, or waiting for approval
    match run.status {
        PipelineRunStatus::Pending
        | PipelineRunStatus::Running
        | PipelineRunStatus::WaitingApproval => {
            run.mark_cancelled();
            state
                .db
                .update_pipeline_run(&run)
                .await
                .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;
            Ok(Json(run.into()))
        }
        _ => Err(ApiError::conflict(format!(
            "Cannot cancel pipeline run in status: {}",
            run.status.as_str()
        ))),
    }
}

async fn list_pipeline_stages(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<PipelineStageResponse>>, ApiError> {
    let stages = state
        .db
        .list_pipeline_stages(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(stages.into_iter().map(|s| s.into()).collect()))
}

// ==================== Approval Handlers ====================

async fn list_pending_approvals(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ApprovalResponse>>, ApiError> {
    let approvals = state
        .db
        .list_pending_approvals()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(approvals.into_iter().map(Into::into).collect()))
}

async fn approve_approval(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<ApprovalDecisionRequest>,
) -> Result<Json<ApprovalResponse>, ApiError> {
    req.validate()?;

    let approval_service = ApprovalService::new(state.db.clone());

    let approval = approval_service
        .approve(id, req.approver.clone(), req.comment.clone())
        .await
        .map_err(|e| match e {
            orchestrate_core::Error::Other(msg) if msg.contains("not found") => {
                ApiError::not_found("Approval")
            }
            orchestrate_core::Error::Other(msg)
                if msg.contains("not authorized") || msg.contains("not an authorized") => {
                ApiError::bad_request(msg)
            }
            orchestrate_core::Error::Other(msg) if msg.contains("already resolved") || msg.contains("already submitted") => {
                ApiError::conflict(msg)
            }
            _ => ApiError::internal(format!("Approval error: {}", e)),
        })?;

    Ok(Json(approval.into()))
}

async fn reject_approval(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<ApprovalDecisionRequest>,
) -> Result<Json<ApprovalResponse>, ApiError> {
    req.validate()?;

    let approval_service = ApprovalService::new(state.db.clone());

    let approval = approval_service
        .reject(id, req.approver.clone(), req.comment.clone())
        .await
        .map_err(|e| match e {
            orchestrate_core::Error::Other(msg) if msg.contains("not found") => {
                ApiError::not_found("Approval")
            }
            orchestrate_core::Error::Other(msg)
                if msg.contains("not authorized") || msg.contains("not an authorized") => {
                ApiError::bad_request(msg)
            }
            orchestrate_core::Error::Other(msg) if msg.contains("already resolved") || msg.contains("already submitted") => {
                ApiError::conflict(msg)
            }
            _ => ApiError::internal(format!("Approval error: {}", e)),
        })?;

    Ok(Json(approval.into()))
}

// ==================== Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub agent_type: AgentType,
    pub task: String,
    #[serde(default)]
    pub worktree_id: Option<String>,
}

impl CreateAgentRequest {
    fn validate(&self) -> Result<(), ApiError> {
        // Check task is not empty
        if self.task.trim().is_empty() {
            return Err(ApiError::validation("Task cannot be empty"));
        }

        // Check task length
        if self.task.len() > MAX_TASK_LENGTH {
            return Err(ApiError::validation(format!(
                "Task exceeds maximum length of {} characters",
                MAX_TASK_LENGTH
            )));
        }

        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AgentResponse {
    pub id: String,
    pub agent_type: AgentType,
    pub state: AgentState,
    pub task: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Agent> for AgentResponse {
    fn from(agent: Agent) -> Self {
        Self {
            id: agent.id.to_string(),
            agent_type: agent.agent_type,
            state: agent.state,
            task: agent.task,
            created_at: agent.created_at.to_rfc3339(),
            updated_at: agent.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageResponse {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
}

impl From<orchestrate_core::Message> for MessageResponse {
    fn from(msg: orchestrate_core::Message) -> Self {
        Self {
            id: msg.id,
            role: msg.role.as_str().to_string(),
            content: msg.content,
            created_at: msg.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemStatus {
    pub total_agents: usize,
    pub running_agents: usize,
    pub paused_agents: usize,
    pub completed_agents: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

// ==================== Instruction Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct ListInstructionsParams {
    pub enabled_only: Option<bool>,
    pub scope: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInstructionRequest {
    pub name: String,
    pub content: String,
    pub scope: Option<String>,
    pub agent_type: Option<String>,
    pub priority: Option<i32>,
    pub tags: Option<Vec<String>>,
    pub created_by: Option<String>,
}

impl CreateInstructionRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if self.name.trim().is_empty() {
            return Err(ApiError::validation("Name cannot be empty"));
        }
        if self.content.trim().is_empty() {
            return Err(ApiError::validation("Content cannot be empty"));
        }
        if self.name.len() > 255 {
            return Err(ApiError::validation(
                "Name exceeds maximum length of 255 characters",
            ));
        }
        if self.content.len() > 10_000 {
            return Err(ApiError::validation(
                "Content exceeds maximum length of 10000 characters",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateInstructionRequest {
    pub name: Option<String>,
    pub content: Option<String>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InstructionResponse {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub scope: String,
    pub agent_type: Option<String>,
    pub priority: i32,
    pub enabled: bool,
    pub source: String,
    pub confidence: f64,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub created_by: Option<String>,
}

impl From<CustomInstruction> for InstructionResponse {
    fn from(inst: CustomInstruction) -> Self {
        Self {
            id: inst.id,
            name: inst.name,
            content: inst.content,
            scope: inst.scope.as_str().to_string(),
            agent_type: inst.agent_type.map(|t| t.as_str().to_string()),
            priority: inst.priority,
            enabled: inst.enabled,
            source: inst.source.as_str().to_string(),
            confidence: inst.confidence,
            tags: inst.tags,
            created_at: inst.created_at.to_rfc3339(),
            updated_at: inst.updated_at.to_rfc3339(),
            created_by: inst.created_by,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EffectivenessResponse {
    pub instruction_id: i64,
    pub usage_count: i64,
    pub success_count: i64,
    pub failure_count: i64,
    pub penalty_score: f64,
    pub success_rate: f64,
    pub avg_completion_time: Option<f64>,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub last_penalty_at: Option<String>,
    pub updated_at: String,
}

impl From<InstructionEffectiveness> for EffectivenessResponse {
    fn from(eff: InstructionEffectiveness) -> Self {
        Self {
            instruction_id: eff.instruction_id,
            usage_count: eff.usage_count,
            success_count: eff.success_count,
            failure_count: eff.failure_count,
            penalty_score: eff.penalty_score,
            success_rate: eff.success_rate,
            avg_completion_time: eff.avg_completion_time,
            last_success_at: eff.last_success_at.map(|dt| dt.to_rfc3339()),
            last_failure_at: eff.last_failure_at.map(|dt| dt.to_rfc3339()),
            last_penalty_at: eff.last_penalty_at.map(|dt| dt.to_rfc3339()),
            updated_at: eff.updated_at.to_rfc3339(),
        }
    }
}

// ==================== Pattern Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct ListPatternsParams {
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternResponse {
    pub id: i64,
    pub pattern_type: String,
    pub agent_type: Option<String>,
    pub pattern_signature: String,
    pub pattern_data: serde_json::Value,
    pub occurrence_count: i64,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub instruction_id: Option<i64>,
    pub status: String,
}

impl From<LearningPattern> for PatternResponse {
    fn from(pattern: LearningPattern) -> Self {
        Self {
            id: pattern.id,
            pattern_type: pattern.pattern_type.as_str().to_string(),
            agent_type: pattern.agent_type.map(|t| t.as_str().to_string()),
            pattern_signature: pattern.pattern_signature,
            pattern_data: pattern.pattern_data,
            occurrence_count: pattern.occurrence_count,
            first_seen_at: pattern.first_seen_at.to_rfc3339(),
            last_seen_at: pattern.last_seen_at.to_rfc3339(),
            instruction_id: pattern.instruction_id,
            status: pattern.status.as_str().to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessPatternsResponse {
    pub created_count: usize,
    pub instruction_ids: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CleanupResponse {
    pub disabled_count: usize,
    pub deleted_names: Vec<String>,
}

// ==================== Pipeline Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct CreatePipelineRequest {
    pub name: String,
    pub definition: String,
    pub enabled: Option<bool>,
}

impl CreatePipelineRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if self.name.trim().is_empty() {
            return Err(ApiError::validation("Pipeline name cannot be empty"));
        }
        if self.name.len() > 255 {
            return Err(ApiError::validation(
                "Pipeline name exceeds maximum length of 255 characters",
            ));
        }
        if self.definition.trim().is_empty() {
            return Err(ApiError::validation("Pipeline definition cannot be empty"));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePipelineRequest {
    pub definition: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineResponse {
    pub id: i64,
    pub name: String,
    pub definition: String,
    pub enabled: bool,
    pub created_at: String,
}

impl From<Pipeline> for PipelineResponse {
    fn from(pipeline: Pipeline) -> Self {
        Self {
            id: pipeline.id.unwrap_or(0),
            name: pipeline.name,
            definition: pipeline.definition,
            enabled: pipeline.enabled,
            created_at: pipeline.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TriggerRunRequest {
    pub trigger_event: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineRunResponse {
    pub id: i64,
    pub pipeline_id: i64,
    pub status: String,
    pub trigger_event: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

impl From<PipelineRun> for PipelineRunResponse {
    fn from(run: PipelineRun) -> Self {
        Self {
            id: run.id.unwrap_or(0),
            pipeline_id: run.pipeline_id,
            status: run.status.as_str().to_string(),
            trigger_event: run.trigger_event,
            started_at: run.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: run.completed_at.map(|dt| dt.to_rfc3339()),
            created_at: run.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PipelineStageResponse {
    pub id: i64,
    pub run_id: i64,
    pub stage_name: String,
    pub status: String,
    pub agent_id: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub created_at: String,
}

impl From<PipelineStage> for PipelineStageResponse {
    fn from(stage: PipelineStage) -> Self {
        Self {
            id: stage.id.unwrap_or(0),
            run_id: stage.run_id,
            stage_name: stage.stage_name,
            status: stage.status.as_str().to_string(),
            agent_id: stage.agent_id,
            started_at: stage.started_at.map(|dt| dt.to_rfc3339()),
            completed_at: stage.completed_at.map(|dt| dt.to_rfc3339()),
            created_at: stage.created_at.to_rfc3339(),
        }
    }
}

// ==================== Approval Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct ApprovalDecisionRequest {
    pub approver: String,
    pub comment: Option<String>,
}

impl ApprovalDecisionRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if self.approver.trim().is_empty() {
            return Err(ApiError::validation("Approver cannot be empty"));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub id: i64,
    pub stage_id: i64,
    pub run_id: i64,
    pub status: String,
    pub required_approvers: String,
    pub required_count: i32,
    pub approval_count: i32,
    pub rejection_count: i32,
    pub timeout_seconds: Option<i64>,
    pub timeout_action: Option<String>,
    pub timeout_at: Option<String>,
    pub resolved_at: Option<String>,
    pub created_at: String,
}

impl From<ApprovalRequest> for ApprovalResponse {
    fn from(req: ApprovalRequest) -> Self {
        Self {
            id: req.id.unwrap_or(0),
            stage_id: req.stage_id,
            run_id: req.run_id,
            status: req.status.as_str().to_string(),
            required_approvers: req.required_approvers,
            required_count: req.required_count,
            approval_count: req.approval_count,
            rejection_count: req.rejection_count,
            timeout_seconds: req.timeout_seconds,
            timeout_action: req.timeout_action,
            timeout_at: req.timeout_at.map(|dt| dt.to_rfc3339()),
            resolved_at: req.resolved_at.map(|dt| dt.to_rfc3339()),
            created_at: req.created_at.to_rfc3339(),
        }
    }
}

// ==================== Schedule Handlers ====================

async fn list_schedules(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<ScheduleResponse>>, ApiError> {
    let schedules = state
        .db
        .list_schedules(false)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(schedules.into_iter().map(Into::into).collect()))
}

async fn get_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ScheduleResponse>, ApiError> {
    let schedule = state
        .db
        .get_schedule(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Schedule"))?;

    Ok(Json(schedule.into()))
}

async fn create_schedule(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateScheduleRequest>,
) -> Result<Json<ScheduleResponse>, ApiError> {
    req.validate()?;

    let mut schedule = Schedule::new(
        req.name,
        req.cron_expression,
        req.agent_type,
        req.task,
    );

    schedule
        .validate_cron()
        .map_err(|e| ApiError::validation(format!("Invalid cron expression: {}", e)))?;

    schedule
        .update_next_run()
        .map_err(|e| ApiError::internal(format!("Failed to calculate next run: {}", e)))?;

    if let Some(enabled) = req.enabled {
        schedule.enabled = enabled;
    }

    let id = state
        .db
        .insert_schedule(&schedule)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    schedule.id = id;
    Ok(Json(schedule.into()))
}

async fn update_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<UpdateScheduleRequest>,
) -> Result<Json<ScheduleResponse>, ApiError> {
    let mut schedule = state
        .db
        .get_schedule(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Schedule"))?;

    if let Some(name) = req.name {
        schedule.name = name;
    }
    if let Some(cron_expression) = req.cron_expression {
        schedule.cron_expression = cron_expression;
        schedule
            .validate_cron()
            .map_err(|e| ApiError::validation(format!("Invalid cron expression: {}", e)))?;
        schedule
            .update_next_run()
            .map_err(|e| ApiError::internal(format!("Failed to calculate next run: {}", e)))?;
    }
    if let Some(agent_type) = req.agent_type {
        schedule.agent_type = agent_type;
    }
    if let Some(task) = req.task {
        schedule.task = task;
    }
    if let Some(enabled) = req.enabled {
        schedule.enabled = enabled;
    }

    state
        .db
        .update_schedule(&schedule)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(schedule.into()))
}

async fn delete_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    state
        .db
        .delete_schedule(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

async fn pause_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ScheduleResponse>, ApiError> {
    let mut schedule = state
        .db
        .get_schedule(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Schedule"))?;

    schedule.enabled = false;
    state
        .db
        .update_schedule(&schedule)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(schedule.into()))
}

async fn resume_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ScheduleResponse>, ApiError> {
    let mut schedule = state
        .db
        .get_schedule(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Schedule"))?;

    schedule.enabled = true;
    state
        .db
        .update_schedule(&schedule)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(schedule.into()))
}

async fn run_schedule(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ScheduleRunResponse>, ApiError> {
    let schedule = state
        .db
        .get_schedule(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Schedule"))?;

    let run = ScheduleRun::new(schedule.id);
    let run_id = state
        .db
        .insert_schedule_run(&run)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Retrieve the created run from the list of runs
    let runs = state
        .db
        .get_schedule_runs(schedule.id, 1)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let run = runs
        .into_iter()
        .find(|r| r.id == run_id)
        .ok_or_else(|| ApiError::internal("Failed to retrieve created run".to_string()))?;

    Ok(Json(run.into()))
}

async fn get_schedule_runs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<ScheduleRunResponse>>, ApiError> {
    let runs = state
        .db
        .get_schedule_runs(id, 50)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(runs.into_iter().map(Into::into).collect()))
}

// ==================== Schedule Request/Response Types ====================

#[derive(Debug, Deserialize)]
struct CreateScheduleRequest {
    name: String,
    cron_expression: String,
    agent_type: String,
    task: String,
    enabled: Option<bool>,
}

impl CreateScheduleRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if self.name.is_empty() || self.name.len() > 255 {
            return Err(ApiError::validation("Name must be 1-255 characters"));
        }
        if self.cron_expression.is_empty() {
            return Err(ApiError::validation("Cron expression is required"));
        }
        if self.agent_type.is_empty() {
            return Err(ApiError::validation("Agent type is required"));
        }
        if self.task.is_empty() || self.task.len() > MAX_TASK_LENGTH {
            return Err(ApiError::validation(format!(
                "Task must be 1-{} characters",
                MAX_TASK_LENGTH
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct UpdateScheduleRequest {
    name: Option<String>,
    cron_expression: Option<String>,
    agent_type: Option<String>,
    task: Option<String>,
    enabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct ScheduleResponse {
    id: i64,
    name: String,
    cron_expression: String,
    agent_type: String,
    task: String,
    enabled: bool,
    next_run_at: Option<String>,
    last_run_at: Option<String>,
    created_at: String,
}

impl From<Schedule> for ScheduleResponse {
    fn from(schedule: Schedule) -> Self {
        Self {
            id: schedule.id,
            name: schedule.name,
            cron_expression: schedule.cron_expression,
            agent_type: schedule.agent_type,
            task: schedule.task,
            enabled: schedule.enabled,
            next_run_at: schedule.next_run.map(|dt| dt.to_rfc3339()),
            last_run_at: schedule.last_run.map(|dt| dt.to_rfc3339()),
            created_at: schedule.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
struct ScheduleRunResponse {
    id: i64,
    schedule_id: i64,
    status: String,
    agent_id: Option<String>,
    error_message: Option<String>,
    started_at: String,
    completed_at: Option<String>,
}

impl From<ScheduleRun> for ScheduleRunResponse {
    fn from(run: ScheduleRun) -> Self {
        Self {
            id: run.id,
            schedule_id: run.schedule_id,
            status: run.status.as_str().to_string(),
            agent_id: run.agent_id,
            error_message: run.error_message,
            started_at: run.started_at.to_rfc3339(),
            completed_at: run.completed_at.map(|dt| dt.to_rfc3339()),
        }
    }
}

// ==================== Feedback Handlers ====================

#[derive(Debug, Deserialize)]
struct CreateFeedbackRequest {
    agent_id: String,
    rating: String,
    #[serde(default)]
    comment: Option<String>,
    #[serde(default)]
    message_id: Option<i64>,
}

#[derive(Debug, Serialize)]
struct FeedbackResponse {
    id: i64,
    agent_id: String,
    message_id: Option<i64>,
    rating: String,
    comment: Option<String>,
    source: String,
    created_by: String,
    created_at: String,
}

impl From<Feedback> for FeedbackResponse {
    fn from(fb: Feedback) -> Self {
        Self {
            id: fb.id,
            agent_id: fb.agent_id.to_string(),
            message_id: fb.message_id,
            rating: fb.rating.as_str().to_string(),
            comment: fb.comment,
            source: fb.source.as_str().to_string(),
            created_by: fb.created_by,
            created_at: fb.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize)]
struct FeedbackStatsResponse {
    total: i64,
    positive: i64,
    negative: i64,
    neutral: i64,
    score: f64,
    positive_percentage: f64,
}

impl From<FeedbackStats> for FeedbackStatsResponse {
    fn from(stats: FeedbackStats) -> Self {
        Self {
            total: stats.total,
            positive: stats.positive,
            negative: stats.negative,
            neutral: stats.neutral,
            score: stats.score,
            positive_percentage: stats.positive_percentage,
        }
    }
}

#[derive(Debug, Deserialize)]
struct FeedbackListQuery {
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    rating: Option<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default = "default_feedback_limit")]
    limit: i64,
}

fn default_feedback_limit() -> i64 {
    50
}

async fn create_feedback(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateFeedbackRequest>,
) -> Result<Json<FeedbackResponse>, ApiError> {
    use std::str::FromStr;

    // Parse agent ID
    let agent_uuid = Uuid::parse_str(&req.agent_id)
        .map_err(|e| ApiError::validation(format!("Invalid agent ID: {}", e)))?;

    // Verify agent exists
    if state.db.get_agent(agent_uuid).await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .is_none()
    {
        return Err(ApiError::not_found("Agent"));
    }

    // Parse rating
    let rating = FeedbackRating::from_str(&req.rating)
        .map_err(|_| ApiError::validation("Invalid rating. Use: positive, negative, neutral"))?;

    // Build feedback
    let mut feedback = Feedback::new(agent_uuid, rating, "api")
        .with_source(FeedbackSource::Api);

    if let Some(msg_id) = req.message_id {
        feedback = feedback.with_message_id(msg_id);
    }

    if let Some(comment) = req.comment {
        feedback = feedback.with_comment(comment);
    }

    // Insert feedback
    let id = state.db.insert_feedback(&feedback).await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    // Retrieve the created feedback
    let created = state.db.get_feedback(id).await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::internal("Failed to retrieve created feedback".to_string()))?;

    Ok(Json(created.into()))
}

async fn list_feedback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FeedbackListQuery>,
) -> Result<Json<Vec<FeedbackResponse>>, ApiError> {
    use std::str::FromStr;

    let feedbacks = if let Some(agent_id) = query.agent_id {
        let agent_uuid = Uuid::parse_str(&agent_id)
            .map_err(|e| ApiError::validation(format!("Invalid agent ID: {}", e)))?;
        state.db.list_feedback_for_agent(agent_uuid, query.limit).await
    } else {
        let rating_filter = query.rating
            .as_ref()
            .map(|r| FeedbackRating::from_str(r))
            .transpose()
            .map_err(|_| ApiError::validation("Invalid rating filter"))?;
        let source_filter = query.source
            .as_ref()
            .map(|s| FeedbackSource::from_str(s))
            .transpose()
            .map_err(|_| ApiError::validation("Invalid source filter"))?;
        state.db.list_feedback(rating_filter, source_filter, query.limit).await
    };

    let feedbacks = feedbacks
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(feedbacks.into_iter().map(Into::into).collect()))
}

async fn get_feedback(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<FeedbackResponse>, ApiError> {
    let feedback = state.db.get_feedback(id).await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Feedback"))?;

    Ok(Json(feedback.into()))
}

async fn delete_feedback(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    let deleted = state.db.delete_feedback(id).await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("Feedback"))
    }
}

async fn get_feedback_stats(
    State(state): State<Arc<AppState>>,
    Query(query): Query<FeedbackStatsQuery>,
) -> Result<Json<FeedbackStatsResponse>, ApiError> {
    let stats = if let Some(agent_id) = query.agent_id {
        let agent_uuid = Uuid::parse_str(&agent_id)
            .map_err(|e| ApiError::validation(format!("Invalid agent ID: {}", e)))?;
        state.db.get_feedback_stats_for_agent(agent_uuid).await
    } else {
        state.db.get_feedback_stats().await
    };

    let stats = stats
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(stats.into()))
}

#[derive(Debug, Deserialize)]
struct FeedbackStatsQuery {
    #[serde(default)]
    agent_id: Option<String>,
}

// ==================== Learning Analytics Handlers ====================

#[derive(Debug, Serialize)]
struct LearningEffectivenessResponse {
    instructions: Vec<InstructionEffectivenessItem>,
    summary: EffectivenessSummaryItem,
}

#[derive(Debug, Serialize)]
struct InstructionEffectivenessItem {
    instruction_id: i64,
    name: String,
    source: String,
    enabled: bool,
    usage_count: i64,
    success_rate: f64,
    penalty_score: f64,
    level: String,
}

#[derive(Debug, Serialize)]
struct EffectivenessSummaryItem {
    total_instructions: i64,
    enabled_count: i64,
    used_count: i64,
    total_usage: i64,
    avg_success_rate: f64,
    avg_penalty_score: f64,
    ineffective_count: i64,
}

async fn get_learning_effectiveness(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EffectivenessQuery>,
) -> Result<Json<LearningEffectivenessResponse>, ApiError> {
    let instructions = state
        .db
        .list_instruction_effectiveness(query.include_disabled.unwrap_or(false), query.min_usage.unwrap_or(1))
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let summary = state
        .db
        .get_effectiveness_summary()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let items: Vec<InstructionEffectivenessItem> = instructions
        .into_iter()
        .map(|i| InstructionEffectivenessItem {
            instruction_id: i.instruction_id,
            name: i.name.clone(),
            source: i.instruction_source.clone(),
            enabled: i.enabled,
            usage_count: i.usage_count,
            success_rate: i.success_rate,
            penalty_score: i.penalty_score,
            level: i.effectiveness_level().to_string(),
        })
        .collect();

    Ok(Json(LearningEffectivenessResponse {
        instructions: items,
        summary: EffectivenessSummaryItem {
            total_instructions: summary.total_instructions,
            enabled_count: summary.enabled_count,
            used_count: summary.used_count,
            total_usage: summary.total_usage,
            avg_success_rate: summary.avg_success_rate,
            avg_penalty_score: summary.avg_penalty_score,
            ineffective_count: summary.ineffective_count,
        },
    }))
}

#[derive(Debug, Deserialize)]
struct EffectivenessQuery {
    #[serde(default)]
    min_usage: Option<i64>,
    #[serde(default)]
    include_disabled: Option<bool>,
}

#[derive(Debug, Serialize)]
struct SuggestionResponse {
    suggestions: Vec<SuggestionItem>,
    total: usize,
}

#[derive(Debug, Serialize)]
struct SuggestionItem {
    instruction_id: i64,
    name: String,
    success_rate: f64,
    usage_count: i64,
    suggestion: String,
}

async fn get_learning_suggestions(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SuggestionsQuery>,
) -> Result<Json<SuggestionResponse>, ApiError> {
    let ineffective = state
        .db
        .list_ineffective_instructions(0.5, 5)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let limit = query.limit.unwrap_or(10);
    let suggestions: Vec<SuggestionItem> = ineffective
        .into_iter()
        .take(limit)
        .map(|i| SuggestionItem {
            instruction_id: i.instruction_id,
            name: i.name.clone(),
            success_rate: i.success_rate,
            usage_count: i.usage_count,
            suggestion: "Review and update instruction content or disable if no longer relevant".to_string(),
        })
        .collect();

    let total = suggestions.len();
    Ok(Json(SuggestionResponse { suggestions, total }))
}

#[derive(Debug, Deserialize)]
struct SuggestionsQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
struct AnalysisResponse {
    patterns_processed: usize,
    instructions_created: Vec<String>,
}

async fn trigger_learning_analysis(
    State(state): State<Arc<AppState>>,
) -> Result<Json<AnalysisResponse>, ApiError> {
    let engine = LearningEngine::new();
    let created = engine
        .process_patterns(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("Analysis error: {}", e)))?;

    Ok(Json(AnalysisResponse {
        patterns_processed: created.len(),
        instructions_created: created.iter().map(|i| i.name.clone()).collect(),
    }))
}

// ==================== Experiment Handlers ====================

use orchestrate_core::{Experiment, ExperimentStatus, ExperimentType, ExperimentMetric};

#[derive(Debug, Serialize)]
struct ExperimentResponse {
    id: i64,
    name: String,
    description: Option<String>,
    experiment_type: String,
    metric: String,
    status: String,
    min_samples: i64,
    confidence_level: f64,
    created_at: String,
}

impl From<Experiment> for ExperimentResponse {
    fn from(exp: Experiment) -> Self {
        Self {
            id: exp.id,
            name: exp.name,
            description: exp.description,
            experiment_type: exp.experiment_type.as_str().to_string(),
            metric: exp.metric.as_str().to_string(),
            status: exp.status.as_str().to_string(),
            min_samples: exp.min_samples,
            confidence_level: exp.confidence_level,
            created_at: exp.created_at.to_rfc3339(),
        }
    }
}

async fn list_experiments(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ExperimentListQuery>,
) -> Result<Json<Vec<ExperimentResponse>>, ApiError> {
    let status_filter = query.status.as_ref().and_then(|s| {
        use std::str::FromStr;
        ExperimentStatus::from_str(s).ok()
    });

    let experiments = state
        .db
        .list_experiments(status_filter, query.limit.unwrap_or(100))
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(experiments.into_iter().map(Into::into).collect()))
}

#[derive(Debug, Deserialize)]
struct ExperimentListQuery {
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CreateExperimentRequest {
    name: String,
    description: Option<String>,
    experiment_type: String,
    metric: String,
    min_samples: Option<i64>,
    confidence_level: Option<f64>,
}

async fn create_experiment(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateExperimentRequest>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    use std::str::FromStr;

    let experiment_type = ExperimentType::from_str(&req.experiment_type)
        .map_err(|_| ApiError::validation(format!("Invalid experiment type: {}", req.experiment_type)))?;

    let metric = ExperimentMetric::from_str(&req.metric)
        .map_err(|_| ApiError::validation(format!("Invalid metric: {}", req.metric)))?;

    let experiment = Experiment {
        id: 0,
        name: req.name,
        description: req.description,
        hypothesis: None,
        experiment_type,
        metric,
        agent_type: None,
        status: ExperimentStatus::Draft,
        min_samples: req.min_samples.unwrap_or(100),
        confidence_level: req.confidence_level.unwrap_or(0.95),
        created_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
        winner_variant_id: None,
    };

    let id = state
        .db
        .create_experiment(&experiment)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let created = state
        .db
        .get_experiment(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::internal("Failed to retrieve created experiment"))?;

    Ok(Json(created.into()))
}

async fn get_experiment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ExperimentResponse>, ApiError> {
    let experiment = state
        .db
        .get_experiment(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Experiment"))?;

    Ok(Json(experiment.into()))
}

#[derive(Debug, Serialize)]
struct ExperimentResultsResponse {
    experiment_id: i64,
    variants: Vec<VariantResultItem>,
    is_significant: bool,
    p_value: Option<f64>,
    improvement: Option<f64>,
    winning_variant: Option<String>,
}

#[derive(Debug, Serialize)]
struct VariantResultItem {
    variant_id: i64,
    name: String,
    is_control: bool,
    sample_count: i64,
    mean: f64,
    std_dev: f64,
    success_rate: f64,
}

async fn get_experiment_results(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<ExperimentResultsResponse>, ApiError> {
    let experiment = state
        .db
        .get_experiment(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Experiment"))?;

    let results = state
        .db
        .get_experiment_results(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let variants: Vec<VariantResultItem> = results
        .iter()
        .map(|v| VariantResultItem {
            variant_id: v.variant_id,
            name: v.variant_name.clone(),
            is_control: v.is_control,
            sample_count: v.sample_count,
            mean: v.mean,
            std_dev: v.std_dev,
            success_rate: v.success_rate.unwrap_or(0.0),
        })
        .collect();

    // Calculate significance if we have control and treatment
    let (is_significant, p_value, improvement, winning_variant) = if let (Some(control), Some(treatment)) =
        (results.iter().find(|v| v.is_control), results.iter().find(|v| !v.is_control))
    {
        use orchestrate_core::ExperimentResults;
        let (sig, pval) = ExperimentResults::calculate_significance(control, treatment, experiment.confidence_level);
        let imp = ExperimentResults::calculate_improvement(control.mean, treatment.mean);
        let winner = if sig {
            if treatment.mean > control.mean {
                Some(treatment.variant_name.clone())
            } else {
                Some(control.variant_name.clone())
            }
        } else {
            None
        };
        (sig, Some(pval), Some(imp), winner)
    } else {
        (false, None, None, None)
    };

    Ok(Json(ExperimentResultsResponse {
        experiment_id: id,
        variants,
        is_significant,
        p_value,
        improvement,
        winning_variant,
    }))
}

#[derive(Debug, Deserialize)]
struct PromoteExperimentRequest {
    winner_variant_id: i64,
}

async fn promote_experiment(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
    Json(req): Json<PromoteExperimentRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let experiment = state
        .db
        .get_experiment(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Experiment"))?;

    // Complete the experiment with the winner
    state
        .db
        .update_experiment_status(id, ExperimentStatus::Completed)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(serde_json::json!({
        "message": format!("Experiment '{}' completed with variant {} as winner", experiment.name, req.winner_variant_id),
        "experiment_id": id,
        "winner_variant_id": req.winner_variant_id
    })))
}

// ==================== Prediction Handlers ====================

#[derive(Debug, Deserialize)]
struct PredictionRequest {
    task: String,
    agent_type: Option<String>,
}

#[derive(Debug, Serialize)]
struct PredictionResponse {
    task_description: String,
    success_probability: f64,
    confidence: f64,
    estimated_tokens: TokenEstimateItem,
    estimated_duration: DurationEstimateItem,
    recommended_model: String,
    risk_factors: Vec<RiskFactorItem>,
    recommendations: Vec<String>,
}

#[derive(Debug, Serialize)]
struct TokenEstimateItem {
    min: i64,
    max: i64,
    expected: i64,
}

#[derive(Debug, Serialize)]
struct DurationEstimateItem {
    min_minutes: f64,
    max_minutes: f64,
    expected_minutes: f64,
}

#[derive(Debug, Serialize)]
struct RiskFactorItem {
    name: String,
    description: String,
    severity: String,
    impact_on_success: f64,
}

async fn get_prediction(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PredictionRequest>,
) -> Result<Json<PredictionResponse>, ApiError> {
    use orchestrate_core::predict_task_outcome;

    // Parse agent type if provided
    let agent_type_parsed = req.agent_type.as_ref().and_then(|t| {
        use std::str::FromStr;
        AgentType::from_str(t).ok()
    });

    // Get historical data for prediction
    let agents = state
        .db
        .list_agents_paginated(1000, 0, None, agent_type_parsed)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let total_agents = agents.len();
    let successful = agents
        .iter()
        .filter(|a| a.state == AgentState::Completed)
        .count();
    let historical_success_rate = if total_agents > 0 {
        successful as f64 / total_agents as f64
    } else {
        0.75 // Default assumption
    };

    let prediction = predict_task_outcome(
        &req.task,
        historical_success_rate,
        50000, // Default token estimate
        30.0,  // Default duration estimate
        total_agents as i64,
    );

    Ok(Json(PredictionResponse {
        task_description: prediction.task_description,
        success_probability: prediction.success_probability,
        confidence: prediction.confidence,
        estimated_tokens: TokenEstimateItem {
            min: prediction.estimated_tokens.min,
            max: prediction.estimated_tokens.max,
            expected: prediction.estimated_tokens.expected,
        },
        estimated_duration: DurationEstimateItem {
            min_minutes: prediction.estimated_duration.min_minutes,
            max_minutes: prediction.estimated_duration.max_minutes,
            expected_minutes: prediction.estimated_duration.expected_minutes,
        },
        recommended_model: prediction.recommended_model,
        risk_factors: prediction
            .risk_factors
            .into_iter()
            .map(|r| RiskFactorItem {
                name: r.name,
                description: r.description,
                severity: r.severity.as_str().to_string(),
                impact_on_success: r.impact_on_success,
            })
            .collect(),
        recommendations: prediction.recommendations,
    }))
}

// ==================== Documentation Handlers ====================

#[derive(Debug, Serialize, Deserialize)]
struct DocGenerateRequest {
    doc_type: String,
    format: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DocGenerateResponse {
    doc_type: String,
    format: String,
    content: String,
    generated_at: String,
}

async fn generate_documentation(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<DocGenerateRequest>,
) -> Result<Json<DocGenerateResponse>, ApiError> {
    use orchestrate_core::{ApiDocumentation, ApiEndpoint, DocType, ReadmeContent, ReadmeSection};

    let format = req.format.unwrap_or_else(|| "yaml".to_string());

    let content = match req.doc_type.to_lowercase().as_str() {
        "api" => {
            let mut api_doc = ApiDocumentation::new(
                "Orchestrate API",
                "1.0.0",
                Some("Agent orchestration and automation API"),
            );
            api_doc.add_server("http://localhost:8080", Some("Development server"));

            // Add basic endpoints
            api_doc.add_endpoint(
                ApiEndpoint::new("GET", "/api/agents")
                    .with_summary("List all agents")
                    .with_tag("agents"),
            );
            api_doc.add_endpoint(
                ApiEndpoint::new("GET", "/api/agents/{id}")
                    .with_summary("Get agent by ID")
                    .with_tag("agents")
                    .with_path_param("id", Some("Agent UUID")),
            );
            api_doc.add_endpoint(
                ApiEndpoint::new("POST", "/api/agents")
                    .with_summary("Create a new agent")
                    .with_tag("agents"),
            );
            api_doc.add_endpoint(
                ApiEndpoint::new("GET", "/api/sessions")
                    .with_summary("List sessions")
                    .with_tag("sessions"),
            );
            api_doc.add_endpoint(
                ApiEndpoint::new("GET", "/api/prs")
                    .with_summary("List pull requests")
                    .with_tag("pull-requests"),
            );

            match format.to_lowercase().as_str() {
                "yaml" | "yml" => api_doc.to_openapi_yaml(),
                "json" => serde_json::to_string_pretty(&api_doc.to_openapi_json())
                    .map_err(|e| ApiError::internal(format!("JSON error: {}", e)))?,
                _ => return Err(ApiError::bad_request(format!("Unknown format: {}", format))),
            }
        }
        "readme" => {
            let mut readme = ReadmeContent {
                project_name: "Orchestrate".to_string(),
                description: Some("An agent orchestration and automation system".to_string()),
                sections: vec![],
            };

            readme.add_section(
                ReadmeSection::Installation,
                "```bash\ncargo install orchestrate\n```".to_string(),
            );
            readme.add_section(
                ReadmeSection::Usage,
                "```bash\norchestrate daemon start\norchestrate agent create --type story-developer --task \"Implement feature\"\n```".to_string(),
            );

            readme.to_markdown()
        }
        _ => {
            return Err(ApiError::bad_request(format!(
                "Unknown doc type: {}. Valid: api, readme",
                req.doc_type
            )))
        }
    };

    Ok(Json(DocGenerateResponse {
        doc_type: req.doc_type,
        format,
        content,
        generated_at: chrono::Utc::now().to_rfc3339(),
    }))
}

#[derive(Debug, Serialize, Deserialize)]
struct DocValidateRequest {
    path: Option<String>,
    coverage_threshold: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DocValidateResponse {
    total_items: usize,
    documented_items: usize,
    coverage_percentage: f64,
    issues_count: usize,
    is_valid: bool,
    summary: String,
}

async fn validate_documentation(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<DocValidateRequest>,
) -> Result<Json<DocValidateResponse>, ApiError> {
    use orchestrate_core::DocValidationResult;

    let _check_path = req.path.unwrap_or_else(|| ".".to_string());
    let _threshold = req.coverage_threshold.unwrap_or(80);

    // Create a mock validation result for now
    let mut result = DocValidationResult::new();
    result.total_items = 100;
    result.documented_items = 85;
    result.calculate_coverage();

    Ok(Json(DocValidateResponse {
        total_items: result.total_items,
        documented_items: result.documented_items,
        coverage_percentage: result.coverage_percentage,
        issues_count: result.issues.len(),
        is_valid: result.is_valid(),
        summary: result.to_summary(),
    }))
}

#[derive(Debug, Serialize, Deserialize)]
struct AdrListItem {
    number: u32,
    title: String,
    status: String,
    file_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AdrListResponse {
    adrs: Vec<AdrListItem>,
    total: usize,
}

async fn list_adrs(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<AdrListResponse>, ApiError> {
    let adr_dir = std::path::Path::new("docs/adrs");
    let mut adrs = vec![];

    if adr_dir.exists() {
        let entries: Vec<_> = std::fs::read_dir(adr_dir)
            .map_err(|e| ApiError::internal(format!("Failed to read ADR directory: {}", e)))?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().ends_with(".md"))
            .collect();

        for entry in entries {
            let content = std::fs::read_to_string(entry.path())
                .map_err(|e| ApiError::internal(format!("Failed to read ADR: {}", e)))?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            // Parse number from filename
            let number = name_str
                .strip_prefix("adr-")
                .and_then(|s| s.strip_suffix(".md"))
                .and_then(|s| s.parse().ok())
                .unwrap_or(0);

            // Parse title from first line
            let title = content
                .lines()
                .next()
                .unwrap_or("")
                .trim_start_matches("# ")
                .to_string();

            // Parse status
            let status = content
                .lines()
                .skip_while(|l| !l.starts_with("## Status"))
                .nth(2)
                .unwrap_or("Unknown")
                .to_string();

            adrs.push(AdrListItem {
                number,
                title,
                status,
                file_path: entry.path().to_string_lossy().to_string(),
            });
        }
    }

    adrs.sort_by_key(|a| a.number);
    let total = adrs.len();

    Ok(Json(AdrListResponse { adrs, total }))
}

#[derive(Debug, Serialize, Deserialize)]
struct AdrCreateRequest {
    title: String,
    status: Option<String>,
    context: Option<String>,
    decision: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AdrCreateResponse {
    number: u32,
    title: String,
    status: String,
    file_path: String,
}

async fn create_adr(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<AdrCreateRequest>,
) -> Result<Json<AdrCreateResponse>, ApiError> {
    use orchestrate_core::{Adr, AdrStatus};
    use std::str::FromStr;

    let status_str = req.status.unwrap_or_else(|| "proposed".to_string());
    let adr_status = AdrStatus::from_str(&status_str)
        .map_err(|_| ApiError::bad_request(format!("Invalid status: {}", status_str)))?;

    // Find the next ADR number
    let adr_dir = std::path::Path::new("docs/adrs");
    std::fs::create_dir_all(adr_dir)
        .map_err(|e| ApiError::internal(format!("Failed to create ADR directory: {}", e)))?;

    let mut max_number = 0u32;
    if adr_dir.exists() {
        for entry in std::fs::read_dir(adr_dir)
            .map_err(|e| ApiError::internal(format!("Failed to read directory: {}", e)))?
        {
            if let Ok(entry) = entry {
                let name = entry.file_name();
                let name = name.to_string_lossy();
                if let Some(num_str) = name
                    .strip_prefix("adr-")
                    .and_then(|s| s.strip_suffix(".md"))
                {
                    if let Ok(num) = num_str.parse::<u32>() {
                        max_number = max_number.max(num);
                    }
                }
            }
        }
    }
    let adr_number = max_number + 1;

    let adr = Adr {
        number: adr_number,
        title: req.title.clone(),
        status: adr_status,
        date: chrono::Utc::now(),
        context: req.context.unwrap_or_default(),
        decision: req.decision.unwrap_or_default(),
        consequences: vec![],
        related_adrs: vec![],
        superseded_by: None,
        tags: vec![],
    };

    let file_path = adr_dir.join(format!("adr-{:04}.md", adr_number));
    std::fs::write(&file_path, adr.to_markdown())
        .map_err(|e| ApiError::internal(format!("Failed to write ADR: {}", e)))?;

    Ok(Json(AdrCreateResponse {
        number: adr_number,
        title: req.title,
        status: status_str,
        file_path: file_path.to_string_lossy().to_string(),
    }))
}

async fn get_adr(
    State(_state): State<Arc<AppState>>,
    Path(number): Path<u32>,
) -> Result<String, ApiError> {
    let adr_path = std::path::Path::new("docs/adrs").join(format!("adr-{:04}.md", number));
    if !adr_path.exists() {
        return Err(ApiError::not_found(format!("ADR not found: adr-{:04}", number)));
    }
    std::fs::read_to_string(&adr_path)
        .map_err(|e| ApiError::internal(format!("Failed to read ADR: {}", e)))
}

#[derive(Debug, Serialize, Deserialize)]
struct AdrUpdateRequest {
    status: Option<String>,
    superseded_by: Option<u32>,
}

async fn update_adr(
    State(_state): State<Arc<AppState>>,
    Path(number): Path<u32>,
    Json(req): Json<AdrUpdateRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let adr_path = std::path::Path::new("docs/adrs").join(format!("adr-{:04}.md", number));
    if !adr_path.exists() {
        return Err(ApiError::not_found(format!("ADR not found: adr-{:04}", number)));
    }

    let content = std::fs::read_to_string(&adr_path)
        .map_err(|e| ApiError::internal(format!("Failed to read ADR: {}", e)))?;

    let mut new_content = String::new();
    let mut in_status_section = false;
    let mut status_updated = false;

    for line in content.lines() {
        if line.starts_with("## Status") {
            in_status_section = true;
            new_content.push_str(line);
            new_content.push('\n');
        } else if in_status_section && !status_updated && !line.is_empty() {
            if let Some(ref new_status) = req.status {
                let mut status_line = new_status.clone();
                if let Some(by) = req.superseded_by {
                    status_line.push_str(&format!(" by [ADR-{:04}](./adr-{:04}.md)", by, by));
                }
                new_content.push_str(&status_line);
                new_content.push('\n');
            } else {
                new_content.push_str(line);
                new_content.push('\n');
            }
            status_updated = true;
            in_status_section = false;
        } else {
            new_content.push_str(line);
            new_content.push('\n');
        }
    }

    std::fs::write(&adr_path, new_content)
        .map_err(|e| ApiError::internal(format!("Failed to write ADR: {}", e)))?;

    Ok(Json(serde_json::json!({
        "success": true,
        "number": number,
        "status": req.status.unwrap_or_else(|| "unchanged".to_string())
    })))
}

#[derive(Debug, Serialize, Deserialize)]
struct ChangelogRequest {
    from: Option<String>,
    to: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChangelogResponse {
    version: String,
    date: String,
    entries: Vec<ChangelogEntryItem>,
    markdown: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ChangelogEntryItem {
    change_type: String,
    description: String,
    commit_hash: Option<String>,
    author: Option<String>,
    breaking: bool,
}

async fn generate_changelog(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<ChangelogRequest>,
) -> Result<Json<ChangelogResponse>, ApiError> {
    use orchestrate_core::{ChangelogEntry, ChangelogRelease, ChangeType};

    let from_ref = req.from.unwrap_or_else(|| "HEAD~20".to_string());
    let to_ref = req.to.unwrap_or_else(|| "HEAD".to_string());

    // Get git log
    let git_output = std::process::Command::new("git")
        .args([
            "log",
            "--oneline",
            "--pretty=format:%s|%H|%an",
            &format!("{}..{}", from_ref, to_ref),
        ])
        .output()
        .map_err(|e| ApiError::internal(format!("Failed to run git: {}", e)))?;

    let log_output = String::from_utf8_lossy(&git_output.stdout);
    let mut entries = vec![];
    let mut entry_items = vec![];

    for line in log_output.lines() {
        let parts: Vec<&str> = line.splitn(3, '|').collect();
        if parts.len() >= 3 {
            let message = parts[0];
            let hash = parts[1];
            let author = parts[2];

            // Parse conventional commit
            let change_type = if message.starts_with("feat") {
                Some(ChangeType::Added)
            } else if message.starts_with("fix") {
                Some(ChangeType::Fixed)
            } else if message.starts_with("docs") {
                Some(ChangeType::Changed)
            } else if message.starts_with("refactor") || message.starts_with("chore") {
                Some(ChangeType::Changed)
            } else {
                None
            };

            if let Some(ct) = change_type {
                let desc = message
                    .split(':')
                    .nth(1)
                    .map(|s| s.trim())
                    .unwrap_or(message)
                    .to_string();
                let breaking = message.contains("BREAKING");

                entries.push(ChangelogEntry {
                    change_type: ct,
                    description: desc.clone(),
                    commit_hash: Some(hash[..7].to_string()),
                    pr_number: None,
                    issue_number: None,
                    author: Some(author.to_string()),
                    scope: None,
                    breaking,
                });

                entry_items.push(ChangelogEntryItem {
                    change_type: ct.as_str().to_string(),
                    description: desc,
                    commit_hash: Some(hash[..7].to_string()),
                    author: Some(author.to_string()),
                    breaking,
                });
            }
        }
    }

    let release = ChangelogRelease {
        version: "Unreleased".to_string(),
        date: chrono::Utc::now(),
        entries,
        yanked: false,
    };

    Ok(Json(ChangelogResponse {
        version: release.version.clone(),
        date: release.date.to_rfc3339(),
        entries: entry_items,
        markdown: release.to_markdown(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use orchestrate_core::Database;
    use tower::util::ServiceExt;

    struct TestApp {
        router: Router,
        state: Arc<AppState>,
    }

    async fn setup_app() -> TestApp {
        // Use in-memory DB for simpler testing with single connection
        let db = Database::in_memory().await.unwrap();
        let state = Arc::new(AppState::new(db, None));
        let router = create_api_router(state.clone());
        TestApp { router, state }
    }

    async fn setup_app_with_auth(api_key: &str) -> TestApp {
        let db = Database::in_memory().await.unwrap();
        let state = Arc::new(AppState::new(db, Some(api_key.to_string())));
        let router = create_api_router(state.clone());
        TestApp { router, state }
    }

    async fn body_to_string(body: Body) -> String {
        let bytes = body.collect().await.unwrap().to_bytes();
        String::from_utf8(bytes.to_vec()).unwrap()
    }

    /// Helper to properly transition agent to Running state
    fn make_running(agent: &mut Agent) {
        agent.transition_to(AgentState::Initializing).unwrap();
        agent.transition_to(AgentState::Running).unwrap();
    }

    // ==================== Health Check Tests ====================

    #[tokio::test]
    async fn test_health_check() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let health: HealthResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(health.status, "ok");
        assert!(!health.version.is_empty());
    }

    // ==================== Authentication Tests ====================

    #[tokio::test]
    async fn test_auth_no_key_configured_allows_access() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/agents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_missing_key_returns_unauthorized() {
        let test_app = setup_app_with_auth("secret-key").await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/agents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_wrong_key_returns_unauthorized() {
        let test_app = setup_app_with_auth("secret-key").await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/agents")
                    .header("x-api-key", "wrong-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_auth_correct_key_allows_access() {
        let test_app = setup_app_with_auth("secret-key").await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/agents")
                    .header("x-api-key", "secret-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_auth_bearer_token_works() {
        let test_app = setup_app_with_auth("secret-key").await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/agents")
                    .header("authorization", "Bearer secret-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // ==================== Agent CRUD Tests ====================

    #[tokio::test]
    async fn test_list_agents_empty() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/agents")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let agents: Vec<AgentResponse> = serde_json::from_str(&body).unwrap();
        assert!(agents.is_empty());
    }

    #[tokio::test]
    async fn test_create_agent_success() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"agent_type":"story_developer","task":"Build feature X"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let agent: AgentResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(agent.task, "Build feature X");
        assert_eq!(agent.agent_type, AgentType::StoryDeveloper);
        assert_eq!(agent.state, AgentState::Created);
    }

    #[tokio::test]
    async fn test_create_agent_empty_task_fails() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"agent_type":"story_developer","task":""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = body_to_string(response.into_body()).await;
        let error: ApiError = serde_json::from_str(&body).unwrap();
        assert_eq!(error.code, "validation_error");
    }

    #[tokio::test]
    async fn test_create_agent_whitespace_task_fails() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"agent_type":"story_developer","task":"   "}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_agent_task_too_long_fails() {
        let test_app = setup_app().await;

        let long_task = "x".repeat(MAX_TASK_LENGTH + 1);
        let body = format!(
            r#"{{"agent_type":"story_developer","task":"{}"}}"#,
            long_task
        );

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_agent_success() {
        let test_app = setup_app().await;

        // Create agent directly in DB
        let agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        let agent_id = agent.id.to_string();
        test_app.state.db.insert_agent(&agent).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/agents/{}", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: AgentResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.id, agent_id);
        assert_eq!(resp.task, "Test task");
    }

    #[tokio::test]
    async fn test_get_agent_not_found() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/agents/00000000-0000-0000-0000-000000000000")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_agent_invalid_uuid() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/agents/not-a-uuid")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Axum returns 400 for invalid path params, but our route might not match
        // Invalid UUID format in path should return 400 (bad request)
        assert!(
            response.status() == StatusCode::BAD_REQUEST
                || response.status() == StatusCode::NOT_FOUND
        );
    }

    // ==================== Agent Action Tests ====================

    #[tokio::test]
    async fn test_pause_running_agent() {
        let test_app = setup_app().await;

        // Create and start agent (Created -> Initializing -> Running)
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        make_running(&mut agent);
        let agent_id = agent.id.to_string();
        test_app.state.db.insert_agent(&agent).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/agents/{}/pause", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: AgentResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.state, AgentState::Paused);
    }

    #[tokio::test]
    async fn test_pause_completed_agent_fails() {
        let test_app = setup_app().await;

        // Create completed agent
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        make_running(&mut agent);
        agent.transition_to(AgentState::Completed).unwrap();
        let agent_id = agent.id.to_string();
        test_app.state.db.insert_agent(&agent).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/agents/{}/pause", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_resume_paused_agent() {
        let test_app = setup_app().await;

        // Create paused agent
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        make_running(&mut agent);
        agent.transition_to(AgentState::Paused).unwrap();
        let agent_id = agent.id.to_string();
        test_app.state.db.insert_agent(&agent).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/agents/{}/resume", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: AgentResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.state, AgentState::Running);
    }

    #[tokio::test]
    async fn test_terminate_agent() {
        let test_app = setup_app().await;

        // Create running agent
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        make_running(&mut agent);
        let agent_id = agent.id.to_string();
        test_app.state.db.insert_agent(&agent).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/agents/{}/terminate", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: AgentResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.state, AgentState::Terminated);
    }

    #[tokio::test]
    async fn test_terminate_already_terminated_agent_succeeds() {
        let test_app = setup_app().await;

        // Create terminated agent
        // Note: The state machine allows (_, Terminated) -> true, meaning
        // any state including Terminated can transition to Terminated
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        agent.transition_to(AgentState::Terminated).unwrap();
        let agent_id = agent.id.to_string();
        test_app.state.db.insert_agent(&agent).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/agents/{}/terminate", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Terminate is idempotent - terminating an already terminated agent succeeds
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_pause_terminated_agent_fails() {
        let test_app = setup_app().await;

        // Create terminated agent
        let mut agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        agent.transition_to(AgentState::Terminated).unwrap();
        let agent_id = agent.id.to_string();
        test_app.state.db.insert_agent(&agent).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/agents/{}/pause", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        // Cannot pause a terminated agent
        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    // ==================== Messages Tests ====================

    #[tokio::test]
    async fn test_get_messages_empty() {
        let test_app = setup_app().await;

        let agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        let agent_id = agent.id.to_string();
        test_app.state.db.insert_agent(&agent).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/agents/{}/messages", agent_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let messages: Vec<MessageResponse> = serde_json::from_str(&body).unwrap();
        assert!(messages.is_empty());
    }

    #[tokio::test]
    async fn test_get_messages_for_nonexistent_agent() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/agents/00000000-0000-0000-0000-000000000000/messages")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ==================== System Status Tests ====================

    #[tokio::test]
    async fn test_system_status_empty() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let status: SystemStatus = serde_json::from_str(&body).unwrap();
        assert_eq!(status.total_agents, 0);
        assert_eq!(status.running_agents, 0);
        assert_eq!(status.paused_agents, 0);
        assert_eq!(status.completed_agents, 0);
    }

    #[tokio::test]
    async fn test_system_status_with_agents() {
        let test_app = setup_app().await;

        // Create agents in different states
        let mut running_agent = Agent::new(AgentType::StoryDeveloper, "Running task");
        make_running(&mut running_agent);
        test_app
            .state
            .db
            .insert_agent(&running_agent)
            .await
            .unwrap();

        let mut paused_agent = Agent::new(AgentType::StoryDeveloper, "Paused task");
        make_running(&mut paused_agent);
        paused_agent.transition_to(AgentState::Paused).unwrap();
        test_app.state.db.insert_agent(&paused_agent).await.unwrap();

        let mut completed_agent = Agent::new(AgentType::StoryDeveloper, "Completed task");
        make_running(&mut completed_agent);
        completed_agent
            .transition_to(AgentState::Completed)
            .unwrap();
        test_app
            .state
            .db
            .insert_agent(&completed_agent)
            .await
            .unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let status: SystemStatus = serde_json::from_str(&body).unwrap();
        assert_eq!(status.total_agents, 3);
        assert_eq!(status.running_agents, 1);
        assert_eq!(status.paused_agents, 1);
        assert_eq!(status.completed_agents, 1);
    }

    // ==================== ApiError Tests ====================

    #[test]
    fn test_api_error_status_codes() {
        assert_eq!(
            ApiError::unauthorized().into_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            ApiError::not_found("Agent").into_response().status(),
            StatusCode::NOT_FOUND
        );
        assert_eq!(
            ApiError::bad_request("Bad").into_response().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::validation("Invalid").into_response().status(),
            StatusCode::BAD_REQUEST
        );
        assert_eq!(
            ApiError::conflict("Conflict").into_response().status(),
            StatusCode::CONFLICT
        );
        assert_eq!(
            ApiError::internal("Error").into_response().status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    // ==================== CreateAgentRequest Validation Tests ====================

    #[test]
    fn test_create_agent_request_validation() {
        // Valid request
        let valid = CreateAgentRequest {
            agent_type: AgentType::StoryDeveloper,
            task: "Valid task".to_string(),
            worktree_id: None,
        };
        assert!(valid.validate().is_ok());

        // Empty task
        let empty_task = CreateAgentRequest {
            agent_type: AgentType::StoryDeveloper,
            task: "".to_string(),
            worktree_id: None,
        };
        assert!(empty_task.validate().is_err());

        // Whitespace only task
        let whitespace_task = CreateAgentRequest {
            agent_type: AgentType::StoryDeveloper,
            task: "   \t\n".to_string(),
            worktree_id: None,
        };
        assert!(whitespace_task.validate().is_err());

        // Task at max length (should pass)
        let max_task = CreateAgentRequest {
            agent_type: AgentType::StoryDeveloper,
            task: "x".repeat(MAX_TASK_LENGTH),
            worktree_id: None,
        };
        assert!(max_task.validate().is_ok());

        // Task over max length
        let over_max_task = CreateAgentRequest {
            agent_type: AgentType::StoryDeveloper,
            task: "x".repeat(MAX_TASK_LENGTH + 1),
            worktree_id: None,
        };
        assert!(over_max_task.validate().is_err());
    }

    // ==================== Response Conversion Tests ====================

    #[test]
    fn test_agent_response_from_agent() {
        let agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        let response: AgentResponse = agent.clone().into();

        assert_eq!(response.id, agent.id.to_string());
        assert_eq!(response.agent_type, agent.agent_type);
        assert_eq!(response.state, agent.state);
        assert_eq!(response.task, agent.task);
    }

    #[test]
    fn test_message_response_from_message() {
        use orchestrate_core::Message;

        let agent_id = Uuid::new_v4();
        let msg = Message::user(agent_id, "Hello");
        let response: MessageResponse = msg.into();

        assert_eq!(response.role, "user");
        assert_eq!(response.content, "Hello");
    }

    // ==================== Pipeline CRUD Tests ====================

    #[tokio::test]
    async fn test_list_pipelines_empty() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/pipelines")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let pipelines: Vec<PipelineResponse> = serde_json::from_str(&body).unwrap();
        assert!(pipelines.is_empty());
    }

    #[tokio::test]
    async fn test_create_pipeline_success() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/pipelines")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"test-pipeline","definition":"name: test\nstages: []"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let pipeline: PipelineResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(pipeline.name, "test-pipeline");
        assert!(pipeline.enabled);
        assert!(pipeline.id > 0);
    }

    #[tokio::test]
    async fn test_create_pipeline_empty_name_fails() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/pipelines")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"","definition":"test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_pipeline_empty_definition_fails() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/pipelines")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name":"test","definition":""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_create_pipeline_long_name_fails() {
        let test_app = setup_app().await;

        let long_name = "x".repeat(256);
        let body = format!(r#"{{"name":"{}","definition":"test"}}"#, long_name);

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/pipelines")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_pipeline_success() {
        let test_app = setup_app().await;

        // Create pipeline directly in DB
        let pipeline = Pipeline::new("test-pipeline".to_string(), "name: test".to_string());
        let id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();
        assert!(id > 0);

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/pipelines/test-pipeline")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: PipelineResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.name, "test-pipeline");
        assert_eq!(resp.definition, "name: test");
    }

    #[tokio::test]
    async fn test_get_pipeline_not_found() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/pipelines/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_pipeline_success() {
        let test_app = setup_app().await;

        // Create pipeline
        let pipeline = Pipeline::new("test-pipeline".to_string(), "old definition".to_string());
        test_app.state.db.insert_pipeline(&pipeline).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri("/api/pipelines/test-pipeline")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"definition":"new definition","enabled":false}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: PipelineResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.definition, "new definition");
        assert!(!resp.enabled);
    }

    #[tokio::test]
    async fn test_delete_pipeline_success() {
        let test_app = setup_app().await;

        // Create pipeline
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        test_app.state.db.insert_pipeline(&pipeline).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/api/pipelines/test-pipeline")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify deleted
        let deleted = test_app
            .state
            .db
            .get_pipeline_by_name("test-pipeline")
            .await
            .unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_delete_pipeline_not_found() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/api/pipelines/nonexistent")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ==================== Pipeline Run Tests ====================

    #[tokio::test]
    async fn test_trigger_pipeline_run_success() {
        let test_app = setup_app().await;

        // Create pipeline
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        test_app.state.db.insert_pipeline(&pipeline).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/pipelines/test-pipeline/run")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"trigger_event":"manual"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let run: PipelineRunResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(run.status, "pending");
        assert_eq!(run.trigger_event, Some("manual".to_string()));
        assert!(run.id > 0);
    }

    #[tokio::test]
    async fn test_trigger_pipeline_run_nonexistent_pipeline() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/pipelines/nonexistent/run")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_list_pipeline_runs_success() {
        let test_app = setup_app().await;

        // Create pipeline and runs
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();

        let run1 = PipelineRun::new(pipeline_id, Some("event1".to_string()));
        test_app.state.db.insert_pipeline_run(&run1).await.unwrap();

        let run2 = PipelineRun::new(pipeline_id, Some("event2".to_string()));
        test_app.state.db.insert_pipeline_run(&run2).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/pipelines/test-pipeline/runs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let runs: Vec<PipelineRunResponse> = serde_json::from_str(&body).unwrap();
        assert_eq!(runs.len(), 2);
    }

    #[tokio::test]
    async fn test_get_pipeline_run_success() {
        let test_app = setup_app().await;

        // Create pipeline and run
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, Some("test-event".to_string()));
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/pipeline-runs/{}", run_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: PipelineRunResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.id, run_id);
        assert_eq!(resp.trigger_event, Some("test-event".to_string()));
    }

    #[tokio::test]
    async fn test_get_pipeline_run_not_found() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/pipeline-runs/99999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_cancel_pipeline_run_pending() {
        let test_app = setup_app().await;

        // Create pipeline and pending run
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/pipeline-runs/{}/cancel", run_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: PipelineRunResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "cancelled");
    }

    #[tokio::test]
    async fn test_cancel_pipeline_run_running() {
        let test_app = setup_app().await;

        // Create pipeline and running run
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();

        let mut run = PipelineRun::new(pipeline_id, None);
        run.mark_running();
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/pipeline-runs/{}/cancel", run_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: PipelineRunResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "cancelled");
    }

    #[tokio::test]
    async fn test_cancel_pipeline_run_already_completed() {
        let test_app = setup_app().await;

        // Create pipeline and completed run
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();

        let mut run = PipelineRun::new(pipeline_id, None);
        run.mark_succeeded();
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/pipeline-runs/{}/cancel", run_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_list_pipeline_stages() {
        let test_app = setup_app().await;

        // Create pipeline and run
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();

        let run = PipelineRun::new(pipeline_id, None);
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();

        // Create stages
        let stage1 = PipelineStage::new(run_id, "build".to_string());
        let stage2 = PipelineStage::new(run_id, "test".to_string());
        test_app.state.db.insert_pipeline_stage(&stage1).await.unwrap();
        test_app.state.db.insert_pipeline_stage(&stage2).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/pipeline-runs/{}/stages", run_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let stages: Vec<PipelineStageResponse> = serde_json::from_str(&body).unwrap();
        assert_eq!(stages.len(), 2);
        assert_eq!(stages[0].stage_name, "build");
        assert_eq!(stages[1].stage_name, "test");
    }

    // ==================== Approval Tests ====================

    #[tokio::test]
    async fn test_list_pending_approvals_empty() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/approvals")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let approvals: Vec<ApprovalResponse> = serde_json::from_str(&body).unwrap();
        assert!(approvals.is_empty());
    }

    #[tokio::test]
    async fn test_list_pending_approvals_with_data() {
        let test_app = setup_app().await;

        // Create pipeline, run, and stage first
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();
        let run = PipelineRun::new(pipeline_id, None);
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();
        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = test_app.state.db.insert_pipeline_stage(&stage).await.unwrap();

        // Create approval request
        let approval = ApprovalRequest::new(
            stage_id,
            run_id,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );
        test_app
            .state
            .db
            .create_approval_request(approval)
            .await
            .unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/approvals")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let approvals: Vec<ApprovalResponse> = serde_json::from_str(&body).unwrap();
        assert_eq!(approvals.len(), 1);
        assert_eq!(approvals[0].status, "pending");
    }

    #[tokio::test]
    async fn test_approve_approval_success() {
        let test_app = setup_app().await;

        // Create pipeline, run, and stage first
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();
        let run = PipelineRun::new(pipeline_id, None);
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();
        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = test_app.state.db.insert_pipeline_stage(&stage).await.unwrap();

        // Create approval request
        let approval = ApprovalRequest::new(
            stage_id,
            run_id,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );
        let created = test_app
            .state
            .db
            .create_approval_request(approval)
            .await
            .unwrap();
        let approval_id = created.id.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/approvals/{}/approve", approval_id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"approver":"user1@example.com","comment":"LGTM"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: ApprovalResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "approved");
        assert_eq!(resp.approval_count, 1);
    }

    #[tokio::test]
    async fn test_approve_approval_empty_approver_fails() {
        let test_app = setup_app().await;

        // Create pipeline, run, and stage first
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();
        let run = PipelineRun::new(pipeline_id, None);
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();
        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = test_app.state.db.insert_pipeline_stage(&stage).await.unwrap();

        // Create approval request
        let approval = ApprovalRequest::new(
            stage_id,
            run_id,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );
        let created = test_app
            .state
            .db
            .create_approval_request(approval)
            .await
            .unwrap();
        let approval_id = created.id.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/approvals/{}/approve", approval_id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"approver":""}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_approve_approval_unauthorized_approver() {
        let test_app = setup_app().await;

        // Create pipeline, run, and stage first
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();
        let run = PipelineRun::new(pipeline_id, None);
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();
        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = test_app.state.db.insert_pipeline_stage(&stage).await.unwrap();

        // Create approval request requiring user1
        let approval = ApprovalRequest::new(
            stage_id,
            run_id,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );
        let created = test_app
            .state
            .db
            .create_approval_request(approval)
            .await
            .unwrap();
        let approval_id = created.id.unwrap();

        // Try to approve as user2
        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/approvals/{}/approve", approval_id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"approver":"user2@example.com"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_reject_approval_success() {
        let test_app = setup_app().await;

        // Create pipeline, run, and stage first
        let pipeline = Pipeline::new("test-pipeline".to_string(), "definition".to_string());
        let pipeline_id = test_app.state.db.insert_pipeline(&pipeline).await.unwrap();
        let run = PipelineRun::new(pipeline_id, None);
        let run_id = test_app.state.db.insert_pipeline_run(&run).await.unwrap();
        let stage = PipelineStage::new(run_id, "deploy".to_string());
        let stage_id = test_app.state.db.insert_pipeline_stage(&stage).await.unwrap();

        // Create approval request
        let approval = ApprovalRequest::new(
            stage_id,
            run_id,
            "user1@example.com".to_string(),
            1,
            None,
            None,
        );
        let created = test_app
            .state
            .db
            .create_approval_request(approval)
            .await
            .unwrap();
        let approval_id = created.id.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/approvals/{}/reject", approval_id))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"approver":"user1@example.com","comment":"Not ready"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: ApprovalResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.status, "rejected");
        assert_eq!(resp.rejection_count, 1);
    }

    #[tokio::test]
    async fn test_approve_approval_not_found() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/approvals/99999/approve")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"approver":"user@example.com"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_reject_approval_not_found() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/approvals/99999/reject")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"approver":"user@example.com"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // ==================== Request Validation Tests ====================

    #[test]
    fn test_create_pipeline_request_validation() {
        // Valid request
        let valid = CreatePipelineRequest {
            name: "test".to_string(),
            definition: "definition".to_string(),
            enabled: Some(true),
        };
        assert!(valid.validate().is_ok());

        // Empty name
        let empty_name = CreatePipelineRequest {
            name: "".to_string(),
            definition: "definition".to_string(),
            enabled: None,
        };
        assert!(empty_name.validate().is_err());

        // Empty definition
        let empty_def = CreatePipelineRequest {
            name: "test".to_string(),
            definition: "".to_string(),
            enabled: None,
        };
        assert!(empty_def.validate().is_err());

        // Name too long
        let long_name = CreatePipelineRequest {
            name: "x".repeat(256),
            definition: "definition".to_string(),
            enabled: None,
        };
        assert!(long_name.validate().is_err());
    }

    #[test]
    fn test_approval_decision_request_validation() {
        // Valid request
        let valid = ApprovalDecisionRequest {
            approver: "user@example.com".to_string(),
            comment: Some("LGTM".to_string()),
        };
        assert!(valid.validate().is_ok());

        // Empty approver
        let empty = ApprovalDecisionRequest {
            approver: "".to_string(),
            comment: None,
        };
        assert!(empty.validate().is_err());

        // Whitespace approver
        let whitespace = ApprovalDecisionRequest {
            approver: "   ".to_string(),
            comment: None,
        };
        assert!(whitespace.validate().is_err());
    }

    // ==================== Response Conversion Tests ====================

    #[test]
    fn test_pipeline_response_from_pipeline() {
        let mut pipeline = Pipeline::new("test".to_string(), "definition".to_string());
        pipeline.id = Some(42);
        pipeline.enabled = false;

        let response: PipelineResponse = pipeline.clone().into();

        assert_eq!(response.id, 42);
        assert_eq!(response.name, "test");
        assert_eq!(response.definition, "definition");
        assert!(!response.enabled);
    }

    #[test]
    fn test_pipeline_run_response_from_run() {
        let mut run = PipelineRun::new(1, Some("event".to_string()));
        run.id = Some(42);
        run.mark_running();

        let response: PipelineRunResponse = run.clone().into();

        assert_eq!(response.id, 42);
        assert_eq!(response.pipeline_id, 1);
        assert_eq!(response.status, "running");
        assert_eq!(response.trigger_event, Some("event".to_string()));
        assert!(response.started_at.is_some());
    }

    #[test]
    fn test_approval_response_from_approval() {
        let mut approval = ApprovalRequest::new(
            1,
            2,
            "user@example.com".to_string(),
            1,
            Some(3600),
            Some("approve".to_string()),
        );
        approval.id = Some(42);

        let response: ApprovalResponse = approval.clone().into();

        assert_eq!(response.id, 42);
        assert_eq!(response.stage_id, 1);
        assert_eq!(response.run_id, 2);
        assert_eq!(response.status, "pending");
        assert_eq!(response.required_approvers, "user@example.com");
        assert_eq!(response.required_count, 1);
        assert_eq!(response.timeout_seconds, Some(3600));
    }
}
