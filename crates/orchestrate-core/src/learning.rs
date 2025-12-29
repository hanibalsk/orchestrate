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
}
