//! Recovery Strategies
//!
//! Epic 016: Autonomous Epic Processing - Story 7
//!
//! Implements recovery actions for stuck or failed agents.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::stuck_detection::{StuckDetection, StuckSeverity, StuckType};
use crate::model_selection::ModelTier;

/// Recovery action types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryActionType {
    /// Pause the agent and alert for human intervention
    PauseAndAlert,
    /// Escalate to a more capable model
    ModelEscalation,
    /// Spawn a specialized fixer agent
    SpawnFixer,
    /// Fork context and retry with fresh session
    FreshRetry,
    /// Escalate to parent controller
    EscalateToParent,
    /// Simple retry with same configuration
    Retry,
    /// Wait for external condition (CI, review, etc.)
    Wait,
    /// Abort the task
    Abort,
}

impl RecoveryActionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PauseAndAlert => "pause_and_alert",
            Self::ModelEscalation => "model_escalation",
            Self::SpawnFixer => "spawn_fixer",
            Self::FreshRetry => "fresh_retry",
            Self::EscalateToParent => "escalate_to_parent",
            Self::Retry => "retry",
            Self::Wait => "wait",
            Self::Abort => "abort",
        }
    }
}

impl std::str::FromStr for RecoveryActionType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pause_and_alert" => Ok(Self::PauseAndAlert),
            "model_escalation" => Ok(Self::ModelEscalation),
            "spawn_fixer" => Ok(Self::SpawnFixer),
            "fresh_retry" => Ok(Self::FreshRetry),
            "escalate_to_parent" => Ok(Self::EscalateToParent),
            "retry" => Ok(Self::Retry),
            "wait" => Ok(Self::Wait),
            "abort" => Ok(Self::Abort),
            _ => Err(crate::Error::Other(format!(
                "Invalid recovery action type: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for RecoveryActionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Outcome of a recovery attempt
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryOutcome {
    /// Recovery succeeded
    Success,
    /// Recovery failed, may try another action
    Failed,
    /// Recovery is in progress
    InProgress,
    /// Recovery was cancelled
    Cancelled,
    /// Recovery was skipped (not applicable)
    Skipped,
}

impl RecoveryOutcome {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::InProgress => "in_progress",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }
}

impl std::str::FromStr for RecoveryOutcome {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "success" => Ok(Self::Success),
            "failed" => Ok(Self::Failed),
            "in_progress" => Ok(Self::InProgress),
            "cancelled" => Ok(Self::Cancelled),
            "skipped" => Ok(Self::Skipped),
            _ => Err(crate::Error::Other(format!(
                "Invalid recovery outcome: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for RecoveryOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A recovery attempt record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAttempt {
    pub id: i64,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub stuck_detection_id: Option<i64>,
    pub action_type: RecoveryActionType,
    pub outcome: RecoveryOutcome,
    pub details: serde_json::Value,
    pub attempt_number: u32,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
}

impl RecoveryAttempt {
    pub fn new(agent_id: impl Into<String>, action_type: RecoveryActionType) -> Self {
        Self {
            id: 0,
            agent_id: agent_id.into(),
            session_id: None,
            stuck_detection_id: None,
            action_type,
            outcome: RecoveryOutcome::InProgress,
            details: serde_json::json!({}),
            attempt_number: 1,
            started_at: Utc::now(),
            completed_at: None,
            error_message: None,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_detection(mut self, detection_id: i64) -> Self {
        self.stuck_detection_id = Some(detection_id);
        self
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }

    pub fn with_attempt_number(mut self, number: u32) -> Self {
        self.attempt_number = number;
        self
    }

    pub fn succeed(&mut self) {
        self.outcome = RecoveryOutcome::Success;
        self.completed_at = Some(Utc::now());
    }

    pub fn fail(&mut self, error: impl Into<String>) {
        self.outcome = RecoveryOutcome::Failed;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error.into());
    }

    pub fn skip(&mut self, reason: impl Into<String>) {
        self.outcome = RecoveryOutcome::Skipped;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(reason.into());
    }

    pub fn cancel(&mut self) {
        self.outcome = RecoveryOutcome::Cancelled;
        self.completed_at = Some(Utc::now());
    }
}

/// A planned recovery action with context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedRecoveryAction {
    pub action_type: RecoveryActionType,
    pub priority: u8,
    pub reason: String,
    pub details: serde_json::Value,
}

impl PlannedRecoveryAction {
    pub fn new(action_type: RecoveryActionType, priority: u8, reason: impl Into<String>) -> Self {
        Self {
            action_type,
            priority,
            reason: reason.into(),
            details: serde_json::json!({}),
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = details;
        self
    }
}

/// Configuration for recovery strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryConfig {
    /// Maximum retry attempts per action type
    pub max_retries: HashMap<RecoveryActionType, u32>,
    /// Maximum total recovery attempts per agent
    pub max_total_attempts: u32,
    /// Wait time in seconds before retry
    pub retry_delay_secs: u64,
    /// Enable automatic model escalation
    pub auto_model_escalation: bool,
    /// Enable spawning fixer agents
    pub enable_fixer_agents: bool,
    /// Stuck types that should pause for human intervention
    pub pause_for_human: Vec<StuckType>,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        let mut max_retries = HashMap::new();
        max_retries.insert(RecoveryActionType::Retry, 3);
        max_retries.insert(RecoveryActionType::ModelEscalation, 2);
        max_retries.insert(RecoveryActionType::SpawnFixer, 1);
        max_retries.insert(RecoveryActionType::FreshRetry, 1);
        max_retries.insert(RecoveryActionType::Wait, 5);

        Self {
            max_retries,
            max_total_attempts: 10,
            retry_delay_secs: 30,
            auto_model_escalation: true,
            enable_fixer_agents: true,
            pause_for_human: vec![
                StuckType::MergeConflict,
                StuckType::ContextLimit,
            ],
        }
    }
}

impl RecoveryConfig {
    /// Get max retries for an action type
    pub fn max_retries_for(&self, action_type: RecoveryActionType) -> u32 {
        self.max_retries.get(&action_type).copied().unwrap_or(1)
    }
}

/// Recovery strategy selector
#[derive(Debug, Clone)]
pub struct RecoverySelector {
    config: RecoveryConfig,
}

impl RecoverySelector {
    pub fn new() -> Self {
        Self {
            config: RecoveryConfig::default(),
        }
    }

    pub fn with_config(config: RecoveryConfig) -> Self {
        Self { config }
    }

    /// Select recovery actions for a stuck detection
    pub fn select_actions(
        &self,
        detection: &StuckDetection,
        current_model: ModelTier,
        attempt_counts: &HashMap<RecoveryActionType, u32>,
    ) -> Vec<PlannedRecoveryAction> {
        let mut actions = Vec::new();

        // Check if we should pause for human intervention
        if self.config.pause_for_human.contains(&detection.detection_type) {
            actions.push(PlannedRecoveryAction::new(
                RecoveryActionType::PauseAndAlert,
                100,
                format!("{} requires human intervention", detection.detection_type),
            ));
            return actions;
        }

        match detection.detection_type {
            StuckType::TurnLimit => {
                // Try model escalation first, then fresh retry
                if self.can_try(RecoveryActionType::ModelEscalation, attempt_counts)
                    && self.config.auto_model_escalation
                    && current_model.escalate().is_some()
                {
                    actions.push(
                        PlannedRecoveryAction::new(
                            RecoveryActionType::ModelEscalation,
                            80,
                            "Escalate to more capable model to complete work faster",
                        )
                        .with_details(serde_json::json!({
                            "current_model": current_model.as_str(),
                            "target_model": current_model.escalate().map(|m| m.as_str()),
                        })),
                    );
                }
                if self.can_try(RecoveryActionType::FreshRetry, attempt_counts) {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::FreshRetry,
                        60,
                        "Start fresh session with summarized context",
                    ));
                }
            }

            StuckType::NoProgress => {
                // Retry first, then model escalation, then fixer
                if self.can_try(RecoveryActionType::Retry, attempt_counts) {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::Retry,
                        70,
                        "Retry current task with nudge to make progress",
                    ));
                }
                if self.can_try(RecoveryActionType::ModelEscalation, attempt_counts)
                    && self.config.auto_model_escalation
                    && current_model.escalate().is_some()
                {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::ModelEscalation,
                        60,
                        "Escalate to smarter model",
                    ));
                }
                if self.can_try(RecoveryActionType::SpawnFixer, attempt_counts)
                    && self.config.enable_fixer_agents
                {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::SpawnFixer,
                        40,
                        "Spawn fixer agent to help unblock",
                    ));
                }
            }

            StuckType::CiTimeout => {
                // Wait first, then retry
                if self.can_try(RecoveryActionType::Wait, attempt_counts) {
                    actions.push(
                        PlannedRecoveryAction::new(
                            RecoveryActionType::Wait,
                            80,
                            "Wait for CI to complete",
                        )
                        .with_details(serde_json::json!({
                            "wait_minutes": 10,
                        })),
                    );
                }
                if self.can_try(RecoveryActionType::Retry, attempt_counts) {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::Retry,
                        50,
                        "Retry CI check",
                    ));
                }
            }

            StuckType::ReviewDelay => {
                // Wait for review, or escalate to parent
                if self.can_try(RecoveryActionType::Wait, attempt_counts) {
                    actions.push(
                        PlannedRecoveryAction::new(
                            RecoveryActionType::Wait,
                            80,
                            "Wait for code review",
                        )
                        .with_details(serde_json::json!({
                            "wait_minutes": 30,
                        })),
                    );
                }
                if self.can_try(RecoveryActionType::EscalateToParent, attempt_counts) {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::EscalateToParent,
                        60,
                        "Escalate to controller for review prioritization",
                    ));
                }
            }

            StuckType::MergeConflict => {
                // This should be in pause_for_human, but if not:
                actions.push(PlannedRecoveryAction::new(
                    RecoveryActionType::PauseAndAlert,
                    100,
                    "Merge conflict requires human intervention to resolve",
                ));
            }

            StuckType::RateLimit => {
                // Wait with backoff
                if self.can_try(RecoveryActionType::Wait, attempt_counts) {
                    let wait_minutes =
                        self.config.retry_delay_secs / 60 * (attempt_counts.get(&RecoveryActionType::Wait).unwrap_or(&0) + 1) as u64;
                    actions.push(
                        PlannedRecoveryAction::new(
                            RecoveryActionType::Wait,
                            90,
                            "Wait for rate limit to reset",
                        )
                        .with_details(serde_json::json!({
                            "wait_minutes": wait_minutes.min(60),
                        })),
                    );
                }
            }

            StuckType::ContextLimit => {
                // Fresh retry with summarized context
                if self.can_try(RecoveryActionType::FreshRetry, attempt_counts) {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::FreshRetry,
                        90,
                        "Start fresh session with summarized context",
                    ));
                }
                // Also consider pause if fresh retry has been tried
                if !self.can_try(RecoveryActionType::FreshRetry, attempt_counts) {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::PauseAndAlert,
                        100,
                        "Context limit exceeded after retry, needs human intervention",
                    ));
                }
            }

            StuckType::ErrorLoop => {
                // Model escalation first, then fixer, then abort
                if self.can_try(RecoveryActionType::ModelEscalation, attempt_counts)
                    && self.config.auto_model_escalation
                    && current_model.escalate().is_some()
                {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::ModelEscalation,
                        80,
                        "Escalate to smarter model to break error loop",
                    ));
                }
                if self.can_try(RecoveryActionType::SpawnFixer, attempt_counts)
                    && self.config.enable_fixer_agents
                {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::SpawnFixer,
                        60,
                        "Spawn specialized fixer agent",
                    ));
                }
                if !self.can_try(RecoveryActionType::ModelEscalation, attempt_counts)
                    && !self.can_try(RecoveryActionType::SpawnFixer, attempt_counts)
                {
                    actions.push(PlannedRecoveryAction::new(
                        RecoveryActionType::Abort,
                        100,
                        "Error loop unrecoverable, aborting task",
                    ));
                }
            }
        }

        // If severity is critical and no actions yet, escalate to parent
        if detection.severity == StuckSeverity::Critical && actions.is_empty() {
            actions.push(PlannedRecoveryAction::new(
                RecoveryActionType::EscalateToParent,
                100,
                "Critical issue with no automated recovery options",
            ));
        }

        // Sort by priority (highest first)
        actions.sort_by(|a, b| b.priority.cmp(&a.priority));

        actions
    }

    /// Check if we can try a recovery action
    fn can_try(
        &self,
        action_type: RecoveryActionType,
        attempt_counts: &HashMap<RecoveryActionType, u32>,
    ) -> bool {
        let max = self.config.max_retries_for(action_type);
        let current = attempt_counts.get(&action_type).copied().unwrap_or(0);
        current < max
    }

    /// Get the next action to try from a list
    pub fn next_action<'a>(&self, actions: &'a [PlannedRecoveryAction]) -> Option<&'a PlannedRecoveryAction> {
        actions.first()
    }
}

impl Default for RecoverySelector {
    fn default() -> Self {
        Self::new()
    }
}

/// Fixer agent types for specialized recovery
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FixerAgentType {
    /// Fix test failures
    TestFixer,
    /// Fix linting issues
    LintFixer,
    /// Fix build errors
    BuildFixer,
    /// Fix security issues
    SecurityFixer,
    /// General debugging
    Debugger,
}

impl FixerAgentType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TestFixer => "test_fixer",
            Self::LintFixer => "lint_fixer",
            Self::BuildFixer => "build_fixer",
            Self::SecurityFixer => "security_fixer",
            Self::Debugger => "debugger",
        }
    }

    /// Get the agent prompt name for this fixer type
    pub fn prompt_name(&self) -> &'static str {
        match self {
            Self::TestFixer => "test-fixer",
            Self::LintFixer => "lint-fixer",
            Self::BuildFixer => "build-fixer",
            Self::SecurityFixer => "security-fixer",
            Self::Debugger => "debugger",
        }
    }
}

impl std::str::FromStr for FixerAgentType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "test_fixer" => Ok(Self::TestFixer),
            "lint_fixer" => Ok(Self::LintFixer),
            "build_fixer" => Ok(Self::BuildFixer),
            "security_fixer" => Ok(Self::SecurityFixer),
            "debugger" => Ok(Self::Debugger),
            _ => Err(crate::Error::Other(format!(
                "Invalid fixer agent type: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for FixerAgentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Request to spawn a fixer agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixerRequest {
    pub fixer_type: FixerAgentType,
    pub parent_agent_id: String,
    pub issue_description: String,
    pub context: serde_json::Value,
    pub files_involved: Vec<String>,
}

impl FixerRequest {
    pub fn new(
        fixer_type: FixerAgentType,
        parent_agent_id: impl Into<String>,
        issue_description: impl Into<String>,
    ) -> Self {
        Self {
            fixer_type,
            parent_agent_id: parent_agent_id.into(),
            issue_description: issue_description.into(),
            context: serde_json::json!({}),
            files_involved: Vec::new(),
        }
    }

    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = context;
        self
    }

    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files_involved = files;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recovery_action_type_roundtrip() {
        let types = [
            RecoveryActionType::PauseAndAlert,
            RecoveryActionType::ModelEscalation,
            RecoveryActionType::SpawnFixer,
            RecoveryActionType::FreshRetry,
            RecoveryActionType::EscalateToParent,
            RecoveryActionType::Retry,
            RecoveryActionType::Wait,
            RecoveryActionType::Abort,
        ];

        for t in types {
            let s = t.as_str();
            let parsed: RecoveryActionType = s.parse().unwrap();
            assert_eq!(t, parsed);
        }
    }

    #[test]
    fn test_recovery_outcome_roundtrip() {
        let outcomes = [
            RecoveryOutcome::Success,
            RecoveryOutcome::Failed,
            RecoveryOutcome::InProgress,
            RecoveryOutcome::Cancelled,
            RecoveryOutcome::Skipped,
        ];

        for o in outcomes {
            let s = o.as_str();
            let parsed: RecoveryOutcome = s.parse().unwrap();
            assert_eq!(o, parsed);
        }
    }

    #[test]
    fn test_recovery_attempt_new() {
        let attempt = RecoveryAttempt::new("agent-1", RecoveryActionType::Retry);
        assert_eq!(attempt.agent_id, "agent-1");
        assert_eq!(attempt.action_type, RecoveryActionType::Retry);
        assert_eq!(attempt.outcome, RecoveryOutcome::InProgress);
        assert_eq!(attempt.attempt_number, 1);
    }

    #[test]
    fn test_recovery_attempt_succeed() {
        let mut attempt = RecoveryAttempt::new("agent-1", RecoveryActionType::Retry);
        attempt.succeed();
        assert_eq!(attempt.outcome, RecoveryOutcome::Success);
        assert!(attempt.completed_at.is_some());
    }

    #[test]
    fn test_recovery_attempt_fail() {
        let mut attempt = RecoveryAttempt::new("agent-1", RecoveryActionType::Retry);
        attempt.fail("Something went wrong");
        assert_eq!(attempt.outcome, RecoveryOutcome::Failed);
        assert_eq!(attempt.error_message, Some("Something went wrong".to_string()));
        assert!(attempt.completed_at.is_some());
    }

    #[test]
    fn test_recovery_attempt_skip() {
        let mut attempt = RecoveryAttempt::new("agent-1", RecoveryActionType::SpawnFixer);
        attempt.skip("Fixer agents disabled");
        assert_eq!(attempt.outcome, RecoveryOutcome::Skipped);
        assert_eq!(attempt.error_message, Some("Fixer agents disabled".to_string()));
    }

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();
        assert_eq!(config.max_retries_for(RecoveryActionType::Retry), 3);
        assert_eq!(config.max_retries_for(RecoveryActionType::ModelEscalation), 2);
        assert!(config.auto_model_escalation);
        assert!(config.enable_fixer_agents);
    }

    #[test]
    fn test_selector_turn_limit_recovery() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::TurnLimit, StuckSeverity::High);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Balanced, &attempt_counts);
        assert!(!actions.is_empty());

        // Should recommend model escalation first (highest priority)
        assert_eq!(actions[0].action_type, RecoveryActionType::ModelEscalation);
    }

    #[test]
    fn test_selector_no_progress_recovery() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::NoProgress, StuckSeverity::Medium);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Smart, &attempt_counts);
        assert!(!actions.is_empty());

        // Should have multiple options
        assert!(actions.len() >= 2);
    }

    #[test]
    fn test_selector_ci_timeout_recovery() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::CiTimeout, StuckSeverity::Medium);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Smart, &attempt_counts);
        assert!(!actions.is_empty());

        // Should recommend wait first
        assert_eq!(actions[0].action_type, RecoveryActionType::Wait);
    }

    #[test]
    fn test_selector_merge_conflict_pauses() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::MergeConflict, StuckSeverity::High);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Smart, &attempt_counts);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, RecoveryActionType::PauseAndAlert);
    }

    #[test]
    fn test_selector_rate_limit_wait() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::RateLimit, StuckSeverity::Medium);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Smart, &attempt_counts);
        assert!(!actions.is_empty());
        assert_eq!(actions[0].action_type, RecoveryActionType::Wait);
    }

    #[test]
    fn test_selector_context_limit_fresh_retry() {
        // Use config without ContextLimit in pause_for_human
        let config = RecoveryConfig {
            pause_for_human: vec![StuckType::MergeConflict], // Only merge conflict
            ..Default::default()
        };
        let selector = RecoverySelector::with_config(config);
        let detection = StuckDetection::new("agent-1", StuckType::ContextLimit, StuckSeverity::High);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Smart, &attempt_counts);
        assert!(!actions.is_empty());
        assert_eq!(actions[0].action_type, RecoveryActionType::FreshRetry);
    }

    #[test]
    fn test_selector_context_limit_after_retry_pauses() {
        // Use config without ContextLimit in pause_for_human
        let config = RecoveryConfig {
            pause_for_human: vec![StuckType::MergeConflict],
            ..Default::default()
        };
        let selector = RecoverySelector::with_config(config);
        let detection = StuckDetection::new("agent-1", StuckType::ContextLimit, StuckSeverity::High);

        // Fresh retry already exhausted
        let mut attempt_counts = HashMap::new();
        attempt_counts.insert(RecoveryActionType::FreshRetry, 1);

        let actions = selector.select_actions(&detection, ModelTier::Smart, &attempt_counts);
        assert!(!actions.is_empty());
        assert_eq!(actions[0].action_type, RecoveryActionType::PauseAndAlert);
    }

    #[test]
    fn test_selector_error_loop_recovery() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::ErrorLoop, StuckSeverity::High);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Balanced, &attempt_counts);
        assert!(!actions.is_empty());

        // Should try model escalation first
        assert_eq!(actions[0].action_type, RecoveryActionType::ModelEscalation);
    }

    #[test]
    fn test_selector_error_loop_abort_when_exhausted() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::ErrorLoop, StuckSeverity::High);

        // All options exhausted
        let mut attempt_counts = HashMap::new();
        attempt_counts.insert(RecoveryActionType::ModelEscalation, 2);
        attempt_counts.insert(RecoveryActionType::SpawnFixer, 1);

        let actions = selector.select_actions(&detection, ModelTier::Premium, &attempt_counts);
        assert!(!actions.is_empty());
        assert_eq!(actions[0].action_type, RecoveryActionType::Abort);
    }

    #[test]
    fn test_selector_at_premium_no_escalation() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::TurnLimit, StuckSeverity::High);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Premium, &attempt_counts);

        // Should not include model escalation when already at Premium
        let has_escalation = actions
            .iter()
            .any(|a| a.action_type == RecoveryActionType::ModelEscalation);
        assert!(!has_escalation);
    }

    #[test]
    fn test_selector_respects_max_retries() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::NoProgress, StuckSeverity::Medium);

        // Retry already at max
        let mut attempt_counts = HashMap::new();
        attempt_counts.insert(RecoveryActionType::Retry, 3);

        let actions = selector.select_actions(&detection, ModelTier::Smart, &attempt_counts);

        // Should not include retry
        let has_retry = actions.iter().any(|a| a.action_type == RecoveryActionType::Retry);
        assert!(!has_retry);
    }

    #[test]
    fn test_fixer_agent_type_roundtrip() {
        let types = [
            FixerAgentType::TestFixer,
            FixerAgentType::LintFixer,
            FixerAgentType::BuildFixer,
            FixerAgentType::SecurityFixer,
            FixerAgentType::Debugger,
        ];

        for t in types {
            let s = t.as_str();
            let parsed: FixerAgentType = s.parse().unwrap();
            assert_eq!(t, parsed);
        }
    }

    #[test]
    fn test_fixer_request_new() {
        let request = FixerRequest::new(
            FixerAgentType::TestFixer,
            "agent-parent",
            "Tests are failing in module X",
        )
        .with_files(vec!["src/lib.rs".to_string(), "src/tests.rs".to_string()]);

        assert_eq!(request.fixer_type, FixerAgentType::TestFixer);
        assert_eq!(request.parent_agent_id, "agent-parent");
        assert_eq!(request.files_involved.len(), 2);
    }

    #[test]
    fn test_recovery_action_priority_sorting() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::NoProgress, StuckSeverity::Medium);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Balanced, &attempt_counts);

        // Verify sorted by priority (descending)
        for i in 0..actions.len().saturating_sub(1) {
            assert!(actions[i].priority >= actions[i + 1].priority);
        }
    }

    #[test]
    fn test_custom_config_pause_for_human() {
        let mut config = RecoveryConfig::default();
        config.pause_for_human.push(StuckType::ErrorLoop);

        let selector = RecoverySelector::with_config(config);
        let detection = StuckDetection::new("agent-1", StuckType::ErrorLoop, StuckSeverity::High);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Smart, &attempt_counts);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].action_type, RecoveryActionType::PauseAndAlert);
    }

    #[test]
    fn test_custom_config_disable_fixer_agents() {
        let mut config = RecoveryConfig::default();
        config.enable_fixer_agents = false;

        let selector = RecoverySelector::with_config(config);
        let detection = StuckDetection::new("agent-1", StuckType::NoProgress, StuckSeverity::Medium);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Smart, &attempt_counts);

        // Should not include spawn fixer
        let has_fixer = actions
            .iter()
            .any(|a| a.action_type == RecoveryActionType::SpawnFixer);
        assert!(!has_fixer);
    }

    #[test]
    fn test_next_action() {
        let selector = RecoverySelector::new();
        let detection = StuckDetection::new("agent-1", StuckType::TurnLimit, StuckSeverity::High);
        let attempt_counts = HashMap::new();

        let actions = selector.select_actions(&detection, ModelTier::Balanced, &attempt_counts);
        let next = selector.next_action(&actions);

        assert!(next.is_some());
        assert_eq!(next.unwrap().action_type, RecoveryActionType::ModelEscalation);
    }

    #[test]
    fn test_next_action_empty() {
        let selector = RecoverySelector::new();
        let actions: Vec<PlannedRecoveryAction> = Vec::new();

        let next = selector.next_action(&actions);
        assert!(next.is_none());
    }
}
