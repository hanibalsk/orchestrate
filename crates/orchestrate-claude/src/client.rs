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
            enable_caching: true, // Enabled by default for cost savings
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
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
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
        let regular =
            self.input_tokens - self.cache_read_input_tokens - self.cache_creation_input_tokens;
        let cache_read = self.cache_read_input_tokens as f64 * 0.1; // 90% discount
        let cache_write = self.cache_creation_input_tokens as f64 * 1.25; // 25% premium for writing
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

// ==================== Claude CLI Client ====================

/// Claude CLI client - uses the `claude` command line tool
/// This leverages OAuth authentication from Claude Code
#[derive(Clone)]
pub struct ClaudeCliClient {
    model: String,
    /// Working directory for the CLI
    working_dir: Option<std::path::PathBuf>,
}

impl ClaudeCliClient {
    /// Create a new CLI client with default model
    pub fn new() -> Self {
        Self {
            model: "sonnet".to_string(),
            working_dir: None,
        }
    }

    /// Create a CLI client with specific model
    pub fn with_model(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            working_dir: None,
        }
    }

    /// Set working directory
    pub fn with_working_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Create a new message using claude CLI
    pub async fn create_message(&self, request: CreateMessageRequest) -> Result<MessageResponse> {
        use tokio::process::Command;

        // Build the prompt from messages
        let prompt = self.build_prompt(&request)?;

        // Build command
        let mut cmd = Command::new("claude");
        cmd.arg("-p")
            .arg("--output-format")
            .arg("json")
            .arg("--model")
            .arg(&self.model);

        // Add system prompt if present
        if let Some(ref system) = request.system {
            if let Some(s) = system.as_str() {
                cmd.arg("--system-prompt").arg(s);
            }
        }

        // Add tools if present
        if let Some(ref tools) = request.tools {
            let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
            if !tool_names.is_empty() {
                // Map our tool names to Claude CLI tool names
                let cli_tools = self.map_tools_to_cli(&tool_names);
                if !cli_tools.is_empty() {
                    cmd.arg("--tools").arg(cli_tools.join(","));
                }
            }
        }

        // Set working directory
        if let Some(ref dir) = self.working_dir {
            cmd.current_dir(dir);
        }

        // Pass prompt via stdin
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        let mut child = cmd.spawn()?;

        // Write prompt to stdin
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(prompt.as_bytes()).await?;
        }

        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Claude CLI failed: {}", stderr);
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        self.parse_cli_response(&stdout)
    }

    /// Build prompt string from request messages
    fn build_prompt(&self, request: &CreateMessageRequest) -> Result<String> {
        let mut prompt = String::new();

        for msg in &request.messages {
            let role = &msg.role;
            let content = match &msg.content {
                serde_json::Value::String(s) => s.clone(),
                serde_json::Value::Array(arr) => {
                    // Handle array of content blocks
                    let mut text = String::new();
                    for block in arr {
                        if let Some(t) = block.get("text").and_then(|v| v.as_str()) {
                            text.push_str(t);
                            text.push('\n');
                        }
                    }
                    text
                }
                _ => serde_json::to_string(&msg.content)?,
            };

            if role == "user" {
                prompt.push_str(&content);
                prompt.push('\n');
            }
            // Assistant messages are context, handled differently
        }

        Ok(prompt)
    }

    /// Map our tool names to Claude CLI tool names
    fn map_tools_to_cli(&self, tools: &[&str]) -> Vec<String> {
        tools
            .iter()
            .filter_map(|t| match *t {
                "bash" => Some("Bash".to_string()),
                "read" => Some("Read".to_string()),
                "write" => Some("Write".to_string()),
                "edit" => Some("Edit".to_string()),
                "glob" => Some("Glob".to_string()),
                "grep" => Some("Grep".to_string()),
                "task" => Some("Task".to_string()),
                _ => None,
            })
            .collect()
    }

    /// Parse CLI JSON response into MessageResponse
    fn parse_cli_response(&self, stdout: &str) -> Result<MessageResponse> {
        #[derive(Deserialize)]
        struct CliResponse {
            #[serde(rename = "type")]
            #[allow(dead_code)]
            response_type: String,
            result: Option<String>,
            is_error: bool,
            usage: Option<CliUsage>,
            #[serde(default)]
            session_id: String,
        }

        #[derive(Deserialize)]
        struct CliUsage {
            input_tokens: i32,
            output_tokens: i32,
            #[serde(default)]
            cache_read_input_tokens: i32,
            #[serde(default)]
            cache_creation_input_tokens: i32,
        }

        let cli_resp: CliResponse = serde_json::from_str(stdout)?;

        if cli_resp.is_error {
            anyhow::bail!("Claude CLI error: {:?}", cli_resp.result);
        }

        let content = cli_resp.result.unwrap_or_default();
        let usage = cli_resp
            .usage
            .map(|u| Usage {
                input_tokens: u.input_tokens,
                output_tokens: u.output_tokens,
                cache_read_input_tokens: u.cache_read_input_tokens,
                cache_creation_input_tokens: u.cache_creation_input_tokens,
            })
            .unwrap_or_default();

        Ok(MessageResponse {
            id: cli_resp.session_id,
            model: self.model.clone(),
            stop_reason: Some("end_turn".to_string()),
            content: vec![ContentBlock::Text { text: content }],
            usage,
        })
    }
}

impl Default for ClaudeCliClient {
    fn default() -> Self {
        Self::new()
    }
}
