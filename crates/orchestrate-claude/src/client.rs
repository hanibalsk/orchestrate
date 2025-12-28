//! Claude API client
//!
//! Uses the secrecy crate to protect API keys in memory.
//! Supports prompt caching for reduced token costs.

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
    /// Enable prompt caching (beta feature)
    enable_caching: bool,
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
            enable_caching: config.enable_caching,
        }
    }

    /// Create a new message
    pub async fn create_message(&self, request: CreateMessageRequest) -> Result<MessageResponse> {
        let mut headers = vec![
            ("x-api-key", self.api_key.expose_secret().to_string()),
            ("anthropic-version", "2023-06-01".to_string()),
            ("content-type", "application/json".to_string()),
        ];

        // Enable prompt caching beta if configured
        if self.enable_caching {
            headers.push(("anthropic-beta", "prompt-caching-2024-07-31".to_string()));
        }

        let mut req_builder = self.client.post(format!("{}/messages", self.base_url));
        for (key, value) in headers {
            req_builder = req_builder.header(key, value);
        }

        let response = req_builder.json(&request).send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error = response.text().await?;
            anyhow::bail!("Claude API error ({}): {}", status, error);
        }

        Ok(response.json().await?)
    }

    /// Check if caching is enabled
    pub fn caching_enabled(&self) -> bool {
        self.enable_caching
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
    /// Enable prompt caching (reduces costs for repeated prompts)
    pub enable_caching: bool,
}

impl Default for ClaudeClientConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.anthropic.com/v1".to_string(),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            connect_timeout_secs: 30,
            enable_caching: true,  // Enabled by default for cost savings
        }
    }
}

/// Cache control for prompt caching
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String,
}

impl CacheControl {
    /// Create ephemeral cache control (5 minute TTL)
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
        }
    }
}

/// System prompt content block with optional cache control
#[derive(Debug, Clone, Serialize)]
pub struct SystemContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl SystemContent {
    /// Create a cacheable system content block
    pub fn cacheable(text: String) -> Self {
        Self {
            content_type: "text".to_string(),
            text,
            cache_control: Some(CacheControl::ephemeral()),
        }
    }

    /// Create a non-cached system content block
    pub fn plain(text: String) -> Self {
        Self {
            content_type: "text".to_string(),
            text,
            cache_control: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CreateMessageRequest {
    pub model: String,
    pub max_tokens: u32,
    pub messages: Vec<MessageContent>,
    /// System prompt - can be string or structured content for caching
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

impl CreateMessageRequest {
    /// Create a new request with a simple string system prompt
    pub fn new(model: String, max_tokens: u32, messages: Vec<MessageContent>) -> Self {
        Self {
            model,
            max_tokens,
            messages,
            system: None,
            tools: None,
        }
    }

    /// Set system prompt as a plain string (no caching)
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(serde_json::Value::String(system.into()));
        self
    }

    /// Set system prompt with caching enabled
    /// The prompt is split into a cacheable base and dynamic suffix
    pub fn with_cached_system(mut self, base_prompt: &str, dynamic_suffix: Option<&str>) -> Self {
        let mut contents = vec![SystemContent::cacheable(base_prompt.to_string())];

        if let Some(suffix) = dynamic_suffix {
            if !suffix.is_empty() {
                contents.push(SystemContent::plain(suffix.to_string()));
            }
        }

        self.system = Some(serde_json::to_value(contents).unwrap_or_default());
        self
    }

    /// Set tools
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }
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
    /// Cache control for tool definition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl Tool {
    /// Create a new tool
    pub fn new(name: impl Into<String>, description: impl Into<String>, input_schema: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            cache_control: None,
        }
    }

    /// Make this tool cacheable
    pub fn cacheable(mut self) -> Self {
        self.cache_control = Some(CacheControl::ephemeral());
        self
    }
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

#[derive(Debug, Clone, Deserialize, Default)]
pub struct Usage {
    pub input_tokens: i32,
    pub output_tokens: i32,
    /// Tokens read from cache (reduced cost)
    #[serde(default)]
    pub cache_read_input_tokens: i32,
    /// Tokens written to cache (one-time cost)
    #[serde(default)]
    pub cache_creation_input_tokens: i32,
}

impl Usage {
    /// Calculate effective input tokens (accounting for cache savings)
    /// Cached tokens cost 90% less, so effective = regular + (cached * 0.1)
    pub fn effective_input_tokens(&self) -> f64 {
        let regular = self.input_tokens - self.cache_read_input_tokens - self.cache_creation_input_tokens;
        let cache_read = self.cache_read_input_tokens as f64 * 0.1;  // 90% discount
        let cache_write = self.cache_creation_input_tokens as f64 * 1.25;  // 25% premium for writing
        regular as f64 + cache_read + cache_write
    }

    /// Get cache hit rate as percentage
    pub fn cache_hit_rate(&self) -> f64 {
        if self.input_tokens == 0 {
            return 0.0;
        }
        (self.cache_read_input_tokens as f64 / self.input_tokens as f64) * 100.0
    }
}
