//! WebSocket handling for real-time updates

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use orchestrate_core::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Agent state changed
    AgentState { agent_id: String, state: String },
    /// New message from agent
    AgentMessage {
        agent_id: String,
        role: String,
        content: String,
    },
    /// PR status update
    PrUpdate { pr_number: i32, status: String },
    /// System status
    SystemStatus {
        total_agents: usize,
        running_agents: usize,
    },
    /// Client subscription request
    Subscribe { channels: Vec<String> },
    /// Client message to agent
    SendMessage { agent_id: String, content: String },
    /// Error response
    Error { message: String },
    /// Success response
    Success { message: String },
}

/// WebSocket state for managing connections
pub struct WsState {
    pub broadcast_tx: broadcast::Sender<WsMessage>,
    pub db: Database,
}

impl WsState {
    pub fn new(db: Database) -> Self {
        let (broadcast_tx, _) = broadcast::channel(100);
        Self { broadcast_tx, db }
    }

    /// Get a broadcast sender for publishing messages
    pub fn sender(&self) -> broadcast::Sender<WsMessage> {
        self.broadcast_tx.clone()
    }
}

/// WebSocket handler with state
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<WsState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<WsState>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.broadcast_tx.subscribe();
    let db = state.db.clone();

    // Track which agents this client is subscribed to
    let subscribed_agents: Arc<tokio::sync::RwLock<HashSet<String>>> =
        Arc::new(tokio::sync::RwLock::new(HashSet::new()));
    let subscribed_agents_clone = subscribed_agents.clone();

    // Spawn task to forward broadcast messages to client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            // Check if client is subscribed to this agent
            let should_send = match &msg {
                WsMessage::AgentState { agent_id, .. }
                | WsMessage::AgentMessage { agent_id, .. } => {
                    let subscribed = subscribed_agents_clone.read().await;
                    subscribed.is_empty() || subscribed.contains(agent_id)
                }
                // Always send system status and other messages
                _ => true,
            };

            if should_send {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if sender.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
            }
        }
    });

    // Handle incoming messages from client
    while let Some(msg) = receiver.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text.to_string()) {
                match ws_msg {
                    WsMessage::Subscribe { channels } => {
                        // Update subscriptions
                        let mut subscribed = subscribed_agents.write().await;
                        subscribed.clear();
                        for channel in channels {
                            // Channel format: "agent:<uuid>" or just "<uuid>"
                            let agent_id = channel.strip_prefix("agent:").unwrap_or(&channel);
                            subscribed.insert(agent_id.to_string());
                        }
                    }
                    WsMessage::SendMessage { agent_id, content } => {
                        // Route message to agent via database
                        match handle_send_message(&db, &agent_id, &content).await {
                            Ok(_) => {
                                // Broadcast the new message to all subscribers
                                let _ = state.broadcast_tx.send(WsMessage::AgentMessage {
                                    agent_id,
                                    role: "user".to_string(),
                                    content,
                                });
                            }
                            Err(e) => {
                                // Log error but don't disconnect
                                tracing::warn!("Failed to send message to agent: {}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    send_task.abort();
}

/// Handle sending a message to an agent
async fn handle_send_message(db: &Database, agent_id: &str, content: &str) -> anyhow::Result<()> {
    let uuid = Uuid::parse_str(agent_id)?;

    // Verify agent exists and is in a state that accepts input
    let agent = db
        .get_agent(uuid)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

    // Check if agent can receive input
    match agent.state {
        orchestrate_core::AgentState::WaitingForInput => {
            // Perfect - agent is waiting for input
        }
        orchestrate_core::AgentState::Running | orchestrate_core::AgentState::Paused => {
            // Allow sending but agent might not process immediately
        }
        _ => {
            return Err(anyhow::anyhow!(
                "Agent is in state {:?} and cannot receive messages",
                agent.state
            ));
        }
    }

    // Insert the user message
    let message = orchestrate_core::Message::user(uuid, content);
    db.insert_message(&message).await?;

    Ok(())
}
