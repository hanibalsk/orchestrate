//! Token estimation and context management
//!
//! This module provides:
//! - Token estimation for messages and prompts
//! - Message windowing to stay within context limits
//! - Conversation summarization for older messages
//! - Dynamic token allocation for output

use orchestrate_core::Message;

/// Approximate tokens per character (Claude uses ~4 chars per token on average)
const CHARS_PER_TOKEN: f64 = 4.0;

/// Model context limits
#[derive(Debug, Clone, Copy)]
pub struct ModelLimits {
    /// Maximum context window size
    pub max_context_tokens: usize,
    /// Maximum output tokens
    pub max_output_tokens: usize,
    /// Reserved tokens for system prompt (including cached portion)
    pub system_prompt_reserve: usize,
    /// Reserved tokens for tool definitions
    pub tools_reserve: usize,
    /// Minimum tokens to reserve for model output
    pub min_output_reserve: usize,
}

impl ModelLimits {
    /// Claude Sonnet 4 limits
    pub fn sonnet_4() -> Self {
        Self {
            max_context_tokens: 200_000,
            max_output_tokens: 8192,
            system_prompt_reserve: 8000,
            tools_reserve: 4000,
            min_output_reserve: 4096,
        }
    }

    /// Claude Haiku 3 limits (for explorer agent)
    pub fn haiku_3() -> Self {
        Self {
            max_context_tokens: 200_000,
            max_output_tokens: 4096,
            system_prompt_reserve: 4000,
            tools_reserve: 2000,
            min_output_reserve: 2048,
        }
    }

    /// Get limits for a model name
    pub fn for_model(model: &str) -> Self {
        if model.contains("haiku") {
            Self::haiku_3()
        } else {
            Self::sonnet_4()
        }
    }

    /// Calculate available tokens for conversation messages
    pub fn available_for_messages(&self) -> usize {
        self.max_context_tokens
            .saturating_sub(self.system_prompt_reserve)
            .saturating_sub(self.tools_reserve)
            .saturating_sub(self.min_output_reserve)
    }
}

/// Token estimation configuration
#[derive(Debug, Clone)]
pub struct TokenConfig {
    /// Model limits
    pub limits: ModelLimits,
    /// Target token usage (as fraction of available, e.g., 0.8 = 80%)
    pub target_usage: f64,
    /// Number of recent messages to always include
    pub min_recent_messages: usize,
    /// Enable message summarization
    pub enable_summarization: bool,
    /// Tokens reserved for summary
    pub summary_tokens: usize,
}

impl Default for TokenConfig {
    fn default() -> Self {
        Self {
            limits: ModelLimits::sonnet_4(),
            target_usage: 0.85,
            min_recent_messages: 10,
            enable_summarization: true,
            summary_tokens: 2000,
        }
    }
}

impl TokenConfig {
    /// Create config for a specific model
    pub fn for_model(model: &str) -> Self {
        Self {
            limits: ModelLimits::for_model(model),
            ..Default::default()
        }
    }

    /// Calculate target tokens for messages
    pub fn target_message_tokens(&self) -> usize {
        let available = self.limits.available_for_messages();
        let target = (available as f64 * self.target_usage) as usize;
        if self.enable_summarization {
            target.saturating_sub(self.summary_tokens)
        } else {
            target
        }
    }
}

/// Token estimator for messages and text
#[derive(Debug, Clone, Default)]
pub struct TokenEstimator;

impl TokenEstimator {
    /// Create a new token estimator
    pub fn new() -> Self {
        Self
    }

    /// Estimate tokens for a string
    pub fn estimate_text(&self, text: &str) -> usize {
        // Use character count / 4 as approximation
        // This is a rough estimate; for production, use tiktoken or similar
        let char_count = text.chars().count();
        ((char_count as f64) / CHARS_PER_TOKEN).ceil() as usize
    }

    /// Estimate tokens for a message
    pub fn estimate_message(&self, message: &Message) -> usize {
        let mut tokens = self.estimate_text(&message.content);

        // Add overhead for role and structure (~4 tokens)
        tokens += 4;

        // Add tokens for tool calls
        if let Some(ref tool_calls) = message.tool_calls {
            for call in tool_calls {
                tokens += self.estimate_text(&call.name);
                tokens += self.estimate_text(&call.input.to_string());
                tokens += 10; // Overhead for structure
            }
        }

        // Add tokens for tool results
        if let Some(ref tool_results) = message.tool_results {
            for result in tool_results {
                tokens += self.estimate_text(&result.content);
                tokens += 5; // Overhead for structure
            }
        }

        tokens
    }

    /// Estimate total tokens for a list of messages
    pub fn estimate_messages(&self, messages: &[Message]) -> usize {
        messages.iter().map(|m| self.estimate_message(m)).sum()
    }

    /// Estimate tokens for system prompt
    pub fn estimate_system_prompt(&self, prompt: &str) -> usize {
        self.estimate_text(prompt) + 10 // Small overhead
    }
}

/// Result of message windowing
#[derive(Debug, Clone)]
pub struct WindowedMessages {
    /// Summary of older messages (if any were summarized)
    pub summary: Option<String>,
    /// Token count of the summary
    pub summary_tokens: usize,
    /// Messages to include in the request
    pub messages: Vec<Message>,
    /// Token count of included messages
    pub message_tokens: usize,
    /// Number of messages that were summarized/dropped
    pub summarized_count: usize,
    /// Total original message count
    pub original_count: usize,
}

/// Context manager for message windowing and summarization
#[derive(Debug, Clone)]
pub struct ContextManager {
    config: TokenConfig,
    estimator: TokenEstimator,
}

impl ContextManager {
    /// Create a new context manager
    pub fn new(config: TokenConfig) -> Self {
        Self {
            config,
            estimator: TokenEstimator::new(),
        }
    }

    /// Create with default config for a model
    pub fn for_model(model: &str) -> Self {
        Self::new(TokenConfig::for_model(model))
    }

    /// Get the token estimator
    pub fn estimator(&self) -> &TokenEstimator {
        &self.estimator
    }

    /// Window messages to fit within token budget
    ///
    /// This keeps recent messages and optionally summarizes older ones.
    /// Returns windowed messages with optional summary.
    pub fn window_messages(&self, messages: &[Message]) -> WindowedMessages {
        let target_tokens = self.config.target_message_tokens();
        let total_tokens = self.estimator.estimate_messages(messages);

        // If we're under budget, return all messages
        if total_tokens <= target_tokens {
            return WindowedMessages {
                summary: None,
                summary_tokens: 0,
                messages: messages.to_vec(),
                message_tokens: total_tokens,
                summarized_count: 0,
                original_count: messages.len(),
            };
        }

        // Need to window - keep recent messages
        let min_recent = self.config.min_recent_messages.min(messages.len());

        // Start from the end and work backwards
        let mut included_messages = Vec::new();
        let mut included_tokens = 0;
        let mut start_idx = messages.len();

        // Always include at least min_recent messages
        for i in (0..messages.len()).rev() {
            let msg = &messages[i];
            let msg_tokens = self.estimator.estimate_message(msg);

            // Check if we have room or if we must include (min_recent)
            let remaining = messages.len() - i;
            if remaining <= min_recent || included_tokens + msg_tokens <= target_tokens {
                included_tokens += msg_tokens;
                start_idx = i;
            } else {
                break;
            }
        }

        // Collect the messages we're keeping
        included_messages = messages[start_idx..].to_vec();
        let summarized_count = start_idx;

        // Generate summary if we dropped messages
        let (summary, summary_tokens) = if summarized_count > 0 && self.config.enable_summarization {
            let summary = self.generate_summary(&messages[..summarized_count]);
            let tokens = self.estimator.estimate_text(&summary);
            (Some(summary), tokens)
        } else {
            (None, 0)
        };

        WindowedMessages {
            summary,
            summary_tokens,
            messages: included_messages,
            message_tokens: included_tokens,
            summarized_count,
            original_count: messages.len(),
        }
    }

    /// Generate a summary of older messages
    ///
    /// This creates a structured summary that captures key information
    /// without needing an LLM call (for speed and cost).
    fn generate_summary(&self, messages: &[Message]) -> String {
        let mut summary_parts = Vec::new();

        // Count message types
        let user_count = messages.iter().filter(|m| m.role == orchestrate_core::MessageRole::User).count();
        let assistant_count = messages.iter().filter(|m| m.role == orchestrate_core::MessageRole::Assistant).count();
        let tool_count = messages.iter().filter(|m| m.role == orchestrate_core::MessageRole::Tool).count();

        summary_parts.push(format!(
            "[CONTEXT SUMMARY: {} earlier messages ({} user, {} assistant, {} tool results) omitted for context limit]",
            messages.len(), user_count, assistant_count, tool_count
        ));

        // Extract key tool calls and their outcomes
        let mut tool_outcomes: Vec<String> = Vec::new();
        for msg in messages {
            if let Some(ref tool_calls) = msg.tool_calls {
                for call in tool_calls {
                    // Find corresponding result
                    let result_preview = messages.iter()
                        .filter_map(|m| m.tool_results.as_ref())
                        .flatten()
                        .find(|r| r.tool_call_id == call.id)
                        .map(|r| {
                            let status = if r.is_error { "ERROR" } else { "OK" };
                            let preview: String = r.content.chars().take(50).collect();
                            format!("{}: {}", status, preview)
                        })
                        .unwrap_or_else(|| "pending".to_string());

                    tool_outcomes.push(format!("- {}: {}", call.name, result_preview));
                }
            }
        }

        if !tool_outcomes.is_empty() {
            summary_parts.push("Tool calls in summarized context:".to_string());
            // Limit to most recent 10 tool outcomes
            let start = tool_outcomes.len().saturating_sub(10);
            summary_parts.extend(tool_outcomes[start..].iter().cloned());
        }

        // Include the first user message (original task) if present
        if let Some(first_user) = messages.iter().find(|m| m.role == orchestrate_core::MessageRole::User) {
            let task_preview: String = first_user.content.chars().take(200).collect();
            summary_parts.push(format!("Original task: {}...", task_preview));
        }

        summary_parts.join("\n")
    }

    /// Calculate optimal max_tokens for output based on context usage
    pub fn calculate_output_tokens(&self, context_tokens: usize) -> usize {
        let remaining = self.config.limits.max_context_tokens.saturating_sub(context_tokens);

        // Use at most max_output_tokens, at least min_output_reserve
        remaining
            .min(self.config.limits.max_output_tokens)
            .max(self.config.limits.min_output_reserve)
    }
}

/// Cache status for prompt caching
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheStatus {
    /// Content is not cached
    NotCached,
    /// Content can be cached (mark with cache_control)
    Cacheable,
    /// Content is cached (from previous request)
    Cached,
}

/// Prompt with caching information
#[derive(Debug, Clone)]
pub struct CachedPrompt {
    /// The prompt content
    pub content: String,
    /// Estimated token count
    pub tokens: usize,
    /// Cache status
    pub cache_status: CacheStatus,
    /// Hash for cache key
    pub content_hash: u64,
}

impl CachedPrompt {
    /// Create a new cached prompt
    pub fn new(content: String) -> Self {
        let estimator = TokenEstimator::new();
        let tokens = estimator.estimate_text(&content);
        let content_hash = Self::hash_content(&content);

        Self {
            content,
            tokens,
            cache_status: CacheStatus::Cacheable,
            content_hash,
        }
    }

    /// Hash content for cache key
    fn hash_content(content: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_token_estimation_text() {
        let estimator = TokenEstimator::new();

        // 40 chars should be ~10 tokens
        let text = "This is a test string of forty chars!";
        let tokens = estimator.estimate_text(text);
        assert!(tokens >= 8 && tokens <= 15, "Expected ~10 tokens, got {}", tokens);
    }

    #[test]
    fn test_token_estimation_message() {
        let estimator = TokenEstimator::new();
        let agent_id = Uuid::new_v4();

        let msg = Message::user(agent_id, "Hello, world!");
        let tokens = estimator.estimate_message(&msg);

        // Should include content tokens + overhead
        assert!(tokens >= 5, "Expected at least 5 tokens, got {}", tokens);
    }

    #[test]
    fn test_model_limits() {
        let sonnet = ModelLimits::sonnet_4();
        assert_eq!(sonnet.max_context_tokens, 200_000);

        let haiku = ModelLimits::haiku_3();
        assert_eq!(haiku.max_output_tokens, 4096);

        let from_model = ModelLimits::for_model("claude-3-haiku-20240307");
        assert_eq!(from_model.max_output_tokens, 4096);
    }

    #[test]
    fn test_context_manager_no_windowing_needed() {
        let config = TokenConfig::default();
        let manager = ContextManager::new(config);
        let agent_id = Uuid::new_v4();

        // Create a few short messages
        let messages: Vec<Message> = (0..5)
            .map(|i| Message::user(agent_id, format!("Message {}", i)))
            .collect();

        let result = manager.window_messages(&messages);

        assert!(result.summary.is_none());
        assert_eq!(result.messages.len(), 5);
        assert_eq!(result.summarized_count, 0);
    }

    #[test]
    fn test_context_manager_windowing() {
        let mut config = TokenConfig::default();
        config.limits.max_context_tokens = 1000;  // Very small for testing
        config.limits.system_prompt_reserve = 100;
        config.limits.tools_reserve = 100;
        config.limits.min_output_reserve = 100;
        config.min_recent_messages = 2;

        let manager = ContextManager::new(config);
        let agent_id = Uuid::new_v4();

        // Create many long messages
        let long_content = "x".repeat(500);  // ~125 tokens each
        let messages: Vec<Message> = (0..10)
            .map(|_| Message::user(agent_id, long_content.clone()))
            .collect();

        let result = manager.window_messages(&messages);

        // Should have dropped some messages
        assert!(result.messages.len() < 10);
        assert!(result.summarized_count > 0);
        assert!(result.summary.is_some());
    }

    #[test]
    fn test_cached_prompt() {
        let prompt = CachedPrompt::new("You are a helpful assistant.".to_string());

        assert!(prompt.tokens > 0);
        assert!(prompt.content_hash > 0);
        assert_eq!(prompt.cache_status, CacheStatus::Cacheable);
    }

    #[test]
    fn test_calculate_output_tokens() {
        let config = TokenConfig::default();
        let manager = ContextManager::new(config);

        // With low context usage, should get max output
        let output = manager.calculate_output_tokens(10_000);
        assert_eq!(output, 8192);  // max_output_tokens for Sonnet

        // With high context usage, should get at least min_output_reserve
        let output = manager.calculate_output_tokens(195_000);
        assert!(output >= 4096);
    }
}
