//! WebSocket handling for real-time updates

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Agent state changed
    AgentState {
        agent_id: String,
        state: String,
    },
    /// New message from agent
    AgentMessage {
        agent_id: String,
        role: String,
        content: String,
    },
    /// PR status update
    PrUpdate {
        pr_number: i32,
        status: String,
    },
    /// System status
    SystemStatus {
        total_agents: usize,
        running_agents: usize,
    },
    /// Client subscription request
    Subscribe {
        channels: Vec<String>,
    },
    /// Client message to agent
    SendMessage {
        agent_id: String,
        content: String,
    },
}

/// WebSocket handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    tx: broadcast::Sender<WsMessage>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, tx))
}

async fn handle_socket(socket: WebSocket, tx: broadcast::Sender<WsMessage>) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = tx.subscribe();

    // Spawn task to forward broadcast messages to client
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = rx.recv().await {
            if let Ok(json) = serde_json::to_string(&msg) {
                if sender.send(Message::Text(json)).await.is_err() {
                    break;
                }
            }
        }
    });

    // Handle incoming messages from client
    while let Some(msg) = receiver.next().await {
        if let Ok(Message::Text(text)) = msg {
            if let Ok(ws_msg) = serde_json::from_str::<WsMessage>(&text) {
                match ws_msg {
                    WsMessage::Subscribe { channels: _ } => {
                        // Handle subscription (for now, broadcast everything)
                    }
                    WsMessage::SendMessage { agent_id: _, content: _ } => {
                        // Handle message to agent
                        // TODO: Route to agent
                    }
                    _ => {}
                }
            }
        }
    }

    send_task.abort();
}
