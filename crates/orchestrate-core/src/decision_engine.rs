//! Decision Engine Core
//!
//! The decision engine determines what action to take next in the autonomous
//! workflow by evaluating agent output, checking completion criteria, and
//! detecting when reviews or escalation are needed.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Status signals that agents can emit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AgentStatus {
    /// Agent completed its task successfully
    Complete,
    /// Agent is blocked and needs intervention
    Blocked,
    /// Agent is waiting for external event (CI, review, etc.)
    Waiting,
    /// Agent needs more input/clarification
    NeedsInput,
    /// Agent encountered an error
    Error,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "COMPLETE",
            Self::Blocked => "BLOCKED",
            Self::Waiting => "WAITING",
            Self::NeedsInput => "NEEDS_INPUT",
            Self::Error => "ERROR",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "COMPLETE" | "COMPLETED" | "DONE" => Some(Self::Complete),
            "BLOCKED" | "STUCK" => Some(Self::Blocked),
            "WAITING" | "WAIT" | "PENDING" => Some(Self::Waiting),
            "NEEDS_INPUT" | "NEEDSINPUT" | "INPUT_NEEDED" => Some(Self::NeedsInput),
            "ERROR" | "FAILED" | "FAILURE" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Parsed status signal from agent output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusSignal {
    /// The status type
    pub status: AgentStatus,
    /// Optional reason or message
    pub reason: Option<String>,
    /// Additional context/details
    pub details: Option<serde_json::Value>,
    /// When the signal was parsed
    pub parsed_at: DateTime<Utc>,
}

/// Types of decisions the engine can make
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Decision {
    /// Spawn a new agent with specific task
    SpawnAgent {
        agent_type: String,
        task: String,
        context: Option<serde_json::Value>,
    },
    /// Continue existing agent with new input
    ContinueAgent {
        agent_id: String,
        message: String,
        context: Option<serde_json::Value>,
    },
    /// Trigger a code review
    TriggerReview {
        files_changed: Vec<String>,
        review_type: ReviewType,
    },
    /// Mark work as complete
    CompleteWork {
        work_item_id: String,
        summary: Option<String>,
    },
    /// Escalate for human intervention
    Escalate {
        reason: String,
        severity: EscalationSeverity,
        context: Option<serde_json::Value>,
    },
    /// Wait for external event
    Wait {
        wait_type: WaitType,
        timeout_seconds: Option<u32>,
    },
    /// Retry the current task
    Retry {
        reason: String,
        modified_context: Option<serde_json::Value>,
    },
    /// Transition to next workflow state
    TransitionState {
        new_state: String,
    },
}

/// Types of code review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewType {
    /// Full code review
    Full,
    /// Quick sanity check
    Quick,
    /// Security-focused review
    Security,
    /// Review for specific issue
    Targeted,
}

/// Severity levels for escalation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EscalationSeverity {
    /// Informational, can continue
    Low,
    /// Needs attention soon
    Medium,
    /// Needs immediate attention
    High,
    /// Critical blocker
    Critical,
}

/// Types of waits
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WaitType {
    /// Waiting for CI to complete
    CiCompletion { run_id: Option<String> },
    /// Waiting for PR review
    PrReview { pr_number: Option<i32> },
    /// Waiting for approval
    Approval { request_id: Option<String> },
    /// Waiting for external service
    ExternalService { service: String },
    /// Generic wait with timeout
    Timeout,
}

/// Result of evaluating agent output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationResult {
    /// Parsed status signal (if found)
    pub status_signal: Option<StatusSignal>,
    /// List of files changed (detected from output)
    pub files_changed: Vec<String>,
    /// Tests added or modified
    pub tests_affected: Vec<String>,
    /// Acceptance criteria that appear met
    pub criteria_met: Vec<String>,
    /// Acceptance criteria that appear incomplete
    pub criteria_incomplete: Vec<String>,
    /// Whether code review is recommended
    pub needs_review: bool,
    /// Recommended decision based on evaluation
    pub recommended_decision: Option<Decision>,
    /// Raw output that was evaluated
    pub raw_output: String,
    /// When evaluation was performed
    pub evaluated_at: DateTime<Utc>,
}

/// Configuration for the decision engine
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionEngineConfig {
    /// Minimum number of files changed to trigger review
    pub review_file_threshold: usize,
    /// File patterns that always require review
    pub always_review_patterns: Vec<String>,
    /// Maximum retries before escalation
    pub max_retries: u32,
    /// Whether to auto-escalate on certain errors
    pub auto_escalate_on_error: bool,
}

impl Default for DecisionEngineConfig {
    fn default() -> Self {
        Self {
            review_file_threshold: 1,
            always_review_patterns: vec![
                "Cargo.toml".to_string(),
                "package.json".to_string(),
                ".github/".to_string(),
                "migrations/".to_string(),
            ],
            max_retries: 3,
            auto_escalate_on_error: true,
        }
    }
}

/// The Decision Engine
///
/// Evaluates agent output and determines the next action to take in the
/// autonomous workflow.
#[derive(Debug, Clone)]
pub struct DecisionEngine {
    config: DecisionEngineConfig,
}

impl DecisionEngine {
    /// Create a new decision engine with default config
    pub fn new() -> Self {
        Self {
            config: DecisionEngineConfig::default(),
        }
    }

    /// Create a new decision engine with custom config
    pub fn with_config(config: DecisionEngineConfig) -> Self {
        Self { config }
    }

    /// Evaluate agent output and return evaluation result
    pub fn evaluate_agent_output(&self, output: &str) -> EvaluationResult {
        let status_signal = self.parse_status_signal(output);
        let files_changed = self.detect_files_changed(output);
        let tests_affected = self.detect_tests_affected(output);
        let needs_review = self.check_needs_review(output, &files_changed);

        let recommended_decision = self.determine_decision(
            status_signal.as_ref(),
            &files_changed,
            needs_review,
        );

        EvaluationResult {
            status_signal,
            files_changed,
            tests_affected,
            criteria_met: Vec::new(),     // To be filled by checking against story
            criteria_incomplete: Vec::new(), // To be filled by checking against story
            needs_review,
            recommended_decision,
            raw_output: output.to_string(),
            evaluated_at: Utc::now(),
        }
    }

    /// Parse STATUS signal from agent output
    pub fn parse_status_signal(&self, output: &str) -> Option<StatusSignal> {
        // Look for STATUS: patterns in the output
        // Common formats:
        // STATUS: COMPLETE
        // STATUS: BLOCKED - reason here
        // **STATUS**: WAITING

        let status_patterns = [
            r"(?i)STATUS:\s*(\w+)(?:\s*[-:]\s*(.*))?",
            r"(?i)\*\*STATUS\*\*:\s*(\w+)(?:\s*[-:]\s*(.*))?",
            r"(?i)\[STATUS\]:\s*(\w+)(?:\s*[-:]\s*(.*))?",
        ];

        for pattern in &status_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(output) {
                    if let Some(status_match) = captures.get(1) {
                        if let Some(status) = AgentStatus::from_str(status_match.as_str()) {
                            let reason = captures.get(2).map(|m| m.as_str().trim().to_string());
                            return Some(StatusSignal {
                                status,
                                reason: if reason.as_ref().map(|r| r.is_empty()).unwrap_or(true) {
                                    None
                                } else {
                                    reason
                                },
                                details: None,
                                parsed_at: Utc::now(),
                            });
                        }
                    }
                }
            }
        }

        // Also check for structured JSON status
        if let Some(json_signal) = self.parse_json_status(output) {
            return Some(json_signal);
        }

        None
    }

    /// Parse JSON-formatted status signal
    fn parse_json_status(&self, output: &str) -> Option<StatusSignal> {
        // Look for JSON blocks that might contain status
        let json_pattern = r#"```json\s*\{[^}]*"status"\s*:\s*"(\w+)"[^}]*\}\s*```"#;

        if let Ok(re) = regex::Regex::new(json_pattern) {
            if let Some(captures) = re.captures(output) {
                if let Some(status_match) = captures.get(1) {
                    if let Some(status) = AgentStatus::from_str(status_match.as_str()) {
                        return Some(StatusSignal {
                            status,
                            reason: None,
                            details: None,
                            parsed_at: Utc::now(),
                        });
                    }
                }
            }
        }

        None
    }

    /// Detect files changed from agent output
    pub fn detect_files_changed(&self, output: &str) -> Vec<String> {
        let mut files = Vec::new();

        // Look for common patterns indicating file changes
        let patterns = [
            r"(?:Created|Modified|Updated|Wrote|Edited|Changed)\s+(?:file\s+)?[`']?([^\s`']+\.\w+)[`']?",
            r"(?:Write|Edit)\s+tool.*?[`']([^\s`']+\.\w+)[`']",
            r"git\s+(?:add|diff)\s+[`']?([^\s`']+\.\w+)[`']?",
            r"File:\s+[`']?([^\s`']+\.\w+)[`']?",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for captures in re.captures_iter(output) {
                    if let Some(file_match) = captures.get(1) {
                        let file = file_match.as_str().to_string();
                        if !files.contains(&file) && self.is_valid_file_path(&file) {
                            files.push(file);
                        }
                    }
                }
            }
        }

        files
    }

    /// Check if a string looks like a valid file path
    fn is_valid_file_path(&self, path: &str) -> bool {
        // Basic validation - has extension and reasonable characters
        if path.is_empty() || path.len() > 500 {
            return false;
        }

        // Should contain a dot for extension
        if !path.contains('.') {
            return false;
        }

        // Should not contain certain characters
        !path.contains("```") && !path.contains("  ")
    }

    /// Detect tests that were added or modified
    pub fn detect_tests_affected(&self, output: &str) -> Vec<String> {
        let mut tests = Vec::new();

        let patterns = [
            r"(?:#\[test\]|#\[tokio::test\])\s*(?:async\s+)?fn\s+(\w+)",
            r"test\s+(\w+)\s+\.\.\.\s+(?:ok|FAILED)",
            r"running\s+\d+\s+tests?.*?test\s+(\w+)",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for captures in re.captures_iter(output) {
                    if let Some(test_match) = captures.get(1) {
                        let test = test_match.as_str().to_string();
                        if !tests.contains(&test) {
                            tests.push(test);
                        }
                    }
                }
            }
        }

        tests
    }

    /// Check if the changes require code review
    pub fn check_needs_review(&self, output: &str, files_changed: &[String]) -> bool {
        // Review needed if enough files changed
        if files_changed.len() >= self.config.review_file_threshold {
            return true;
        }

        // Review needed if certain sensitive files changed
        for file in files_changed {
            for pattern in &self.config.always_review_patterns {
                if file.contains(pattern) || file.starts_with(pattern) {
                    return true;
                }
            }
        }

        // Review needed if output mentions significant changes
        let review_indicators = [
            "breaking change",
            "api change",
            "security",
            "authentication",
            "authorization",
            "database migration",
            "schema change",
        ];

        let output_lower = output.to_lowercase();
        for indicator in &review_indicators {
            if output_lower.contains(indicator) {
                return true;
            }
        }

        false
    }

    /// Check acceptance criteria completion against story definition
    pub fn check_acceptance_criteria(
        &self,
        criteria: &[String],
        agent_output: &str,
    ) -> (Vec<String>, Vec<String>) {
        let mut met = Vec::new();
        let mut incomplete = Vec::new();
        let output_lower = agent_output.to_lowercase();

        for criterion in criteria {
            // Extract key terms from criterion
            let criterion_lower = criterion.to_lowercase();
            let key_terms: Vec<&str> = criterion_lower
                .split_whitespace()
                .filter(|w| w.len() > 3)
                .collect();

            // Check if most key terms appear in output
            let matched_terms = key_terms
                .iter()
                .filter(|term| output_lower.contains(*term))
                .count();

            // Consider criterion met if at least 50% of key terms are found
            // and we see positive indicators
            let has_positive_indicator = output_lower.contains("implemented")
                || output_lower.contains("completed")
                || output_lower.contains("added")
                || output_lower.contains("created")
                || output_lower.contains("test")
                || output_lower.contains("pass");

            if !key_terms.is_empty()
                && matched_terms as f64 / key_terms.len() as f64 >= 0.5
                && has_positive_indicator
            {
                met.push(criterion.clone());
            } else {
                incomplete.push(criterion.clone());
            }
        }

        (met, incomplete)
    }

    /// Determine the recommended decision based on evaluation
    fn determine_decision(
        &self,
        status_signal: Option<&StatusSignal>,
        files_changed: &[String],
        needs_review: bool,
    ) -> Option<Decision> {
        // If we have a clear status signal, use it
        if let Some(signal) = status_signal {
            match signal.status {
                AgentStatus::Complete => {
                    if needs_review && !files_changed.is_empty() {
                        return Some(Decision::TriggerReview {
                            files_changed: files_changed.to_vec(),
                            review_type: ReviewType::Full,
                        });
                    }
                    return Some(Decision::CompleteWork {
                        work_item_id: String::new(), // To be filled by caller
                        summary: signal.reason.clone(),
                    });
                }
                AgentStatus::Blocked => {
                    return Some(Decision::Escalate {
                        reason: signal
                            .reason
                            .clone()
                            .unwrap_or_else(|| "Agent blocked".to_string()),
                        severity: EscalationSeverity::Medium,
                        context: signal.details.clone(),
                    });
                }
                AgentStatus::Waiting => {
                    return Some(Decision::Wait {
                        wait_type: WaitType::Timeout,
                        timeout_seconds: Some(300), // 5 minutes default
                    });
                }
                AgentStatus::NeedsInput => {
                    return Some(Decision::Escalate {
                        reason: signal
                            .reason
                            .clone()
                            .unwrap_or_else(|| "Agent needs clarification".to_string()),
                        severity: EscalationSeverity::Low,
                        context: signal.details.clone(),
                    });
                }
                AgentStatus::Error => {
                    if self.config.auto_escalate_on_error {
                        return Some(Decision::Escalate {
                            reason: signal
                                .reason
                                .clone()
                                .unwrap_or_else(|| "Agent encountered error".to_string()),
                            severity: EscalationSeverity::High,
                            context: signal.details.clone(),
                        });
                    }
                    return Some(Decision::Retry {
                        reason: signal
                            .reason
                            .clone()
                            .unwrap_or_else(|| "Agent error".to_string()),
                        modified_context: None,
                    });
                }
            }
        }

        // No clear status - check if we should trigger review
        if needs_review && !files_changed.is_empty() {
            return Some(Decision::TriggerReview {
                files_changed: files_changed.to_vec(),
                review_type: ReviewType::Full,
            });
        }

        // No decision can be made
        None
    }

    /// Make a decision based on full context
    pub fn make_decision(
        &self,
        evaluation: &EvaluationResult,
        current_state: &str,
        retry_count: u32,
    ) -> Decision {
        // Check for max retries
        if retry_count >= self.config.max_retries {
            return Decision::Escalate {
                reason: format!(
                    "Maximum retries ({}) exceeded",
                    self.config.max_retries
                ),
                severity: EscalationSeverity::High,
                context: Some(serde_json::json!({
                    "retry_count": retry_count,
                    "last_status": evaluation.status_signal.as_ref().map(|s| s.status.as_str()),
                })),
            };
        }

        // Use the recommended decision if available
        if let Some(decision) = &evaluation.recommended_decision {
            return decision.clone();
        }

        // Default to state transition based on current state
        let next_state = match current_state {
            "idle" => "analyzing",
            "analyzing" => "discovering",
            "discovering" => "planning",
            "planning" => "executing",
            "executing" => {
                if evaluation.needs_review {
                    "reviewing"
                } else {
                    "pr_creation"
                }
            }
            "reviewing" => "pr_creation",
            "pr_creation" => "pr_monitoring",
            "pr_monitoring" => "pr_merging",
            "pr_merging" => "completing",
            "completing" => "done",
            _ => "done",
        };

        Decision::TransitionState {
            new_state: next_state.to_string(),
        }
    }
}

impl Default for DecisionEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== AgentStatus Tests ====================

    #[test]
    fn test_agent_status_as_str() {
        assert_eq!(AgentStatus::Complete.as_str(), "COMPLETE");
        assert_eq!(AgentStatus::Blocked.as_str(), "BLOCKED");
        assert_eq!(AgentStatus::Waiting.as_str(), "WAITING");
        assert_eq!(AgentStatus::NeedsInput.as_str(), "NEEDS_INPUT");
        assert_eq!(AgentStatus::Error.as_str(), "ERROR");
    }

    #[test]
    fn test_agent_status_from_str() {
        assert_eq!(AgentStatus::from_str("COMPLETE"), Some(AgentStatus::Complete));
        assert_eq!(AgentStatus::from_str("complete"), Some(AgentStatus::Complete));
        assert_eq!(AgentStatus::from_str("DONE"), Some(AgentStatus::Complete));
        assert_eq!(AgentStatus::from_str("BLOCKED"), Some(AgentStatus::Blocked));
        assert_eq!(AgentStatus::from_str("stuck"), Some(AgentStatus::Blocked));
        assert_eq!(AgentStatus::from_str("WAITING"), Some(AgentStatus::Waiting));
        assert_eq!(AgentStatus::from_str("ERROR"), Some(AgentStatus::Error));
        assert_eq!(AgentStatus::from_str("FAILED"), Some(AgentStatus::Error));
        assert_eq!(AgentStatus::from_str("unknown"), None);
    }

    // ==================== Status Signal Parsing Tests ====================

    #[test]
    fn test_parse_status_complete() {
        let engine = DecisionEngine::new();

        let output = "I have completed the task.\n\nSTATUS: COMPLETE";
        let signal = engine.parse_status_signal(output).unwrap();
        assert_eq!(signal.status, AgentStatus::Complete);
        assert!(signal.reason.is_none());
    }

    #[test]
    fn test_parse_status_with_reason() {
        let engine = DecisionEngine::new();

        let output = "STATUS: BLOCKED - Missing API credentials";
        let signal = engine.parse_status_signal(output).unwrap();
        assert_eq!(signal.status, AgentStatus::Blocked);
        assert_eq!(signal.reason, Some("Missing API credentials".to_string()));
    }

    #[test]
    fn test_parse_status_markdown_format() {
        let engine = DecisionEngine::new();

        let output = "Work done!\n\n**STATUS**: COMPLETE";
        let signal = engine.parse_status_signal(output).unwrap();
        assert_eq!(signal.status, AgentStatus::Complete);
    }

    #[test]
    fn test_parse_status_waiting() {
        let engine = DecisionEngine::new();

        let output = "STATUS: WAITING - CI pipeline running";
        let signal = engine.parse_status_signal(output).unwrap();
        assert_eq!(signal.status, AgentStatus::Waiting);
    }

    #[test]
    fn test_parse_no_status() {
        let engine = DecisionEngine::new();

        let output = "I made some changes to the code.";
        let signal = engine.parse_status_signal(output);
        assert!(signal.is_none());
    }

    // ==================== File Detection Tests ====================

    #[test]
    fn test_detect_files_changed() {
        let engine = DecisionEngine::new();

        let output = r#"
I have made the following changes:
- Created file `src/lib.rs`
- Modified `Cargo.toml`
- Updated src/main.rs
        "#;

        let files = engine.detect_files_changed(output);
        assert!(files.contains(&"src/lib.rs".to_string()));
        assert!(files.contains(&"Cargo.toml".to_string()));
        assert!(files.contains(&"src/main.rs".to_string()));
    }

    #[test]
    fn test_detect_tests_affected() {
        let engine = DecisionEngine::new();

        let output = r#"
Running tests:
test test_create_session ... ok
test test_update_session ... ok
test test_delete_session ... FAILED

Added new test:
#[test]
fn test_new_feature() {
    // test implementation
}
        "#;

        let tests = engine.detect_tests_affected(output);
        assert!(tests.contains(&"test_create_session".to_string()));
        assert!(tests.contains(&"test_update_session".to_string()));
        assert!(tests.contains(&"test_delete_session".to_string()));
    }

    // ==================== Review Detection Tests ====================

    #[test]
    fn test_needs_review_file_threshold() {
        let engine = DecisionEngine::new();

        let output = "Made changes to the code";
        let files = vec!["src/lib.rs".to_string()];

        assert!(engine.check_needs_review(output, &files));
    }

    #[test]
    fn test_needs_review_sensitive_files() {
        let engine = DecisionEngine::new();

        let output = "Updated dependencies";
        let files = vec!["Cargo.toml".to_string()];

        assert!(engine.check_needs_review(output, &files));
    }

    #[test]
    fn test_needs_review_security_mention() {
        let engine = DecisionEngine::new();

        let output = "Fixed a security vulnerability in authentication";
        let files = Vec::new();

        assert!(engine.check_needs_review(output, &files));
    }

    #[test]
    fn test_no_review_needed() {
        let config = DecisionEngineConfig {
            review_file_threshold: 5,
            always_review_patterns: Vec::new(),
            ..Default::default()
        };
        let engine = DecisionEngine::with_config(config);

        let output = "Minor documentation update";
        let files = vec!["README.md".to_string()];

        assert!(!engine.check_needs_review(output, &files));
    }

    // ==================== Acceptance Criteria Tests ====================

    #[test]
    fn test_check_acceptance_criteria_met() {
        let engine = DecisionEngine::new();

        let criteria = vec![
            "Create database migration".to_string(),
            "Implement CRUD operations".to_string(),
        ];

        let output = "I have created the database migration and implemented the CRUD operations. All tests pass.";

        let (met, _incomplete) = engine.check_acceptance_criteria(&criteria, output);

        assert!(!met.is_empty());
        assert!(met.contains(&"Create database migration".to_string()) || met.contains(&"Implement CRUD operations".to_string()));
    }

    #[test]
    fn test_check_acceptance_criteria_incomplete() {
        let engine = DecisionEngine::new();

        let criteria = vec![
            "Implement authentication flow".to_string(),
        ];

        let output = "Started working on the project structure.";

        let (met, incomplete) = engine.check_acceptance_criteria(&criteria, output);

        assert!(met.is_empty());
        assert!(!incomplete.is_empty());
    }

    // ==================== Decision Making Tests ====================

    #[test]
    fn test_decision_on_complete_status() {
        let engine = DecisionEngine::new();

        let output = "Task completed successfully.\n\nSTATUS: COMPLETE";
        let eval = engine.evaluate_agent_output(output);

        assert!(eval.status_signal.is_some());
        assert_eq!(eval.status_signal.as_ref().unwrap().status, AgentStatus::Complete);
    }

    #[test]
    fn test_decision_on_blocked_status() {
        let engine = DecisionEngine::new();

        let output = "Cannot proceed.\n\nSTATUS: BLOCKED - Missing dependencies";
        let eval = engine.evaluate_agent_output(output);

        assert!(eval.recommended_decision.is_some());
        match &eval.recommended_decision {
            Some(Decision::Escalate { severity, .. }) => {
                assert_eq!(*severity, EscalationSeverity::Medium);
            }
            _ => panic!("Expected Escalate decision"),
        }
    }

    #[test]
    fn test_decision_triggers_review() {
        let engine = DecisionEngine::new();

        let output = "Created file `src/new_feature.rs`\n\nSTATUS: COMPLETE";
        let eval = engine.evaluate_agent_output(output);

        assert!(eval.needs_review);
        match &eval.recommended_decision {
            Some(Decision::TriggerReview { files_changed, review_type }) => {
                assert!(!files_changed.is_empty());
                assert_eq!(*review_type, ReviewType::Full);
            }
            _ => panic!("Expected TriggerReview decision"),
        }
    }

    #[test]
    fn test_make_decision_max_retries() {
        let engine = DecisionEngine::new();

        let eval = engine.evaluate_agent_output("Some output");
        let decision = engine.make_decision(&eval, "executing", 5);

        match decision {
            Decision::Escalate { reason, severity, .. } => {
                assert!(reason.contains("Maximum retries"));
                assert_eq!(severity, EscalationSeverity::High);
            }
            _ => panic!("Expected Escalate decision"),
        }
    }

    #[test]
    fn test_make_decision_state_transition() {
        let engine = DecisionEngine::new();

        let eval = engine.evaluate_agent_output("Normal progress, no status signal");
        let decision = engine.make_decision(&eval, "planning", 0);

        match decision {
            Decision::TransitionState { new_state } => {
                assert_eq!(new_state, "executing");
            }
            _ => panic!("Expected TransitionState decision"),
        }
    }

    // ==================== Full Evaluation Tests ====================

    #[test]
    fn test_full_evaluation() {
        let engine = DecisionEngine::new();

        let output = r#"
I have implemented the feature as requested:

1. Created the database migration in `migrations/001.sql`
2. Added the new module in `src/feature.rs`
3. Updated `src/lib.rs` to export the module

All tests pass:
test test_feature_creation ... ok
test test_feature_update ... ok

STATUS: COMPLETE - Feature fully implemented
        "#;

        let eval = engine.evaluate_agent_output(output);

        assert!(eval.status_signal.is_some());
        assert_eq!(eval.status_signal.as_ref().unwrap().status, AgentStatus::Complete);
        assert!(!eval.files_changed.is_empty());
        assert!(!eval.tests_affected.is_empty());
        assert!(eval.needs_review);
    }

    #[test]
    fn test_evaluate_error_output() {
        let engine = DecisionEngine::new();

        let output = "Build failed with errors.\n\nSTATUS: ERROR - Compilation failed";
        let eval = engine.evaluate_agent_output(output);

        assert!(eval.status_signal.is_some());
        assert_eq!(eval.status_signal.as_ref().unwrap().status, AgentStatus::Error);

        match &eval.recommended_decision {
            Some(Decision::Escalate { severity, .. }) => {
                assert_eq!(*severity, EscalationSeverity::High);
            }
            _ => panic!("Expected Escalate decision"),
        }
    }

    // ==================== Decision Engine Config Tests ====================

    #[test]
    fn test_config_default() {
        let config = DecisionEngineConfig::default();

        assert_eq!(config.review_file_threshold, 1);
        assert_eq!(config.max_retries, 3);
        assert!(config.auto_escalate_on_error);
        assert!(!config.always_review_patterns.is_empty());
    }

    #[test]
    fn test_custom_config() {
        let config = DecisionEngineConfig {
            review_file_threshold: 10,
            always_review_patterns: vec!["*.rs".to_string()],
            max_retries: 5,
            auto_escalate_on_error: false,
        };

        let engine = DecisionEngine::with_config(config);

        // Test that error doesn't auto-escalate
        let output = "STATUS: ERROR - Something went wrong";
        let eval = engine.evaluate_agent_output(output);

        match &eval.recommended_decision {
            Some(Decision::Retry { .. }) => {}
            _ => panic!("Expected Retry decision when auto_escalate_on_error is false"),
        }
    }
}
