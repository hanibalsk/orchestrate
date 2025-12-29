//! Custom instructions for agent system prompts
//!
//! This module provides types for managing custom instructions that can be
//! injected into agent system prompts. Instructions can be global (apply to
//! all agents) or scoped to specific agent types.

use crate::AgentType;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Scope of instruction application
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionScope {
    /// Applies to all agents
    Global,
    /// Applies only to specific agent type
    AgentType,
}

impl InstructionScope {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            InstructionScope::Global => "global",
            InstructionScope::AgentType => "agent_type",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "global" => Ok(InstructionScope::Global),
            "agent_type" => Ok(InstructionScope::AgentType),
            _ => Err(crate::Error::Other(format!(
                "Unknown instruction scope: {}",
                s
            ))),
        }
    }
}

/// Source of instruction creation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstructionSource {
    /// Manually created by user
    Manual,
    /// Automatically learned from patterns
    Learned,
    /// Imported from external source
    Imported,
}

impl InstructionSource {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            InstructionSource::Manual => "manual",
            InstructionSource::Learned => "learned",
            InstructionSource::Imported => "imported",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "manual" => Ok(InstructionSource::Manual),
            "learned" => Ok(InstructionSource::Learned),
            "imported" => Ok(InstructionSource::Imported),
            _ => Err(crate::Error::Other(format!(
                "Unknown instruction source: {}",
                s
            ))),
        }
    }
}

/// A custom instruction that can be injected into agent prompts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomInstruction {
    /// Database ID (0 if not yet persisted)
    pub id: i64,
    /// Human-readable unique name
    pub name: String,
    /// The actual instruction content
    pub content: String,
    /// Scope of application
    pub scope: InstructionScope,
    /// Agent type (required if scope is AgentType)
    pub agent_type: Option<AgentType>,
    /// Priority (higher = injected earlier in prompt)
    pub priority: i32,
    /// Whether the instruction is enabled
    pub enabled: bool,
    /// Source of the instruction
    pub source: InstructionSource,
    /// Confidence score (0.0-1.0, used for learned instructions)
    pub confidence: f64,
    /// Tags for organization
    pub tags: Vec<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Creator identifier
    pub created_by: Option<String>,
}

impl CustomInstruction {
    /// Create a new global instruction
    pub fn global(name: impl Into<String>, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            name: name.into(),
            content: content.into(),
            scope: InstructionScope::Global,
            agent_type: None,
            priority: 100,
            enabled: true,
            source: InstructionSource::Manual,
            confidence: 1.0,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
            created_by: None,
        }
    }

    /// Create a new agent-type-specific instruction
    pub fn for_agent_type(
        name: impl Into<String>,
        content: impl Into<String>,
        agent_type: AgentType,
    ) -> Self {
        let mut inst = Self::global(name, content);
        inst.scope = InstructionScope::AgentType;
        inst.agent_type = Some(agent_type);
        inst
    }

    /// Create a learned instruction (lower default confidence, starts disabled)
    pub fn learned(name: impl Into<String>, content: impl Into<String>, confidence: f64) -> Self {
        let mut inst = Self::global(name, content);
        inst.source = InstructionSource::Learned;
        inst.confidence = confidence.clamp(0.0, 1.0);
        inst.enabled = false; // Learned instructions start disabled for review
        inst
    }

    /// Check if this instruction applies to a given agent type
    pub fn applies_to(&self, agent_type: AgentType) -> bool {
        if !self.enabled {
            return false;
        }
        match self.scope {
            InstructionScope::Global => true,
            InstructionScope::AgentType => self.agent_type == Some(agent_type),
        }
    }

    /// Set priority
    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    /// Set tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set source
    pub fn with_source(mut self, source: InstructionSource) -> Self {
        self.source = source;
        self
    }

    /// Set creator
    pub fn with_created_by(mut self, created_by: impl Into<String>) -> Self {
        self.created_by = Some(created_by.into());
        self
    }

    /// Disable the instruction
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

/// Effectiveness metrics for an instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionEffectiveness {
    /// Instruction ID
    pub instruction_id: i64,
    /// Number of times the instruction was applied
    pub usage_count: i64,
    /// Number of successful agent runs with this instruction
    pub success_count: i64,
    /// Number of failed agent runs with this instruction
    pub failure_count: i64,
    /// Accumulated penalty score (0.0-2.0)
    pub penalty_score: f64,
    /// Average completion time in seconds
    pub avg_completion_time: Option<f64>,
    /// Calculated success rate (success_count / usage_count)
    pub success_rate: f64,
    /// Last successful run timestamp
    pub last_success_at: Option<DateTime<Utc>>,
    /// Last failed run timestamp
    pub last_failure_at: Option<DateTime<Utc>>,
    /// When the last penalty was applied
    pub last_penalty_at: Option<DateTime<Utc>>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

impl InstructionEffectiveness {
    /// Create new effectiveness metrics for an instruction
    pub fn new(instruction_id: i64) -> Self {
        Self {
            instruction_id,
            usage_count: 0,
            success_count: 0,
            failure_count: 0,
            penalty_score: 0.0,
            avg_completion_time: None,
            success_rate: 0.0,
            last_success_at: None,
            last_failure_at: None,
            last_penalty_at: None,
            updated_at: Utc::now(),
        }
    }

    /// Calculate success rate from counts
    pub fn calculate_success_rate(&mut self) {
        if self.usage_count > 0 {
            self.success_rate = self.success_count as f64 / self.usage_count as f64;
        } else {
            self.success_rate = 0.0;
        }
    }

    /// Check if instruction should be disabled based on penalty
    pub fn should_disable(&self) -> bool {
        self.penalty_score >= 0.7
    }

    /// Check if instruction is eligible for deletion
    pub fn is_eligible_for_deletion(&self) -> bool {
        self.penalty_score >= 1.0 && self.usage_count >= 10 && self.success_rate < 0.3
    }
}

/// Penalty constants
pub mod penalties {
    /// Penalty for agent failure with instruction
    pub const FAILURE: f64 = 0.2;
    /// Penalty for agent blocked with instruction
    pub const BLOCKED: f64 = 0.15;
    /// Penalty for low success rate
    pub const LOW_SUCCESS_RATE: f64 = 0.1;
    /// Penalty for no improvement
    pub const NO_IMPROVEMENT: f64 = 0.05;
    /// Decay amount on success
    pub const DECAY_ON_SUCCESS: f64 = 0.01;
    /// Threshold for auto-disable
    pub const DISABLE_THRESHOLD: f64 = 0.7;
    /// Threshold for deletion eligibility
    pub const DELETE_THRESHOLD: f64 = 1.0;
    /// Maximum penalty score
    pub const MAX_PENALTY: f64 = 2.0;
}

/// Pattern types for automatic learning
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternType {
    /// Error patterns that recur
    ErrorPattern,
    /// Tool usage patterns
    ToolUsagePattern,
    /// Behavioral patterns
    BehaviorPattern,
}

impl PatternType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PatternType::ErrorPattern => "error_pattern",
            PatternType::ToolUsagePattern => "tool_usage_pattern",
            PatternType::BehaviorPattern => "behavior_pattern",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "error_pattern" => Ok(PatternType::ErrorPattern),
            "tool_usage_pattern" => Ok(PatternType::ToolUsagePattern),
            "behavior_pattern" => Ok(PatternType::BehaviorPattern),
            _ => Err(crate::Error::Other(format!("Unknown pattern type: {}", s))),
        }
    }
}

/// Status of a learning pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternStatus {
    /// Pattern has been observed
    Observed,
    /// Pattern pending human review
    PendingReview,
    /// Pattern approved, instruction created
    Approved,
    /// Pattern rejected
    Rejected,
}

impl PatternStatus {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PatternStatus::Observed => "observed",
            PatternStatus::PendingReview => "pending_review",
            PatternStatus::Approved => "approved",
            PatternStatus::Rejected => "rejected",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "observed" => Ok(PatternStatus::Observed),
            "pending_review" => Ok(PatternStatus::PendingReview),
            "approved" => Ok(PatternStatus::Approved),
            "rejected" => Ok(PatternStatus::Rejected),
            _ => Err(crate::Error::Other(format!(
                "Unknown pattern status: {}",
                s
            ))),
        }
    }
}

/// A learned pattern that may become an instruction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningPattern {
    /// Database ID
    pub id: i64,
    /// Type of pattern
    pub pattern_type: PatternType,
    /// Agent type (None for patterns that apply to all types)
    pub agent_type: Option<AgentType>,
    /// Unique signature for deduplication
    pub pattern_signature: String,
    /// Pattern data (JSON)
    pub pattern_data: serde_json::Value,
    /// Number of times this pattern was observed
    pub occurrence_count: i64,
    /// First observation timestamp
    pub first_seen_at: DateTime<Utc>,
    /// Last observation timestamp
    pub last_seen_at: DateTime<Utc>,
    /// Generated instruction ID (if any)
    pub instruction_id: Option<i64>,
    /// Current status
    pub status: PatternStatus,
}

impl LearningPattern {
    /// Create a new learning pattern
    pub fn new(
        pattern_type: PatternType,
        pattern_signature: impl Into<String>,
        pattern_data: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            pattern_type,
            agent_type: None,
            pattern_signature: pattern_signature.into(),
            pattern_data,
            occurrence_count: 1,
            first_seen_at: now,
            last_seen_at: now,
            instruction_id: None,
            status: PatternStatus::Observed,
        }
    }

    /// Set agent type
    pub fn with_agent_type(mut self, agent_type: AgentType) -> Self {
        self.agent_type = Some(agent_type);
        self
    }
}

/// Configuration for the learning system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LearningConfig {
    /// Minimum occurrences before pattern is considered for instruction generation
    pub min_occurrences: i64,
    /// Confidence threshold for auto-approving learned instructions
    pub auto_approve_threshold: f64,
    /// Whether to auto-enable approved instructions
    pub auto_enable: bool,
    /// Pattern types to learn from
    pub enabled_pattern_types: Vec<PatternType>,
    /// Penalty threshold for auto-disabling instructions
    pub penalty_disable_threshold: f64,
    /// Minimum usage count before instruction can be deleted
    pub min_usage_for_deletion: i64,
    /// Success rate threshold below which instruction can be deleted
    pub deletion_success_rate_threshold: f64,
}

impl Default for LearningConfig {
    fn default() -> Self {
        Self {
            min_occurrences: 3,
            auto_approve_threshold: 0.9,
            auto_enable: false,
            enabled_pattern_types: vec![PatternType::ErrorPattern, PatternType::ToolUsagePattern],
            penalty_disable_threshold: penalties::DISABLE_THRESHOLD,
            min_usage_for_deletion: 10,
            deletion_success_rate_threshold: 0.3,
        }
    }
}

/// Types of success patterns that can be learned
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuccessPatternType {
    /// Effective sequences of tool calls
    ToolSequence,
    /// Prompt structures that lead to success
    PromptStructure,
    /// Optimal context sizes for different task types
    ContextSize,
    /// Model choices that work best for task types
    ModelChoice,
    /// Time-of-day patterns affecting success
    Timing,
}

impl SuccessPatternType {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SuccessPatternType::ToolSequence => "tool_sequence",
            SuccessPatternType::PromptStructure => "prompt_structure",
            SuccessPatternType::ContextSize => "context_size",
            SuccessPatternType::ModelChoice => "model_choice",
            SuccessPatternType::Timing => "timing",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "tool_sequence" => Ok(SuccessPatternType::ToolSequence),
            "prompt_structure" => Ok(SuccessPatternType::PromptStructure),
            "context_size" => Ok(SuccessPatternType::ContextSize),
            "model_choice" => Ok(SuccessPatternType::ModelChoice),
            "timing" => Ok(SuccessPatternType::Timing),
            _ => Err(crate::Error::Other(format!(
                "Unknown success pattern type: {}",
                s
            ))),
        }
    }
}

/// A pattern learned from successful agent runs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuccessPattern {
    /// Database ID
    pub id: i64,
    /// Type of success pattern
    pub pattern_type: SuccessPatternType,
    /// Agent type (None for patterns that apply to all types)
    pub agent_type: Option<AgentType>,
    /// Task type categorization
    pub task_type: Option<String>,
    /// Unique signature for deduplication
    pub pattern_signature: String,
    /// Pattern data (JSON)
    pub pattern_data: serde_json::Value,
    /// Number of times this pattern was observed
    pub occurrence_count: i64,
    /// Average completion time in milliseconds
    pub avg_completion_time_ms: Option<i64>,
    /// Average token usage
    pub avg_token_usage: Option<i64>,
    /// Success rate (always 1.0 for success patterns, but can track consistency)
    pub success_rate: f64,
    /// First observation timestamp
    pub first_seen_at: DateTime<Utc>,
    /// Last observation timestamp
    pub last_seen_at: DateTime<Utc>,
}

impl SuccessPattern {
    /// Create a new success pattern
    pub fn new(
        pattern_type: SuccessPatternType,
        pattern_signature: impl Into<String>,
        pattern_data: serde_json::Value,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            pattern_type,
            agent_type: None,
            task_type: None,
            pattern_signature: pattern_signature.into(),
            pattern_data,
            occurrence_count: 1,
            avg_completion_time_ms: None,
            avg_token_usage: None,
            success_rate: 1.0,
            first_seen_at: now,
            last_seen_at: now,
        }
    }

    /// Set agent type
    pub fn with_agent_type(mut self, agent_type: AgentType) -> Self {
        self.agent_type = Some(agent_type);
        self
    }

    /// Set task type
    pub fn with_task_type(mut self, task_type: Option<impl Into<String>>) -> Self {
        self.task_type = task_type.map(|t| t.into());
        self
    }

    /// Set completion time
    pub fn with_completion_time_ms(mut self, time_ms: i64) -> Self {
        self.avg_completion_time_ms = Some(time_ms);
        self
    }

    /// Set token usage
    pub fn with_token_usage(mut self, tokens: i64) -> Self {
        self.avg_token_usage = Some(tokens);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_scope_conversion() {
        assert_eq!(InstructionScope::Global.as_str(), "global");
        assert_eq!(InstructionScope::AgentType.as_str(), "agent_type");
        assert_eq!(
            InstructionScope::from_str("global").unwrap(),
            InstructionScope::Global
        );
        assert_eq!(
            InstructionScope::from_str("agent_type").unwrap(),
            InstructionScope::AgentType
        );
        assert!(InstructionScope::from_str("invalid").is_err());
    }

    #[test]
    fn test_instruction_source_conversion() {
        assert_eq!(InstructionSource::Manual.as_str(), "manual");
        assert_eq!(InstructionSource::Learned.as_str(), "learned");
        assert_eq!(InstructionSource::Imported.as_str(), "imported");
        assert_eq!(
            InstructionSource::from_str("manual").unwrap(),
            InstructionSource::Manual
        );
        assert!(InstructionSource::from_str("invalid").is_err());
    }

    #[test]
    fn test_custom_instruction_global() {
        let inst = CustomInstruction::global("test-rule", "Do something specific");

        assert_eq!(inst.name, "test-rule");
        assert_eq!(inst.content, "Do something specific");
        assert_eq!(inst.scope, InstructionScope::Global);
        assert!(inst.agent_type.is_none());
        assert!(inst.enabled);
        assert_eq!(inst.source, InstructionSource::Manual);
        assert_eq!(inst.confidence, 1.0);
    }

    #[test]
    fn test_custom_instruction_for_agent_type() {
        let inst = CustomInstruction::for_agent_type(
            "review-rule",
            "Check for security issues",
            AgentType::CodeReviewer,
        );

        assert_eq!(inst.scope, InstructionScope::AgentType);
        assert_eq!(inst.agent_type, Some(AgentType::CodeReviewer));
    }

    #[test]
    fn test_custom_instruction_learned() {
        let inst = CustomInstruction::learned("auto-rule", "Avoid this pattern", 0.75);

        assert_eq!(inst.source, InstructionSource::Learned);
        assert_eq!(inst.confidence, 0.75);
        assert!(!inst.enabled); // Learned instructions start disabled
    }

    #[test]
    fn test_instruction_applies_to() {
        let global = CustomInstruction::global("global", "content");
        let scoped =
            CustomInstruction::for_agent_type("scoped", "content", AgentType::CodeReviewer);
        let disabled = CustomInstruction::global("disabled", "content").disabled();

        assert!(global.applies_to(AgentType::StoryDeveloper));
        assert!(global.applies_to(AgentType::CodeReviewer));

        assert!(!scoped.applies_to(AgentType::StoryDeveloper));
        assert!(scoped.applies_to(AgentType::CodeReviewer));

        assert!(!disabled.applies_to(AgentType::StoryDeveloper));
    }

    #[test]
    fn test_instruction_builder_methods() {
        let inst = CustomInstruction::global("test", "content")
            .with_priority(50)
            .with_tags(vec!["security".to_string(), "validation".to_string()])
            .with_source(InstructionSource::Imported)
            .with_created_by("admin");

        assert_eq!(inst.priority, 50);
        assert_eq!(inst.tags, vec!["security", "validation"]);
        assert_eq!(inst.source, InstructionSource::Imported);
        assert_eq!(inst.created_by, Some("admin".to_string()));
    }

    #[test]
    fn test_instruction_effectiveness() {
        let mut eff = InstructionEffectiveness::new(1);

        assert_eq!(eff.usage_count, 0);
        assert_eq!(eff.success_rate, 0.0);
        assert!(!eff.should_disable());
        assert!(!eff.is_eligible_for_deletion());

        // Simulate some usage
        eff.usage_count = 10;
        eff.success_count = 8;
        eff.calculate_success_rate();
        assert_eq!(eff.success_rate, 0.8);

        // Test penalty thresholds
        eff.penalty_score = 0.5;
        assert!(!eff.should_disable());

        eff.penalty_score = 0.7;
        assert!(eff.should_disable());

        // Test deletion eligibility
        eff.penalty_score = 1.0;
        eff.usage_count = 10;
        eff.success_count = 2;
        eff.calculate_success_rate();
        assert!(eff.is_eligible_for_deletion());
    }

    #[test]
    fn test_pattern_type_conversion() {
        assert_eq!(PatternType::ErrorPattern.as_str(), "error_pattern");
        assert_eq!(PatternType::ToolUsagePattern.as_str(), "tool_usage_pattern");
        assert_eq!(
            PatternType::from_str("error_pattern").unwrap(),
            PatternType::ErrorPattern
        );
        assert!(PatternType::from_str("invalid").is_err());
    }

    #[test]
    fn test_pattern_status_conversion() {
        assert_eq!(PatternStatus::Observed.as_str(), "observed");
        assert_eq!(PatternStatus::PendingReview.as_str(), "pending_review");
        assert_eq!(PatternStatus::Approved.as_str(), "approved");
        assert_eq!(PatternStatus::Rejected.as_str(), "rejected");
        assert_eq!(
            PatternStatus::from_str("approved").unwrap(),
            PatternStatus::Approved
        );
        assert!(PatternStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_learning_pattern() {
        let pattern = LearningPattern::new(
            PatternType::ErrorPattern,
            "abc123",
            serde_json::json!({"error": "test error"}),
        )
        .with_agent_type(AgentType::StoryDeveloper);

        assert_eq!(pattern.pattern_type, PatternType::ErrorPattern);
        assert_eq!(pattern.pattern_signature, "abc123");
        assert_eq!(pattern.agent_type, Some(AgentType::StoryDeveloper));
        assert_eq!(pattern.occurrence_count, 1);
        assert_eq!(pattern.status, PatternStatus::Observed);
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
