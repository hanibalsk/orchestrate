//! REST API endpoints with authentication and validation

use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use orchestrate_core::{
    Agent, AgentState, AgentType, CustomInstruction, Database, InstructionEffectiveness,
    InstructionScope, InstructionSource, LearningEngine, LearningPattern, PatternStatus,
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
    let api_router = create_api_router(state.clone());
    let ui_router = crate::ui::create_ui_router().with_state(state.clone());

    // Create WebSocket state with the same database
    let ws_state = Arc::new(crate::websocket::WsState::new(state.db.clone()));

    Router::new()
        .merge(api_router)
        .merge(ui_router)
        .route(
            "/ws",
            axum::routing::get(crate::websocket::ws_handler).with_state(ws_state),
        )
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

    // Validate cron expression
    schedule
        .validate_cron()
        .map_err(|e| ApiError::validation(format!("Invalid cron expression: {}", e)))?;

    // Calculate next run
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
        // Validate and recalculate next run
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
) -> Result<Json<serde_json::Value>, ApiError> {
    let deleted = state
        .db
        .delete_schedule(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    if !deleted {
        return Err(ApiError::not_found("Schedule"));
    }

    Ok(Json(serde_json::json!({"deleted": true})))
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
) -> Result<Json<AgentResponse>, ApiError> {
    let schedule = state
        .db
        .get_schedule(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Schedule"))?;

    // Parse agent type
    let agent_type = AgentType::from_str(&schedule.agent_type)
        .map_err(|_| ApiError::bad_request(format!("Invalid agent type: {}", schedule.agent_type)))?;

    // Create and insert agent
    let agent = Agent::new(agent_type, schedule.task.clone());
    state
        .db
        .insert_agent(&agent)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(agent.into()))
}

async fn get_schedule_runs(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<ScheduleRunResponse>>, ApiError> {
    // Verify schedule exists
    let _ = state
        .db
        .get_schedule(id)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::not_found("Schedule"))?;

    let runs = state
        .db
        .get_schedule_runs(id, 100)
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    Ok(Json(runs.into_iter().map(Into::into).collect()))
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

// ==================== Schedule Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct CreateScheduleRequest {
    pub name: String,
    pub cron_expression: String,
    pub agent_type: String,
    pub task: String,
    pub enabled: Option<bool>,
}

impl CreateScheduleRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if self.name.trim().is_empty() {
            return Err(ApiError::validation("Name cannot be empty"));
        }
        if self.cron_expression.trim().is_empty() {
            return Err(ApiError::validation("Cron expression cannot be empty"));
        }
        if self.agent_type.trim().is_empty() {
            return Err(ApiError::validation("Agent type cannot be empty"));
        }
        if self.task.trim().is_empty() {
            return Err(ApiError::validation("Task cannot be empty"));
        }
        if self.name.len() > 255 {
            return Err(ApiError::validation("Name exceeds maximum length of 255 characters"));
        }
        if self.task.len() > MAX_TASK_LENGTH {
            return Err(ApiError::validation(format!(
                "Task exceeds maximum length of {} characters",
                MAX_TASK_LENGTH
            )));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateScheduleRequest {
    pub name: Option<String>,
    pub cron_expression: Option<String>,
    pub agent_type: Option<String>,
    pub task: Option<String>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleResponse {
    pub id: i64,
    pub name: String,
    pub cron_expression: String,
    pub agent_type: String,
    pub task: String,
    pub enabled: bool,
    pub last_run: Option<String>,
    pub next_run: Option<String>,
    pub created_at: String,
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
            last_run: schedule.last_run.map(|dt| dt.to_rfc3339()),
            next_run: schedule.next_run.map(|dt| dt.to_rfc3339()),
            created_at: schedule.created_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScheduleRunResponse {
    pub id: i64,
    pub schedule_id: i64,
    pub agent_id: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
}

impl From<ScheduleRun> for ScheduleRunResponse {
    fn from(run: ScheduleRun) -> Self {
        Self {
            id: run.id,
            schedule_id: run.schedule_id,
            agent_id: run.agent_id,
            started_at: run.started_at.to_rfc3339(),
            completed_at: run.completed_at.map(|dt| dt.to_rfc3339()),
            status: run.status.as_str().to_string(),
            error_message: run.error_message,
        }
    }
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

    // ==================== Schedule Tests ====================

    #[tokio::test]
    async fn test_list_schedules_empty() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/schedules")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let schedules: Vec<ScheduleResponse> = serde_json::from_str(&body).unwrap();
        assert!(schedules.is_empty());
    }

    #[tokio::test]
    async fn test_create_schedule_success() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/schedules")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"daily-backup","cron_expression":"0 2 * * *","agent_type":"story_developer","task":"Run backup"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let schedule: ScheduleResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(schedule.name, "daily-backup");
        assert_eq!(schedule.cron_expression, "0 2 * * *");
        assert_eq!(schedule.agent_type, "story_developer");
        assert_eq!(schedule.task, "Run backup");
        assert!(schedule.enabled);
        assert!(schedule.next_run.is_some());
    }

    #[tokio::test]
    async fn test_create_schedule_invalid_cron() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/schedules")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"bad-schedule","cron_expression":"invalid","agent_type":"story_developer","task":"Test"}"#,
                    ))
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
    async fn test_create_schedule_empty_name() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/schedules")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"name":"","cron_expression":"0 2 * * *","agent_type":"story_developer","task":"Test"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_get_schedule_success() {
        let test_app = setup_app().await;

        // Create schedule directly in DB
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Test task".to_string(),
        );
        schedule.update_next_run().unwrap();
        let id = test_app.state.db.insert_schedule(&schedule).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/schedules/{}", id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: ScheduleResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.id, id);
        assert_eq!(resp.name, "test-schedule");
    }

    #[tokio::test]
    async fn test_get_schedule_not_found() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/schedules/999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_update_schedule_success() {
        let test_app = setup_app().await;

        // Create schedule
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Original task".to_string(),
        );
        schedule.update_next_run().unwrap();
        let id = test_app.state.db.insert_schedule(&schedule).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/api/schedules/{}", id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"task":"Updated task"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: ScheduleResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.task, "Updated task");
        assert_eq!(resp.name, "test-schedule"); // Unchanged
    }

    #[tokio::test]
    async fn test_update_schedule_cron_expression() {
        let test_app = setup_app().await;

        // Create schedule
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Test task".to_string(),
        );
        schedule.update_next_run().unwrap();
        let id = test_app.state.db.insert_schedule(&schedule).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/api/schedules/{}", id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"cron_expression":"@hourly"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: ScheduleResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(resp.cron_expression, "@hourly");
    }

    #[tokio::test]
    async fn test_update_schedule_invalid_cron() {
        let test_app = setup_app().await;

        // Create schedule
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Test task".to_string(),
        );
        schedule.update_next_run().unwrap();
        let id = test_app.state.db.insert_schedule(&schedule).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::PUT)
                    .uri(format!("/api/schedules/{}", id))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"cron_expression":"invalid"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_delete_schedule_success() {
        let test_app = setup_app().await;

        // Create schedule
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Test task".to_string(),
        );
        schedule.update_next_run().unwrap();
        let id = test_app.state.db.insert_schedule(&schedule).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri(format!("/api/schedules/{}", id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify it's deleted
        let schedule = test_app.state.db.get_schedule(id).await.unwrap();
        assert!(schedule.is_none());
    }

    #[tokio::test]
    async fn test_delete_schedule_not_found() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::DELETE)
                    .uri("/api/schedules/999")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_pause_schedule() {
        let test_app = setup_app().await;

        // Create enabled schedule
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Test task".to_string(),
        );
        schedule.update_next_run().unwrap();
        let id = test_app.state.db.insert_schedule(&schedule).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/schedules/{}/pause", id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: ScheduleResponse = serde_json::from_str(&body).unwrap();
        assert!(!resp.enabled);
    }

    #[tokio::test]
    async fn test_resume_schedule() {
        let test_app = setup_app().await;

        // Create disabled schedule
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Test task".to_string(),
        );
        schedule.enabled = false;
        schedule.update_next_run().unwrap();
        let id = test_app.state.db.insert_schedule(&schedule).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/schedules/{}/resume", id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let resp: ScheduleResponse = serde_json::from_str(&body).unwrap();
        assert!(resp.enabled);
    }

    #[tokio::test]
    async fn test_run_schedule_immediately() {
        let test_app = setup_app().await;

        // Create schedule
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Test task for immediate run".to_string(),
        );
        schedule.update_next_run().unwrap();
        let id = test_app.state.db.insert_schedule(&schedule).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("/api/schedules/{}/run", id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let agent: AgentResponse = serde_json::from_str(&body).unwrap();
        assert_eq!(agent.task, "Test task for immediate run");
        assert_eq!(agent.agent_type, AgentType::StoryDeveloper);
    }

    #[tokio::test]
    async fn test_get_schedule_runs_empty() {
        let test_app = setup_app().await;

        // Create schedule
        let mut schedule = Schedule::new(
            "test-schedule".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Test task".to_string(),
        );
        schedule.update_next_run().unwrap();
        let id = test_app.state.db.insert_schedule(&schedule).await.unwrap();

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("/api/schedules/{}/runs", id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = body_to_string(response.into_body()).await;
        let runs: Vec<ScheduleRunResponse> = serde_json::from_str(&body).unwrap();
        assert!(runs.is_empty());
    }

    #[tokio::test]
    async fn test_get_schedule_runs_not_found() {
        let test_app = setup_app().await;

        let response = test_app
            .router
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/api/schedules/999/runs")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn test_schedule_response_from_schedule() {
        let mut schedule = Schedule::new(
            "test".to_string(),
            "@daily".to_string(),
            "story_developer".to_string(),
            "Test task".to_string(),
        );
        schedule.id = 42;
        let response: ScheduleResponse = schedule.clone().into();

        assert_eq!(response.id, 42);
        assert_eq!(response.name, "test");
        assert_eq!(response.cron_expression, "@daily");
        assert_eq!(response.agent_type, "story_developer");
        assert_eq!(response.task, "Test task");
    }
}
