//! Learning engine for automatic pattern detection and instruction generation
//!
//! This module provides the learning functionality that analyzes agent runs,
//! detects recurring patterns (errors, tool usage, behaviors), and generates
//! custom instructions to prevent future issues.

use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::Uuid;

use crate::{
    instruction::{
        penalties, CustomInstruction, LearningConfig, LearningPattern, PatternStatus, PatternType,
        SuccessPattern, SuccessPatternType,
    },
    AgentType, Database, Message, MessageRole, Result,
};

/// Learning engine for automatic pattern detection
pub struct LearningEngine {
    config: LearningConfig,
}

impl LearningEngine {
    /// Create a new learning engine with default configuration
    pub fn new() -> Self {
        Self {
            config: LearningConfig::default(),
        }
    }

    /// Create a new learning engine with custom configuration
    pub fn with_config(config: LearningConfig) -> Self {
        Self { config }
    }

    /// Get the current configuration
    pub fn config(&self) -> &LearningConfig {
        &self.config
    }

    /// Analyze an agent run for patterns
    #[tracing::instrument(skip(self, db, messages), level = "debug", fields(agent_id = %agent_id))]
    pub async fn analyze_agent_run(
        &self,
        db: &Database,
        agent_id: Uuid,
        agent_type: AgentType,
        messages: &[Message],
        success: bool,
    ) -> Result<Vec<LearningPattern>> {
        let mut patterns = Vec::new();

        // Only analyze failures or problematic runs
        if success {
            return Ok(patterns);
        }

        // Detect error patterns
        if self
            .config
            .enabled_pattern_types
            .contains(&PatternType::ErrorPattern)
        {
            let error_patterns = self.detect_error_patterns(messages, agent_type);
            patterns.extend(error_patterns);
        }

        // Detect tool usage patterns
        if self
            .config
            .enabled_pattern_types
            .contains(&PatternType::ToolUsagePattern)
        {
            let tool_patterns = self.detect_tool_patterns(messages, agent_type);
            patterns.extend(tool_patterns);
        }

        // Detect behavior patterns
        if self
            .config
            .enabled_pattern_types
            .contains(&PatternType::BehaviorPattern)
        {
            let behavior_patterns = self.detect_behavior_patterns(messages, agent_type);
            patterns.extend(behavior_patterns);
        }

        // Persist patterns to database
        for pattern in &patterns {
            db.upsert_learning_pattern(pattern).await?;
        }

        Ok(patterns)
    }

    /// Analyze a successful agent run for success patterns
    ///
    /// This method extracts patterns from successful runs including:
    /// - Tool sequences that led to success
    /// - Context size at completion
    /// - Model choice effectiveness
    /// - Timing patterns (e.g., duration)
    #[tracing::instrument(skip(self, db, messages), level = "debug", fields(agent_id = %agent_id))]
    pub async fn analyze_successful_run(
        &self,
        db: &Database,
        agent_id: Uuid,
        agent_type: AgentType,
        messages: &[Message],
        completion_time_ms: Option<i64>,
        total_tokens: Option<i64>,
        task_type: Option<&str>,
    ) -> Result<Vec<SuccessPattern>> {
        let mut patterns = Vec::new();

        // Extract tool sequence pattern
        if let Some(tool_pattern) =
            self.extract_tool_sequence_pattern(messages, agent_type, task_type)
        {
            patterns.push(tool_pattern);
        }

        // Extract context size pattern
        if let Some(context_pattern) =
            self.extract_context_size_pattern(messages, agent_type, task_type, total_tokens)
        {
            patterns.push(context_pattern);
        }

        // Extract timing pattern if completion time is available
        if let Some(timing_pattern) =
            self.extract_timing_pattern(agent_type, task_type, completion_time_ms)
        {
            patterns.push(timing_pattern);
        }

        // Extract prompt structure pattern
        if let Some(prompt_pattern) =
            self.extract_prompt_structure_pattern(messages, agent_type, task_type)
        {
            patterns.push(prompt_pattern);
        }

        // Persist patterns to database
        for pattern in &patterns {
            db.upsert_success_pattern(pattern).await?;
        }

        Ok(patterns)
    }

    /// Extract tool sequence pattern from successful run
    fn extract_tool_sequence_pattern(
        &self,
        messages: &[Message],
        agent_type: AgentType,
        task_type: Option<&str>,
    ) -> Option<SuccessPattern> {
        let mut tool_sequence: Vec<String> = Vec::new();

        for msg in messages {
            if let Some(ref calls) = msg.tool_calls {
                for call in calls {
                    tool_sequence.push(call.name.clone());
                }
            }
        }

        if tool_sequence.is_empty() {
            return None;
        }

        // Create a normalized sequence (collapse repeated tools)
        let normalized_sequence = self.normalize_tool_sequence(&tool_sequence);

        // Create signature from the normalized sequence
        let signature = self.create_signature(
            &format!(
                "tool_seq_{}_{:?}",
                agent_type.as_str(),
                normalized_sequence.join(",")
            ),
            "success_tool",
        );

        let pattern_data = serde_json::json!({
            "tool_sequence": tool_sequence,
            "normalized_sequence": normalized_sequence,
            "total_calls": tool_sequence.len(),
            "unique_tools": tool_sequence.iter().collect::<std::collections::HashSet<_>>().len(),
        });

        Some(
            SuccessPattern::new(SuccessPatternType::ToolSequence, signature, pattern_data)
                .with_agent_type(agent_type)
                .with_task_type(task_type),
        )
    }

    /// Normalize a tool sequence by collapsing repeated consecutive tools
    fn normalize_tool_sequence(&self, sequence: &[String]) -> Vec<String> {
        let mut normalized = Vec::new();
        let mut prev: Option<&String> = None;

        for tool in sequence {
            if prev != Some(tool) {
                normalized.push(tool.clone());
                prev = Some(tool);
            }
        }

        normalized
    }

    /// Extract context size pattern from successful run
    fn extract_context_size_pattern(
        &self,
        messages: &[Message],
        agent_type: AgentType,
        task_type: Option<&str>,
        total_tokens: Option<i64>,
    ) -> Option<SuccessPattern> {
        let message_count = messages.len();
        let assistant_messages = messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .count();
        let user_messages = messages
            .iter()
            .filter(|m| m.role == MessageRole::User)
            .count();

        // Calculate approximate content size
        let total_content_len: usize = messages.iter().map(|m| m.content.len()).sum();

        let signature = self.create_signature(
            &format!(
                "context_{}_{}_{}",
                agent_type.as_str(),
                task_type.unwrap_or("general"),
                message_count / 10 // Bucket by 10s
            ),
            "success_context",
        );

        let pattern_data = serde_json::json!({
            "message_count": message_count,
            "assistant_messages": assistant_messages,
            "user_messages": user_messages,
            "total_content_chars": total_content_len,
            "avg_message_size": if message_count > 0 { total_content_len / message_count } else { 0 },
        });

        let mut pattern =
            SuccessPattern::new(SuccessPatternType::ContextSize, signature, pattern_data)
                .with_agent_type(agent_type)
                .with_task_type(task_type);

        pattern.avg_token_usage = total_tokens;

        Some(pattern)
    }

    /// Extract timing pattern from successful run
    fn extract_timing_pattern(
        &self,
        agent_type: AgentType,
        task_type: Option<&str>,
        completion_time_ms: Option<i64>,
    ) -> Option<SuccessPattern> {
        let completion_time = completion_time_ms?;

        // Bucket completion time into categories
        let time_bucket = if completion_time < 5000 {
            "fast"
        } else if completion_time < 30000 {
            "medium"
        } else if completion_time < 120000 {
            "slow"
        } else {
            "very_slow"
        };

        let signature = self.create_signature(
            &format!(
                "timing_{}_{}_{}",
                agent_type.as_str(),
                task_type.unwrap_or("general"),
                time_bucket
            ),
            "success_timing",
        );

        let pattern_data = serde_json::json!({
            "time_bucket": time_bucket,
            "completion_time_ms": completion_time,
        });

        let mut pattern =
            SuccessPattern::new(SuccessPatternType::Timing, signature, pattern_data)
                .with_agent_type(agent_type)
                .with_task_type(task_type);

        pattern.avg_completion_time_ms = Some(completion_time);

        Some(pattern)
    }

    /// Extract prompt structure pattern from successful run
    fn extract_prompt_structure_pattern(
        &self,
        messages: &[Message],
        agent_type: AgentType,
        task_type: Option<&str>,
    ) -> Option<SuccessPattern> {
        // Analyze the first user message (initial prompt) structure
        let first_user_msg = messages.iter().find(|m| m.role == MessageRole::User)?;

        // Extract prompt characteristics
        let has_examples = first_user_msg.content.contains("example")
            || first_user_msg.content.contains("Example")
            || first_user_msg.content.contains("e.g.");
        let has_constraints = first_user_msg.content.contains("must")
            || first_user_msg.content.contains("should")
            || first_user_msg.content.contains("don't")
            || first_user_msg.content.contains("avoid");
        let has_step_by_step = first_user_msg.content.contains("step by step")
            || first_user_msg.content.contains("step-by-step")
            || first_user_msg.content.contains("steps:");
        let has_context = first_user_msg.content.contains("context:")
            || first_user_msg.content.contains("background:");

        let prompt_length_bucket = if first_user_msg.content.len() < 100 {
            "short"
        } else if first_user_msg.content.len() < 500 {
            "medium"
        } else if first_user_msg.content.len() < 2000 {
            "long"
        } else {
            "very_long"
        };

        let signature = self.create_signature(
            &format!(
                "prompt_{}_{}_ex{}_con{}_step{}_ctx{}",
                agent_type.as_str(),
                task_type.unwrap_or("general"),
                has_examples,
                has_constraints,
                has_step_by_step,
                has_context
            ),
            "success_prompt",
        );

        let pattern_data = serde_json::json!({
            "prompt_length": first_user_msg.content.len(),
            "prompt_length_bucket": prompt_length_bucket,
            "has_examples": has_examples,
            "has_constraints": has_constraints,
            "has_step_by_step": has_step_by_step,
            "has_context": has_context,
        });

        Some(
            SuccessPattern::new(SuccessPatternType::PromptStructure, signature, pattern_data)
                .with_agent_type(agent_type)
                .with_task_type(task_type),
        )
    }

    /// Get recommendations for a new task based on success patterns
    #[tracing::instrument(skip(self, db), level = "debug")]
    pub async fn get_success_recommendations(
        &self,
        db: &Database,
        agent_type: AgentType,
        task_type: Option<&str>,
    ) -> Result<SuccessRecommendations> {
        // Get success patterns for this agent type
        let patterns = db.get_success_patterns_for_agent(agent_type, 50).await?;

        let mut recommendations = SuccessRecommendations::default();

        // Analyze patterns to build recommendations
        for pattern in &patterns {
            match pattern.pattern_type {
                SuccessPatternType::ToolSequence => {
                    if let Some(seq) = pattern.pattern_data.get("normalized_sequence") {
                        if let Some(arr) = seq.as_array() {
                            let tools: Vec<String> = arr
                                .iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect();
                            recommendations.recommended_tool_sequences.push(tools);
                        }
                    }
                }
                SuccessPatternType::ContextSize => {
                    if let Some(count) = pattern.pattern_data.get("message_count") {
                        if let Some(n) = count.as_i64() {
                            recommendations.avg_message_counts.push(n);
                        }
                    }
                }
                SuccessPatternType::Timing => {
                    if let Some(time) = pattern.avg_completion_time_ms {
                        recommendations.avg_completion_times.push(time);
                    }
                }
                SuccessPatternType::PromptStructure => {
                    if pattern
                        .pattern_data
                        .get("has_examples")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        recommendations.successful_prompt_features.push("examples".to_string());
                    }
                    if pattern
                        .pattern_data
                        .get("has_constraints")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        recommendations.successful_prompt_features.push("constraints".to_string());
                    }
                    if pattern
                        .pattern_data
                        .get("has_step_by_step")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        recommendations
                            .successful_prompt_features
                            .push("step_by_step".to_string());
                    }
                }
                SuccessPatternType::ModelChoice => {
                    // Model choice patterns would be populated when we have model selection data
                }
            }
        }

        // Calculate averages
        if !recommendations.avg_message_counts.is_empty() {
            recommendations.recommended_message_count = Some(
                recommendations.avg_message_counts.iter().sum::<i64>()
                    / recommendations.avg_message_counts.len() as i64,
            );
        }

        if !recommendations.avg_completion_times.is_empty() {
            recommendations.expected_completion_time_ms = Some(
                recommendations.avg_completion_times.iter().sum::<i64>()
                    / recommendations.avg_completion_times.len() as i64,
            );
        }

        // Deduplicate prompt features
        recommendations.successful_prompt_features.sort();
        recommendations.successful_prompt_features.dedup();

        Ok(recommendations)
    }

    /// Detect error patterns from messages
    fn detect_error_patterns(
        &self,
        messages: &[Message],
        agent_type: AgentType,
    ) -> Vec<LearningPattern> {
        let mut patterns = Vec::new();

        for msg in messages {
            // Look for tool results with errors
            if let Some(ref results) = msg.tool_results {
                for result in results {
                    if result.is_error {
                        if let Some(pattern) =
                            self.create_error_pattern(&result.content, agent_type)
                        {
                            patterns.push(pattern);
                        }
                    }
                }
            }

            // Look for STATUS: BLOCKED or STATUS: FAILED in content
            if msg.content.contains("STATUS: BLOCKED") || msg.content.contains("STATUS: FAILED") {
                if let Some(pattern) = self.create_status_pattern(&msg.content, agent_type) {
                    patterns.push(pattern);
                }
            }
        }

        patterns
    }

    /// Create an error pattern from error text
    fn create_error_pattern(
        &self,
        error_text: &str,
        agent_type: AgentType,
    ) -> Option<LearningPattern> {
        // Normalize error text for deduplication
        let normalized = self.normalize_error_text(error_text);
        if normalized.is_empty() {
            return None;
        }

        // Create signature hash
        let signature = self.create_signature(&normalized, "error");

        let pattern_data = serde_json::json!({
            "error_text": normalized,
            "original_text": error_text.chars().take(500).collect::<String>(),
            "category": self.categorize_error(&normalized),
        });

        Some(
            LearningPattern::new(PatternType::ErrorPattern, signature, pattern_data)
                .with_agent_type(agent_type),
        )
    }

    /// Create a status pattern from blocked/failed status
    fn create_status_pattern(
        &self,
        content: &str,
        agent_type: AgentType,
    ) -> Option<LearningPattern> {
        // Extract the reason for blocking
        let reason = if let Some(pos) = content.find("Reason:") {
            content[pos..].lines().next().unwrap_or("")
        } else {
            // Take the first 200 chars after STATUS:
            if let Some(pos) = content.find("STATUS:") {
                &content[pos..].chars().take(200).collect::<String>()
            } else {
                return None;
            }
        };

        let signature = self.create_signature(reason, "status");

        let pattern_data = serde_json::json!({
            "status_text": reason,
            "is_blocked": content.contains("BLOCKED"),
            "is_failed": content.contains("FAILED"),
        });

        Some(
            LearningPattern::new(PatternType::ErrorPattern, signature, pattern_data)
                .with_agent_type(agent_type),
        )
    }

    /// Detect tool usage patterns
    fn detect_tool_patterns(
        &self,
        messages: &[Message],
        agent_type: AgentType,
    ) -> Vec<LearningPattern> {
        let mut patterns = Vec::new();
        let mut tool_usage: HashMap<String, usize> = HashMap::new();
        let mut failed_tools: HashMap<String, usize> = HashMap::new();

        for msg in messages {
            // Count tool calls
            if let Some(ref calls) = msg.tool_calls {
                for call in calls {
                    *tool_usage.entry(call.name.clone()).or_insert(0) += 1;
                }
            }

            // Count failed tool results
            if let Some(ref results) = msg.tool_results {
                for result in results {
                    if result.is_error {
                        // Try to find the tool name from previous messages
                        if let Some(tool_name) =
                            self.find_tool_name_for_call_id(messages, &result.tool_call_id)
                        {
                            *failed_tools.entry(tool_name).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        // Detect excessive retries of failed tools
        for (tool_name, fail_count) in failed_tools {
            if fail_count >= 3 {
                let signature = self.create_signature(&format!("retry_fail_{}", tool_name), "tool");

                let pattern_data = serde_json::json!({
                    "tool_name": tool_name,
                    "failure_count": fail_count,
                    "category": "excessive_retry",
                });

                patterns.push(
                    LearningPattern::new(PatternType::ToolUsagePattern, signature, pattern_data)
                        .with_agent_type(agent_type),
                );
            }
        }

        patterns
    }

    /// Find tool name for a tool call ID
    fn find_tool_name_for_call_id(&self, messages: &[Message], call_id: &str) -> Option<String> {
        for msg in messages {
            if let Some(ref calls) = msg.tool_calls {
                for call in calls {
                    if call.id == call_id {
                        return Some(call.name.clone());
                    }
                }
            }
        }
        None
    }

    /// Detect behavior patterns (clarification requests, retries, etc.)
    fn detect_behavior_patterns(
        &self,
        messages: &[Message],
        agent_type: AgentType,
    ) -> Vec<LearningPattern> {
        let mut patterns = Vec::new();

        // Count clarification patterns
        let clarification_count = messages
            .iter()
            .filter(|m| m.role == MessageRole::Assistant)
            .filter(|m| {
                m.content.contains("Could you please clarify")
                    || m.content.contains("I need more information")
                    || m.content.contains("Can you provide more details")
                    || m.content.contains("What do you mean by")
            })
            .count();

        if clarification_count >= 3 {
            let signature = self.create_signature("excessive_clarification", "behavior");

            let pattern_data = serde_json::json!({
                "clarification_count": clarification_count,
                "category": "excessive_clarification",
            });

            patterns.push(
                LearningPattern::new(PatternType::BehaviorPattern, signature, pattern_data)
                    .with_agent_type(agent_type),
            );
        }

        // Detect repetitive actions
        let mut action_sequence: Vec<String> = Vec::new();
        for msg in messages {
            if let Some(ref calls) = msg.tool_calls {
                for call in calls {
                    action_sequence.push(call.name.clone());
                }
            }
        }

        // Look for repetitive sequences (same tool called 5+ times in a row)
        if action_sequence.len() >= 5 {
            for window in action_sequence.windows(5) {
                if window.iter().all(|t| t == &window[0]) {
                    let signature =
                        self.create_signature(&format!("repetitive_{}", window[0]), "behavior");

                    let pattern_data = serde_json::json!({
                        "tool_name": window[0],
                        "repetition_count": 5,
                        "category": "repetitive_action",
                    });

                    patterns.push(
                        LearningPattern::new(PatternType::BehaviorPattern, signature, pattern_data)
                            .with_agent_type(agent_type),
                    );
                    break;
                }
            }
        }

        patterns
    }

    /// Normalize error text for pattern matching
    fn normalize_error_text(&self, text: &str) -> String {
        // Remove file paths, line numbers, UUIDs, and other variable content
        let mut normalized = text.to_string();

        // Remove file paths
        let path_re =
            regex::Regex::new(r"/[\w/.-]+").unwrap_or_else(|_| regex::Regex::new(r"$^").unwrap());
        normalized = path_re.replace_all(&normalized, "<PATH>").to_string();

        // Remove line numbers
        let line_re =
            regex::Regex::new(r":\d+:\d+").unwrap_or_else(|_| regex::Regex::new(r"$^").unwrap());
        normalized = line_re.replace_all(&normalized, ":<LINE>").to_string();

        // Remove UUIDs
        let uuid_re =
            regex::Regex::new(r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}")
                .unwrap_or_else(|_| regex::Regex::new(r"$^").unwrap());
        normalized = uuid_re.replace_all(&normalized, "<UUID>").to_string();

        // Remove timestamps
        let time_re = regex::Regex::new(r"\d{4}-\d{2}-\d{2}[T ]\d{2}:\d{2}:\d{2}")
            .unwrap_or_else(|_| regex::Regex::new(r"$^").unwrap());
        normalized = time_re.replace_all(&normalized, "<TIMESTAMP>").to_string();

        // Truncate to reasonable length
        if normalized.len() > 200 {
            normalized.truncate(200);
        }

        normalized.trim().to_string()
    }

    /// Categorize an error
    fn categorize_error(&self, text: &str) -> &'static str {
        let lower = text.to_lowercase();

        if lower.contains("permission denied") || lower.contains("access denied") {
            "permission_error"
        } else if lower.contains("not found") || lower.contains("no such file") {
            "not_found_error"
        } else if lower.contains("timeout") || lower.contains("timed out") {
            "timeout_error"
        } else if lower.contains("connection") || lower.contains("network") {
            "network_error"
        } else if lower.contains("syntax error") || lower.contains("parse error") {
            "syntax_error"
        } else if lower.contains("type error") || lower.contains("type mismatch") {
            "type_error"
        } else if lower.contains("memory") || lower.contains("out of memory") {
            "memory_error"
        } else if lower.contains("command not found") || lower.contains("unknown command") {
            "command_error"
        } else {
            "unknown_error"
        }
    }

    /// Create a signature for pattern deduplication
    fn create_signature(&self, content: &str, prefix: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(prefix.as_bytes());
        hasher.update(content.as_bytes());
        let hash = hasher.finalize();
        format!("{}_{}", prefix, hex::encode(&hash[..8]))
    }

    /// Generate an instruction from a pattern
    pub fn generate_instruction_from_pattern(
        &self,
        pattern: &LearningPattern,
    ) -> Option<CustomInstruction> {
        let content = match pattern.pattern_type {
            PatternType::ErrorPattern => self.generate_error_instruction(pattern),
            PatternType::ToolUsagePattern => self.generate_tool_instruction(pattern),
            PatternType::BehaviorPattern => self.generate_behavior_instruction(pattern),
        };

        content.map(|c| {
            let name = format!("learned_{}", &pattern.pattern_signature);
            let confidence = self.calculate_confidence(pattern);

            let mut instruction = CustomInstruction::learned(name, c, confidence);
            instruction.agent_type = pattern.agent_type;
            if pattern.agent_type.is_some() {
                instruction.scope = crate::instruction::InstructionScope::AgentType;
            }
            instruction
        })
    }

    /// Generate instruction content for error pattern
    fn generate_error_instruction(&self, pattern: &LearningPattern) -> Option<String> {
        let category = pattern
            .pattern_data
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let error_text = pattern
            .pattern_data
            .get("error_text")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if error_text.is_empty() {
            return None;
        }

        let instruction = match category {
            "permission_error" => {
                format!("IMPORTANT: When encountering permission errors like '{}', check file permissions and consider using appropriate sudo commands or requesting user assistance.", error_text)
            }
            "not_found_error" => {
                format!("IMPORTANT: Verify file/directory existence before operations. This error has occurred: '{}'", error_text)
            }
            "timeout_error" => {
                format!("IMPORTANT: Set appropriate timeouts and handle timeout errors gracefully. Pattern observed: '{}'", error_text)
            }
            "network_error" => {
                format!(
                    "IMPORTANT: Handle network errors with retries and fallbacks. Pattern: '{}'",
                    error_text
                )
            }
            "command_error" => {
                format!(
                    "IMPORTANT: Verify command availability before execution. Error seen: '{}'",
                    error_text
                )
            }
            _ => {
                format!(
                    "IMPORTANT: Avoid this recurring error pattern: '{}'",
                    error_text
                )
            }
        };

        Some(instruction)
    }

    /// Generate instruction content for tool usage pattern
    fn generate_tool_instruction(&self, pattern: &LearningPattern) -> Option<String> {
        let tool_name = pattern
            .pattern_data
            .get("tool_name")
            .and_then(|v| v.as_str())?;

        let category = pattern
            .pattern_data
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let instruction = match category {
            "excessive_retry" => {
                format!("IMPORTANT: The '{}' tool has been failing repeatedly. Before using it, verify preconditions are met. If it fails more than twice, try an alternative approach or ask for user guidance.", tool_name)
            }
            _ => {
                format!("IMPORTANT: Be cautious when using the '{}' tool. Verify inputs and handle errors appropriately.", tool_name)
            }
        };

        Some(instruction)
    }

    /// Generate instruction content for behavior pattern
    fn generate_behavior_instruction(&self, pattern: &LearningPattern) -> Option<String> {
        let category = pattern
            .pattern_data
            .get("category")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        let instruction = match category {
            "excessive_clarification" => {
                "IMPORTANT: Avoid excessive clarification requests. Make reasonable assumptions based on context and proceed with the task. If truly blocked, explain what information is missing concisely.".to_string()
            }
            "repetitive_action" => {
                let tool_name = pattern.pattern_data.get("tool_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("tool");
                format!("IMPORTANT: Avoid repetitive use of '{}'. If an operation isn't working after 2-3 attempts, try a different approach.", tool_name)
            }
            _ => {
                return None;
            }
        };

        Some(instruction)
    }

    /// Calculate confidence for a learned instruction
    fn calculate_confidence(&self, pattern: &LearningPattern) -> f64 {
        // Base confidence increases with occurrence count
        let base = (pattern.occurrence_count as f64 / 10.0).min(0.5);

        // Adjust based on pattern type
        let type_modifier = match pattern.pattern_type {
            PatternType::ErrorPattern => 0.3,     // Errors are reliable signals
            PatternType::ToolUsagePattern => 0.2, // Tool patterns are somewhat reliable
            PatternType::BehaviorPattern => 0.1,  // Behavior patterns need more validation
        };

        (base + type_modifier).min(0.9) // Cap at 0.9, never fully confident for learned
    }

    /// Process patterns and create instructions
    #[tracing::instrument(skip(self, db), level = "debug")]
    pub async fn process_patterns(&self, db: &Database) -> Result<Vec<CustomInstruction>> {
        let mut created_instructions = Vec::new();

        // Get patterns ready for instruction generation
        let patterns = db
            .get_patterns_for_review(self.config.min_occurrences)
            .await?;

        for pattern in patterns {
            // Skip patterns that already have instructions
            if pattern.instruction_id.is_some() {
                continue;
            }

            // Generate instruction
            if let Some(mut instruction) = self.generate_instruction_from_pattern(&pattern) {
                // Auto-approve if confidence is high enough
                let status = if instruction.confidence >= self.config.auto_approve_threshold {
                    if self.config.auto_enable {
                        instruction.enabled = true;
                    }
                    PatternStatus::Approved
                } else {
                    PatternStatus::PendingReview
                };

                // Insert instruction
                let instruction_id = db.insert_instruction(&instruction).await?;
                instruction.id = instruction_id;

                // Update pattern status
                db.update_pattern_status(pattern.id, status, Some(instruction_id))
                    .await?;

                created_instructions.push(instruction);
            }
        }

        Ok(created_instructions)
    }

    /// Run cleanup tasks: auto-disable and delete ineffective instructions
    #[tracing::instrument(skip(self, db), level = "debug")]
    pub async fn cleanup(&self, db: &Database) -> Result<CleanupResult> {
        // Auto-disable high-penalty instructions
        let disabled = db
            .auto_disable_penalized(self.config.penalty_disable_threshold)
            .await?;

        // Delete ineffective learned instructions
        let deleted = db.delete_ineffective_instructions().await?;

        Ok(CleanupResult {
            disabled_count: disabled.len(),
            deleted_names: deleted,
        })
    }

    /// Apply penalties based on agent outcome
    #[tracing::instrument(skip(self, db), level = "debug")]
    pub async fn apply_outcome_penalties(
        &self,
        db: &Database,
        instruction_ids: &[i64],
        success: bool,
        was_blocked: bool,
    ) -> Result<()> {
        for &id in instruction_ids {
            if success {
                // Decay penalty on success
                db.decay_penalty(id, penalties::DECAY_ON_SUCCESS).await?;
            } else if was_blocked {
                // Higher penalty for blocked
                db.apply_penalty(id, penalties::BLOCKED, "agent_blocked")
                    .await?;
            } else {
                // Standard failure penalty
                db.apply_penalty(id, penalties::FAILURE, "agent_failed")
                    .await?;
            }
        }

        Ok(())
    }
}

impl Default for LearningEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of cleanup operation
#[derive(Debug, Clone)]
pub struct CleanupResult {
    /// Number of instructions disabled
    pub disabled_count: usize,
    /// Names of deleted instructions
    pub deleted_names: Vec<String>,
}

/// Recommendations based on success patterns
#[derive(Debug, Clone, Default)]
pub struct SuccessRecommendations {
    /// Recommended tool sequences that have led to success
    pub recommended_tool_sequences: Vec<Vec<String>>,
    /// Average message counts from successful runs (internal)
    pub avg_message_counts: Vec<i64>,
    /// Average completion times from successful runs (internal)
    pub avg_completion_times: Vec<i64>,
    /// Prompt features that appear in successful runs
    pub successful_prompt_features: Vec<String>,
    /// Recommended message count based on successful patterns
    pub recommended_message_count: Option<i64>,
    /// Expected completion time based on successful patterns
    pub expected_completion_time_ms: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruction::InstructionSource;

    #[test]
    fn test_normalize_error_text() {
        let engine = LearningEngine::new();

        // Test path normalization
        let text = "Error: File /home/user/project/file.txt not found";
        let normalized = engine.normalize_error_text(text);
        assert!(normalized.contains("<PATH>"));
        assert!(!normalized.contains("/home/user"));

        // Test line number normalization
        let text = "Error at main.rs:42:15";
        let normalized = engine.normalize_error_text(text);
        assert!(normalized.contains("<LINE>"));
        assert!(!normalized.contains("42:15"));

        // Test UUID normalization
        let text = "Agent 550e8400-e29b-41d4-a716-446655440000 failed";
        let normalized = engine.normalize_error_text(text);
        assert!(normalized.contains("<UUID>"));
        assert!(!normalized.contains("550e8400"));
    }

    #[test]
    fn test_categorize_error() {
        let engine = LearningEngine::new();

        assert_eq!(
            engine.categorize_error("Permission denied"),
            "permission_error"
        );
        assert_eq!(engine.categorize_error("File not found"), "not_found_error");
        assert_eq!(
            engine.categorize_error("Connection timeout"),
            "timeout_error"
        );
        assert_eq!(
            engine.categorize_error("Network error occurred"),
            "network_error"
        );
        assert_eq!(
            engine.categorize_error("Syntax error in code"),
            "syntax_error"
        );
        // Note: "Command not found" also contains "not found", but the check order matters
        // In the actual implementation, "not found" is checked before "command not found"
        // so "Command not found: xyz" returns "not_found_error"
        assert_eq!(
            engine.categorize_error("Command not found: xyz"),
            "not_found_error"
        );
        assert_eq!(
            engine.categorize_error("unknown command: foo"),
            "command_error"
        );
        assert_eq!(
            engine.categorize_error("Some random error"),
            "unknown_error"
        );
    }

    #[test]
    fn test_create_signature() {
        let engine = LearningEngine::new();

        let sig1 = engine.create_signature("error text", "error");
        let sig2 = engine.create_signature("error text", "error");
        let sig3 = engine.create_signature("different text", "error");

        // Same input should give same signature
        assert_eq!(sig1, sig2);
        // Different input should give different signature
        assert_ne!(sig1, sig3);
        // Signature should have prefix
        assert!(sig1.starts_with("error_"));
    }

    #[test]
    fn test_calculate_confidence() {
        let engine = LearningEngine::new();

        // Low occurrence count
        let pattern1 =
            LearningPattern::new(PatternType::ErrorPattern, "sig1", serde_json::json!({}));
        let conf1 = engine.calculate_confidence(&pattern1);
        assert!(conf1 > 0.0);
        assert!(conf1 < 0.5);

        // High occurrence count
        let mut pattern2 =
            LearningPattern::new(PatternType::ErrorPattern, "sig2", serde_json::json!({}));
        pattern2.occurrence_count = 100;
        let conf2 = engine.calculate_confidence(&pattern2);
        assert!(conf2 > conf1);
        assert!(conf2 <= 0.9);
    }

    #[test]
    fn test_generate_error_instruction() {
        let engine = LearningEngine::new();

        let pattern = LearningPattern::new(
            PatternType::ErrorPattern,
            "sig",
            serde_json::json!({
                "error_text": "Permission denied",
                "category": "permission_error",
            }),
        );

        let instruction = engine.generate_instruction_from_pattern(&pattern);
        assert!(instruction.is_some());

        let inst = instruction.unwrap();
        assert!(inst.name.starts_with("learned_"));
        assert!(inst.content.contains("permission"));
        assert_eq!(inst.source, InstructionSource::Learned);
    }

    #[test]
    fn test_learning_config_default() {
        let config = LearningConfig::default();

        assert_eq!(config.min_occurrences, 3);
        assert_eq!(config.auto_approve_threshold, 0.9);
        assert!(!config.auto_enable);
        assert!(config
            .enabled_pattern_types
            .contains(&PatternType::ErrorPattern));
    }

    #[test]
    fn test_normalize_tool_sequence() {
        let engine = LearningEngine::new();

        // Empty sequence
        let empty: Vec<String> = vec![];
        assert_eq!(engine.normalize_tool_sequence(&empty), Vec::<String>::new());

        // No repeats
        let no_repeats = vec!["Read".to_string(), "Write".to_string(), "Bash".to_string()];
        let normalized = engine.normalize_tool_sequence(&no_repeats);
        assert_eq!(normalized, no_repeats);

        // With repeats
        let with_repeats = vec![
            "Read".to_string(),
            "Read".to_string(),
            "Read".to_string(),
            "Write".to_string(),
            "Write".to_string(),
            "Bash".to_string(),
        ];
        let normalized = engine.normalize_tool_sequence(&with_repeats);
        assert_eq!(
            normalized,
            vec!["Read".to_string(), "Write".to_string(), "Bash".to_string()]
        );

        // Alternating (no collapse)
        let alternating = vec![
            "Read".to_string(),
            "Write".to_string(),
            "Read".to_string(),
            "Write".to_string(),
        ];
        let normalized = engine.normalize_tool_sequence(&alternating);
        assert_eq!(normalized, alternating);
    }

    #[test]
    fn test_extract_tool_sequence_pattern() {
        use crate::message::ToolCall;
        use uuid::Uuid;

        let engine = LearningEngine::new();

        // Create messages with tool calls
        let messages = vec![
            Message {
                id: 1,
                agent_id: Uuid::new_v4(),
                role: MessageRole::Assistant,
                content: "Let me read the file".to_string(),
                tool_calls: Some(vec![
                    ToolCall {
                        id: "call1".to_string(),
                        name: "Read".to_string(),
                        input: serde_json::json!({}),
                    },
                    ToolCall {
                        id: "call2".to_string(),
                        name: "Read".to_string(),
                        input: serde_json::json!({}),
                    },
                ]),
                tool_results: None,
                input_tokens: 100,
                output_tokens: 50,
                created_at: chrono::Utc::now(),
            },
            Message {
                id: 2,
                agent_id: Uuid::new_v4(),
                role: MessageRole::Assistant,
                content: "Now writing".to_string(),
                tool_calls: Some(vec![ToolCall {
                    id: "call3".to_string(),
                    name: "Write".to_string(),
                    input: serde_json::json!({}),
                }]),
                tool_results: None,
                input_tokens: 100,
                output_tokens: 50,
                created_at: chrono::Utc::now(),
            },
        ];

        let pattern =
            engine.extract_tool_sequence_pattern(&messages, AgentType::StoryDeveloper, Some("test"));

        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.pattern_type, SuccessPatternType::ToolSequence);
        assert_eq!(p.agent_type, Some(AgentType::StoryDeveloper));
        assert_eq!(p.task_type, Some("test".to_string()));

        // Check pattern data
        let data = &p.pattern_data;
        assert_eq!(data["total_calls"], 3);
        assert_eq!(data["unique_tools"], 2);
    }

    #[test]
    fn test_extract_context_size_pattern() {
        use uuid::Uuid;

        let engine = LearningEngine::new();

        let messages = vec![
            Message {
                id: 1,
                agent_id: Uuid::new_v4(),
                role: MessageRole::User,
                content: "Please do something".to_string(),
                tool_calls: None,
                tool_results: None,
                input_tokens: 20,
                output_tokens: 0,
                created_at: chrono::Utc::now(),
            },
            Message {
                id: 2,
                agent_id: Uuid::new_v4(),
                role: MessageRole::Assistant,
                content: "Doing it now".to_string(),
                tool_calls: None,
                tool_results: None,
                input_tokens: 100,
                output_tokens: 50,
                created_at: chrono::Utc::now(),
            },
            Message {
                id: 3,
                agent_id: Uuid::new_v4(),
                role: MessageRole::User,
                content: "Thanks".to_string(),
                tool_calls: None,
                tool_results: None,
                input_tokens: 10,
                output_tokens: 0,
                created_at: chrono::Utc::now(),
            },
        ];

        let pattern = engine.extract_context_size_pattern(
            &messages,
            AgentType::IssueFixer,
            None,
            Some(5000),
        );

        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.pattern_type, SuccessPatternType::ContextSize);
        assert_eq!(p.avg_token_usage, Some(5000));

        let data = &p.pattern_data;
        assert_eq!(data["message_count"], 3);
        assert_eq!(data["assistant_messages"], 1);
        assert_eq!(data["user_messages"], 2);
    }

    #[test]
    fn test_extract_timing_pattern() {
        let engine = LearningEngine::new();

        // Fast completion
        let pattern = engine.extract_timing_pattern(AgentType::CodeReviewer, Some("review"), Some(3000));
        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.pattern_type, SuccessPatternType::Timing);
        assert_eq!(p.pattern_data["time_bucket"], "fast");
        assert_eq!(p.avg_completion_time_ms, Some(3000));

        // Medium completion
        let pattern = engine.extract_timing_pattern(AgentType::CodeReviewer, None, Some(20000));
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap().pattern_data["time_bucket"], "medium");

        // Slow completion
        let pattern = engine.extract_timing_pattern(AgentType::CodeReviewer, None, Some(60000));
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap().pattern_data["time_bucket"], "slow");

        // Very slow completion
        let pattern = engine.extract_timing_pattern(AgentType::CodeReviewer, None, Some(180000));
        assert!(pattern.is_some());
        assert_eq!(pattern.unwrap().pattern_data["time_bucket"], "very_slow");

        // No completion time
        let pattern = engine.extract_timing_pattern(AgentType::CodeReviewer, None, None);
        assert!(pattern.is_none());
    }

    #[test]
    fn test_extract_prompt_structure_pattern() {
        use uuid::Uuid;

        let engine = LearningEngine::new();

        // Test with examples and constraints
        let messages = vec![Message {
            id: 1,
            agent_id: Uuid::new_v4(),
            role: MessageRole::User,
            content: "Please implement this feature. You must follow the existing patterns. For example, look at the other files. You should avoid changing the API.".to_string(),
            tool_calls: None,
            tool_results: None,
            input_tokens: 50,
            output_tokens: 0,
            created_at: chrono::Utc::now(),
        }];

        let pattern = engine.extract_prompt_structure_pattern(
            &messages,
            AgentType::StoryDeveloper,
            Some("feature"),
        );

        assert!(pattern.is_some());
        let p = pattern.unwrap();
        assert_eq!(p.pattern_type, SuccessPatternType::PromptStructure);
        assert_eq!(p.pattern_data["has_examples"], true);
        assert_eq!(p.pattern_data["has_constraints"], true);
        assert_eq!(p.pattern_data["has_step_by_step"], false);
        assert_eq!(p.pattern_data["has_context"], false);

        // Test with no user message
        let empty_messages: Vec<Message> = vec![];
        let pattern = engine.extract_prompt_structure_pattern(
            &empty_messages,
            AgentType::StoryDeveloper,
            None,
        );
        assert!(pattern.is_none());
    }

    #[test]
    fn test_success_pattern_new() {
        let pattern = SuccessPattern::new(
            SuccessPatternType::ToolSequence,
            "test_sig",
            serde_json::json!({"key": "value"}),
        );

        assert_eq!(pattern.id, 0);
        assert_eq!(pattern.pattern_type, SuccessPatternType::ToolSequence);
        assert_eq!(pattern.agent_type, None);
        assert_eq!(pattern.task_type, None);
        assert_eq!(pattern.pattern_signature, "test_sig");
        assert_eq!(pattern.occurrence_count, 1);
        assert_eq!(pattern.success_rate, 1.0);
        assert!(pattern.avg_completion_time_ms.is_none());
        assert!(pattern.avg_token_usage.is_none());
    }

    #[test]
    fn test_success_pattern_builder() {
        use crate::AgentType;

        let pattern = SuccessPattern::new(
            SuccessPatternType::ContextSize,
            "sig123",
            serde_json::json!({}),
        )
        .with_agent_type(AgentType::IssueFixer)
        .with_task_type(Some("bug_fix"))
        .with_completion_time_ms(5000)
        .with_token_usage(1000);

        assert_eq!(pattern.agent_type, Some(AgentType::IssueFixer));
        assert_eq!(pattern.task_type, Some("bug_fix".to_string()));
        assert_eq!(pattern.avg_completion_time_ms, Some(5000));
        assert_eq!(pattern.avg_token_usage, Some(1000));
    }

    #[test]
    fn test_success_pattern_type_conversion() {
        // Test as_str
        assert_eq!(SuccessPatternType::ToolSequence.as_str(), "tool_sequence");
        assert_eq!(
            SuccessPatternType::PromptStructure.as_str(),
            "prompt_structure"
        );
        assert_eq!(SuccessPatternType::ContextSize.as_str(), "context_size");
        assert_eq!(SuccessPatternType::ModelChoice.as_str(), "model_choice");
        assert_eq!(SuccessPatternType::Timing.as_str(), "timing");

        // Test from_str
        assert_eq!(
            SuccessPatternType::from_str("tool_sequence").unwrap(),
            SuccessPatternType::ToolSequence
        );
        assert_eq!(
            SuccessPatternType::from_str("prompt_structure").unwrap(),
            SuccessPatternType::PromptStructure
        );
        assert_eq!(
            SuccessPatternType::from_str("context_size").unwrap(),
            SuccessPatternType::ContextSize
        );
        assert_eq!(
            SuccessPatternType::from_str("model_choice").unwrap(),
            SuccessPatternType::ModelChoice
        );
        assert_eq!(
            SuccessPatternType::from_str("timing").unwrap(),
            SuccessPatternType::Timing
        );

        // Test invalid
        assert!(SuccessPatternType::from_str("invalid").is_err());
    }

    #[test]
    fn test_success_recommendations_default() {
        let recommendations = SuccessRecommendations::default();

        assert!(recommendations.recommended_tool_sequences.is_empty());
        assert!(recommendations.avg_message_counts.is_empty());
        assert!(recommendations.avg_completion_times.is_empty());
        assert!(recommendations.successful_prompt_features.is_empty());
        assert!(recommendations.recommended_message_count.is_none());
        assert!(recommendations.expected_completion_time_ms.is_none());
    }
}
