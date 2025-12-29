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

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== MessageRole Tests ====================

    #[test]
    fn test_message_role_as_str() {
        assert_eq!(MessageRole::User.as_str(), "user");
        assert_eq!(MessageRole::Assistant.as_str(), "assistant");
        assert_eq!(MessageRole::System.as_str(), "system");
        assert_eq!(MessageRole::Tool.as_str(), "tool");
    }

    #[test]
    fn test_message_role_from_str() {
        assert_eq!(MessageRole::from_str("user").unwrap(), MessageRole::User);
        assert_eq!(
            MessageRole::from_str("assistant").unwrap(),
            MessageRole::Assistant
        );
        assert_eq!(
            MessageRole::from_str("system").unwrap(),
            MessageRole::System
        );
        assert_eq!(MessageRole::from_str("tool").unwrap(), MessageRole::Tool);
        assert!(MessageRole::from_str("invalid").is_err());
    }

    // ==================== Message Tests ====================

    #[test]
    fn test_user_message() {
        let agent_id = Uuid::new_v4();
        let msg = Message::user(agent_id, "Hello, world!");

        assert_eq!(msg.agent_id, agent_id);
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content, "Hello, world!");
        assert!(msg.tool_calls.is_none());
        assert!(msg.tool_results.is_none());
        assert_eq!(msg.input_tokens, 0);
        assert_eq!(msg.output_tokens, 0);
    }

    #[test]
    fn test_assistant_message() {
        let agent_id = Uuid::new_v4();
        let msg = Message::assistant(agent_id, "I can help with that.");

        assert_eq!(msg.role, MessageRole::Assistant);
        assert_eq!(msg.content, "I can help with that.");
    }

    #[test]
    fn test_system_message() {
        let agent_id = Uuid::new_v4();
        let msg = Message::system(agent_id, "You are a helpful assistant.");

        assert_eq!(msg.role, MessageRole::System);
        assert_eq!(msg.content, "You are a helpful assistant.");
    }

    #[test]
    fn test_tool_result_message() {
        let agent_id = Uuid::new_v4();
        let results = vec![
            ToolResult {
                tool_call_id: "call_123".to_string(),
                content: "File content here".to_string(),
                is_error: false,
            },
            ToolResult {
                tool_call_id: "call_456".to_string(),
                content: "Error: file not found".to_string(),
                is_error: true,
            },
        ];

        let msg = Message::tool_result(agent_id, results);

        assert_eq!(msg.role, MessageRole::Tool);
        assert!(msg.content.is_empty());
        assert!(msg.tool_results.is_some());
        let tool_results = msg.tool_results.unwrap();
        assert_eq!(tool_results.len(), 2);
        assert!(!tool_results[0].is_error);
        assert!(tool_results[1].is_error);
    }

    #[test]
    fn test_message_with_tool_calls() {
        let agent_id = Uuid::new_v4();
        let tool_calls = vec![ToolCall {
            id: "call_123".to_string(),
            name: "Read".to_string(),
            input: serde_json::json!({"file_path": "/tmp/test.txt"}),
        }];

        let msg =
            Message::assistant(agent_id, "Let me read that file.").with_tool_calls(tool_calls);

        assert!(msg.tool_calls.is_some());
        let calls = msg.tool_calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "Read");
    }

    #[test]
    fn test_message_with_tokens() {
        let agent_id = Uuid::new_v4();
        let msg = Message::assistant(agent_id, "Response").with_tokens(100, 50);

        assert_eq!(msg.input_tokens, 100);
        assert_eq!(msg.output_tokens, 50);
        assert_eq!(msg.total_tokens(), 150);
    }

    #[test]
    fn test_total_tokens() {
        let agent_id = Uuid::new_v4();
        let mut msg = Message::user(agent_id, "Test");
        msg.input_tokens = 25;
        msg.output_tokens = 75;

        assert_eq!(msg.total_tokens(), 100);
    }

    // ==================== ToolCall Tests ====================

    #[test]
    fn test_tool_call_serialization() {
        let tool_call = ToolCall {
            id: "call_abc".to_string(),
            name: "Bash".to_string(),
            input: serde_json::json!({
                "command": "ls -la",
                "timeout": 5000
            }),
        };

        let json = serde_json::to_string(&tool_call).unwrap();
        let parsed: ToolCall = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "call_abc");
        assert_eq!(parsed.name, "Bash");
        assert_eq!(parsed.input["command"], "ls -la");
    }

    // ==================== ToolResult Tests ====================

    #[test]
    fn test_tool_result_success() {
        let result = ToolResult {
            tool_call_id: "call_123".to_string(),
            content: "Operation completed successfully".to_string(),
            is_error: false,
        };

        assert!(!result.is_error);
        assert_eq!(result.tool_call_id, "call_123");
    }

    #[test]
    fn test_tool_result_error() {
        let result = ToolResult {
            tool_call_id: "call_456".to_string(),
            content: "Permission denied".to_string(),
            is_error: true,
        };

        assert!(result.is_error);
    }

    #[test]
    fn test_tool_result_serialization() {
        let result = ToolResult {
            tool_call_id: "call_789".to_string(),
            content: "Result content".to_string(),
            is_error: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: ToolResult = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.tool_call_id, result.tool_call_id);
        assert_eq!(parsed.content, result.content);
        assert_eq!(parsed.is_error, result.is_error);
    }
}
