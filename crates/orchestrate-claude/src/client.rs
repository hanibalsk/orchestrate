//! Claude API client
//!
//! Uses the secrecy crate to protect API keys in memory.

use anyhow::Result;
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Default timeout for API requests
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// Claude API client
#[derive(Clone)]
pub struct ClaudeClient {
    api_key: SecretString,
    base_url: String,
    client: reqwest::Client,
}

impl ClaudeClient {
    /// Create a new Claude client with default settings
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::with_config(api_key, ClaudeClientConfig::default())
    }

    /// Create a new Claude client with custom configuration
    pub fn with_config(api_key: impl Into<String>, config: ClaudeClientConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .connect_timeout(Duration::from_secs(config.connect_timeout_secs))
            .build()
            .expect("Failed to build HTTP client");

        Self {
            api_key: SecretString::new(api_key.into()),
            base_url: config.base_url,
            client,
        }
    }

    /// Create a new message
    pub async fn create_message(&self, request: CreateMessageRequest) -> Result<MessageResponse> {
        let response = self
            .client
            .post(format!("{}/messages", self.base_url))
            .header("x-api-key", self.api_key.expose_secret())
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error = response.text().await?;
            anyhow::bail!("Claude API error ({}): {}", status, error);
        }

        Ok(response.json().await?)
    }
}

/// Configuration for the Claude client
pub struct ClaudeClientConfig {
    /// Base URL for the API
    pub base_url: String,
    /// Request timeout in seconds
    pub timeout_secs: u64,
    /// Connection timeout in seconds
    pub connect_timeout_secs: u64,
}

impl Default for ClaudeClientConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.anthropic.com/v1".to_string(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            connect_timeout_secs: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateMessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<MessageContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageContent {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MessageResponse {
    pub id: String,
    pub content: Vec<ContentBlock>,
    pub model: String,
    pub stop_reason: Option<String>,
    pub usage: Usage,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub input_tokens: i32,
    pub output_tokens: i32,
}
