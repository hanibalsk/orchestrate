//! HTML UI routes for the web interface

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post},
    Router,
};
use orchestrate_core::{Agent, AgentState, AgentType, Message};
use serde::Deserialize;
use std::sync::Arc;
use tower_http::services::ServeDir;
use uuid::Uuid;

use crate::api::AppState;

// ==================== View Models ====================

/// Agent view for templates
#[derive(Clone)]
pub struct AgentView {
    pub id: String,
    pub agent_type: String,
    pub state: String,
    pub task: String,
    pub created_at: String,
    pub updated_at: String,
    pub error_message: Option<String>,
}

impl From<Agent> for AgentView {
    fn from(agent: Agent) -> Self {
        Self {
            id: agent.id.to_string(),
            agent_type: format!("{:?}", agent.agent_type),
            state: format!("{:?}", agent.state),
            task: agent.task,
            created_at: agent.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            updated_at: agent.updated_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            error_message: agent.error_message,
        }
    }
}

/// Message view for templates
#[derive(Clone)]
pub struct MessageView {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub tool_calls: Vec<ToolCallView>,
    pub tool_results: Vec<ToolResultView>,
}

#[derive(Clone)]
pub struct ToolCallView {
    pub id: String,
    pub name: String,
    pub input: String,
}

#[derive(Clone)]
pub struct ToolResultView {
    pub tool_call_id: String,
    pub content: String,
    pub is_error: bool,
}

impl From<Message> for MessageView {
    fn from(msg: Message) -> Self {
        Self {
            id: msg.id,
            role: msg.role.as_str().to_string(),
            content: msg.content,
            created_at: msg.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
            tool_calls: msg
                .tool_calls
                .unwrap_or_default()
                .into_iter()
                .map(|tc| ToolCallView {
                    id: tc.id,
                    name: tc.name,
                    input: serde_json::to_string_pretty(&tc.input).unwrap_or_default(),
                })
                .collect(),
            tool_results: msg
                .tool_results
                .unwrap_or_default()
                .into_iter()
                .map(|tr| ToolResultView {
                    tool_call_id: tr.tool_call_id,
                    content: tr.content,
                    is_error: tr.is_error,
                })
                .collect(),
        }
    }
}

/// System status for templates
pub struct SystemStatusView {
    pub total_agents: usize,
    pub running_agents: usize,
    pub paused_agents: usize,
    pub completed_agents: usize,
}

// ==================== Templates ====================

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    status: SystemStatusView,
    agents: Vec<AgentView>,
}

#[derive(Template)]
#[template(path = "agents/list.html")]
struct AgentListTemplate {
    agents: Vec<AgentView>,
}

#[derive(Template)]
#[template(path = "agents/detail.html")]
struct AgentDetailTemplate {
    agent: AgentView,
    messages: Vec<MessageView>,
}

// ==================== Error Handling ====================

struct UiError(String);

impl IntoResponse for UiError {
    fn into_response(self) -> Response {
        Html(format!(
            r#"<!DOCTYPE html>
<html>
<head><title>Error</title></head>
<body>
<h1>Error</h1>
<p>{}</p>
<a href="/">Back to Dashboard</a>
</body>
</html>"#,
            self.0
        ))
        .into_response()
    }
}

impl From<askama::Error> for UiError {
    fn from(err: askama::Error) -> Self {
        UiError(format!("Template error: {}", err))
    }
}

// ==================== Form Types ====================

#[derive(Deserialize)]
pub struct CreateAgentForm {
    pub agent_type: String,
    pub task: String,
}

#[derive(Deserialize)]
pub struct SendMessageForm {
    pub content: String,
}

// ==================== Route Handlers ====================

/// Dashboard page
async fn dashboard(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, UiError> {
    let agents = state
        .db
        .list_agents()
        .await
        .map_err(|e| UiError(format!("Database error: {}", e)))?;

    let running = agents.iter().filter(|a| a.state == AgentState::Running).count();
    let paused = agents.iter().filter(|a| a.state == AgentState::Paused).count();
    let completed = agents.iter().filter(|a| a.state == AgentState::Completed).count();

    let status = SystemStatusView {
        total_agents: agents.len(),
        running_agents: running,
        paused_agents: paused,
        completed_agents: completed,
    };

    // Show only recent agents (last 10)
    let recent_agents: Vec<AgentView> = agents
        .into_iter()
        .take(10)
        .map(Into::into)
        .collect();

    let template = DashboardTemplate {
        status,
        agents: recent_agents,
    };

    Ok(Html(template.render()?))
}

/// Agent list page
async fn agent_list(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, UiError> {
    let agents = state
        .db
        .list_agents()
        .await
        .map_err(|e| UiError(format!("Database error: {}", e)))?;

    let template = AgentListTemplate {
        agents: agents.into_iter().map(Into::into).collect(),
    };

    Ok(Html(template.render()?))
}

/// Agent detail page with chat
async fn agent_detail(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, UiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| UiError("Invalid agent ID".to_string()))?;

    let agent = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| UiError(format!("Database error: {}", e)))?
        .ok_or_else(|| UiError("Agent not found".to_string()))?;

    let messages = state
        .db
        .get_messages(uuid)
        .await
        .map_err(|e| UiError(format!("Database error: {}", e)))?;

    let template = AgentDetailTemplate {
        agent: agent.into(),
        messages: messages.into_iter().map(Into::into).collect(),
    };

    Ok(Html(template.render()?))
}

/// Create a new agent
async fn create_agent(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CreateAgentForm>,
) -> Result<impl IntoResponse, UiError> {
    let agent_type = match form.agent_type.as_str() {
        "StoryDeveloper" => AgentType::StoryDeveloper,
        "CodeReviewer" => AgentType::CodeReviewer,
        "IssueFixer" => AgentType::IssueFixer,
        "Explorer" => AgentType::Explorer,
        "PrShepherd" => AgentType::PrShepherd,
        _ => return Err(UiError(format!("Unknown agent type: {}", form.agent_type))),
    };

    if form.task.trim().is_empty() {
        return Err(UiError("Task cannot be empty".to_string()));
    }

    let agent = Agent::new(agent_type, form.task);
    let agent_id = agent.id;

    state
        .db
        .insert_agent(&agent)
        .await
        .map_err(|e| UiError(format!("Failed to create agent: {}", e)))?;

    Ok(Redirect::to(&format!("/agents/{}", agent_id)))
}

/// Send a message to an agent
async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Form(form): Form<SendMessageForm>,
) -> Result<impl IntoResponse, UiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| UiError("Invalid agent ID".to_string()))?;

    // Verify agent exists
    let _agent = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| UiError(format!("Database error: {}", e)))?
        .ok_or_else(|| UiError("Agent not found".to_string()))?;

    if form.content.trim().is_empty() {
        return Err(UiError("Message cannot be empty".to_string()));
    }

    // Insert user message
    let msg = Message::user(uuid, form.content);
    state
        .db
        .insert_message(&msg)
        .await
        .map_err(|e| UiError(format!("Failed to send message: {}", e)))?;

    Ok(Redirect::to(&format!("/agents/{}", id)))
}

/// Pause an agent
async fn pause_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, UiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| UiError("Invalid agent ID".to_string()))?;

    let mut agent = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| UiError(format!("Database error: {}", e)))?
        .ok_or_else(|| UiError("Agent not found".to_string()))?;

    agent
        .transition_to(AgentState::Paused)
        .map_err(|_| UiError(format!("Cannot pause agent in state {:?}", agent.state)))?;

    state
        .db
        .update_agent(&agent)
        .await
        .map_err(|e| UiError(format!("Failed to update agent: {}", e)))?;

    Ok(Redirect::to(&format!("/agents/{}", id)))
}

/// Resume an agent
async fn resume_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, UiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| UiError("Invalid agent ID".to_string()))?;

    let mut agent = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| UiError(format!("Database error: {}", e)))?
        .ok_or_else(|| UiError("Agent not found".to_string()))?;

    agent
        .transition_to(AgentState::Running)
        .map_err(|_| UiError(format!("Cannot resume agent in state {:?}", agent.state)))?;

    state
        .db
        .update_agent(&agent)
        .await
        .map_err(|e| UiError(format!("Failed to update agent: {}", e)))?;

    Ok(Redirect::to(&format!("/agents/{}", id)))
}

/// Terminate an agent
async fn terminate_agent(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<impl IntoResponse, UiError> {
    let uuid = Uuid::parse_str(&id).map_err(|_| UiError("Invalid agent ID".to_string()))?;

    let mut agent = state
        .db
        .get_agent(uuid)
        .await
        .map_err(|e| UiError(format!("Database error: {}", e)))?
        .ok_or_else(|| UiError("Agent not found".to_string()))?;

    agent
        .transition_to(AgentState::Terminated)
        .map_err(|_| UiError(format!("Cannot terminate agent in state {:?}", agent.state)))?;

    state
        .db
        .update_agent(&agent)
        .await
        .map_err(|e| UiError(format!("Failed to update agent: {}", e)))?;

    Ok(Redirect::to(&format!("/agents/{}", id)))
}

// ==================== Router ====================

/// Create the UI router with static file serving
pub fn create_ui_router() -> Router<Arc<AppState>> {
    // Get the path to static files relative to the crate
    let static_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("static");

    Router::new()
        // Pages
        .route("/", get(dashboard))
        .route("/agents", get(agent_list))
        .route("/agents/:id", get(agent_detail))
        // Actions
        .route("/agents/create", post(create_agent))
        .route("/agents/:id/send", post(send_message))
        .route("/agents/:id/pause", post(pause_agent))
        .route("/agents/:id/resume", post(resume_agent))
        .route("/agents/:id/terminate", post(terminate_agent))
        // Static files
        .nest_service("/static", ServeDir::new(static_path))
}
