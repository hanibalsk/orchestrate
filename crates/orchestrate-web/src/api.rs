//! REST API endpoints with authentication and validation

use axum::{
    body::Body,
    extract::{Path, State},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use orchestrate_core::{Agent, AgentState, AgentType, Database};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Maximum task length
const MAX_TASK_LENGTH: usize = 10_000;

/// API error response
#[derive(Debug, Serialize)]
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
    let protected_routes = Router::new()
        .route("/api/agents", get(list_agents).post(create_agent))
        .route("/api/agents/{id}", get(get_agent))
        .route("/api/agents/{id}/pause", post(pause_agent))
        .route("/api/agents/{id}/resume", post(resume_agent))
        .route("/api/agents/{id}/terminate", post(terminate_agent))
        .route("/api/agents/{id}/messages", get(get_messages))
        .route("/api/status", get(system_status))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware));

    // Public routes (no auth required)
    let public_routes = Router::new()
        .route("/api/health", get(health_check));

    Router::new()
        .merge(protected_routes)
        .merge(public_routes)
        .with_state(state)
}

/// Create the full router with both API and UI routes
pub fn create_router(state: Arc<AppState>) -> Router {
    let api_router = create_api_router(state.clone());
    let ui_router = crate::ui::create_ui_router().with_state(state);

    Router::new()
        .merge(api_router)
        .merge(ui_router)
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

    let agent = Agent::new(req.agent_type, req.task);

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

    agent
        .transition_to(AgentState::Paused)
        .map_err(|_| ApiError::conflict(format!(
            "Cannot pause agent in state {:?}",
            agent.state
        )))?;

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

    agent
        .transition_to(AgentState::Running)
        .map_err(|_| ApiError::conflict(format!(
            "Cannot resume agent in state {:?}",
            agent.state
        )))?;

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

    agent
        .transition_to(AgentState::Terminated)
        .map_err(|_| ApiError::conflict(format!(
            "Cannot terminate agent in state {:?}",
            agent.state
        )))?;

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

async fn system_status(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SystemStatus>, ApiError> {
    let agents = state
        .db
        .list_agents()
        .await
        .map_err(|e| ApiError::internal(format!("Database error: {}", e)))?;

    let running = agents.iter().filter(|a| a.state == AgentState::Running).count();
    let paused = agents.iter().filter(|a| a.state == AgentState::Paused).count();
    let completed = agents.iter().filter(|a| a.state == AgentState::Completed).count();

    Ok(Json(SystemStatus {
        total_agents: agents.len(),
        running_agents: running,
        paused_agents: paused,
        completed_agents: completed,
    }))
}

// ==================== Request/Response Types ====================

#[derive(Debug, Deserialize)]
pub struct CreateAgentRequest {
    pub agent_type: AgentType,
    pub task: String,
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub struct SystemStatus {
    pub total_agents: usize,
    pub running_agents: usize,
    pub paused_agents: usize,
    pub completed_agents: usize,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}
