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
        .route("/api/agents", get(list_agents).post(create_agent))
        .route("/api/agents/:id", get(get_agent))
        .route("/api/agents/:id/pause", post(pause_agent))
        .route("/api/agents/:id/resume", post(resume_agent))
        .route("/api/agents/:id/terminate", post(terminate_agent))
        .route("/api/agents/:id/messages", get(get_messages))
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/api/agents")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"agent_type":"story_developer","task":"   "}"#))
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
        let body = format!(r#"{{"agent_type":"story_developer","task":"{}"}}"#, long_task);

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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
        assert!(response.status() == StatusCode::BAD_REQUEST || response.status() == StatusCode::NOT_FOUND);
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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

        let response = test_app.router
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
        test_app.state.db.insert_agent(&running_agent).await.unwrap();

        let mut paused_agent = Agent::new(AgentType::StoryDeveloper, "Paused task");
        make_running(&mut paused_agent);
        paused_agent.transition_to(AgentState::Paused).unwrap();
        test_app.state.db.insert_agent(&paused_agent).await.unwrap();

        let mut completed_agent = Agent::new(AgentType::StoryDeveloper, "Completed task");
        make_running(&mut completed_agent);
        completed_agent.transition_to(AgentState::Completed).unwrap();
        test_app.state.db.insert_agent(&completed_agent).await.unwrap();

        let response = test_app.router
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
        };
        assert!(valid.validate().is_ok());

        // Empty task
        let empty_task = CreateAgentRequest {
            agent_type: AgentType::StoryDeveloper,
            task: "".to_string(),
        };
        assert!(empty_task.validate().is_err());

        // Whitespace only task
        let whitespace_task = CreateAgentRequest {
            agent_type: AgentType::StoryDeveloper,
            task: "   \t\n".to_string(),
        };
        assert!(whitespace_task.validate().is_err());

        // Task at max length (should pass)
        let max_task = CreateAgentRequest {
            agent_type: AgentType::StoryDeveloper,
            task: "x".repeat(MAX_TASK_LENGTH),
        };
        assert!(max_task.validate().is_ok());

        // Task over max length
        let over_max_task = CreateAgentRequest {
            agent_type: AgentType::StoryDeveloper,
            task: "x".repeat(MAX_TASK_LENGTH + 1),
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
}
