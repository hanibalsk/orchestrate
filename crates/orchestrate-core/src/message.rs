//! Message types for agent communication

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

impl MessageRole {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "user",
            MessageRole::Assistant => "assistant",
            MessageRole::System => "system",
            MessageRole::Tool => "tool",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "user" => Ok(MessageRole::User),
            "assistant" => Ok(MessageRole::Assistant),
            "system" => Ok(MessageRole::System),
            "tool" => Ok(MessageRole::Tool),
            _ => Err(crate::Error::Other(format!("Unknown message role: {}", s))),
        }
    }
}

/// A tool call made by the assistant
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool call ID
    pub id: String,
    /// Tool name
    pub name: String,
    /// Tool input as JSON
    pub input: serde_json::Value,
}

/// A tool result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool call ID this result is for
    pub tool_call_id: String,
    /// Result content
    pub content: String,
    /// Whether the tool call was successful
    pub is_error: bool,
}

/// A message in an agent conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID
    pub id: i64,
    /// Agent ID this message belongs to
    pub agent_id: Uuid,
    /// Role of the message sender
    pub role: MessageRole,
    /// Message content
    pub content: String,
    /// Tool calls (for assistant messages)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Tool results (for tool messages)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<ToolResult>>,
    /// Input tokens used
    pub input_tokens: i32,
    /// Output tokens used
    pub output_tokens: i32,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
}

impl Message {
    /// Create a new user message
    pub fn user(agent_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: 0, // Will be set by database
            agent_id,
            role: MessageRole::User,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            input_tokens: 0,
            output_tokens: 0,
            created_at: Utc::now(),
        }
    }

    /// Create a new assistant message
    pub fn assistant(agent_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: 0,
            agent_id,
            role: MessageRole::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            input_tokens: 0,
            output_tokens: 0,
            created_at: Utc::now(),
        }
    }

    /// Create a new system message
    pub fn system(agent_id: Uuid, content: impl Into<String>) -> Self {
        Self {
            id: 0,
            agent_id,
            role: MessageRole::System,
            content: content.into(),
            tool_calls: None,
            tool_results: None,
            input_tokens: 0,
            output_tokens: 0,
            created_at: Utc::now(),
        }
    }

    /// Create a new tool result message
    pub fn tool_result(agent_id: Uuid, results: Vec<ToolResult>) -> Self {
        Self {
            id: 0,
            agent_id,
            role: MessageRole::Tool,
            content: String::new(),
            tool_calls: None,
            tool_results: Some(results),
            input_tokens: 0,
            output_tokens: 0,
            created_at: Utc::now(),
        }
    }

    /// Add tool calls to an assistant message
    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }

    /// Set token usage
    pub fn with_tokens(mut self, input: i32, output: i32) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self
    }

    /// Total tokens used
    pub fn total_tokens(&self) -> i32 {
        self.input_tokens + self.output_tokens
    }
}
