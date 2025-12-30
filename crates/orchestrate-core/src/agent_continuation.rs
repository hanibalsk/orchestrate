//! Agent Continuation Management
//!
//! Enables completed or paused agents to be resumed with new tasks
//! while preserving their context and message history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Reasons for continuing an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationReason {
    /// Code review provided feedback
    ReviewFeedback,
    /// Tests failed, need fixes
    TestFailures,
    /// Acceptance criteria incomplete
    IncompleteCriteria,
    /// Additional task assigned
    AdditionalTask,
    /// Fix request from human/system
    FixRequest,
    /// Retry due to error
    Retry,
}

impl ContinuationReason {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ReviewFeedback => "review_feedback",
            Self::TestFailures => "test_failures",
            Self::IncompleteCriteria => "incomplete_criteria",
            Self::AdditionalTask => "additional_task",
            Self::FixRequest => "fix_request",
            Self::Retry => "retry",
        }
    }

    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "review_feedback" => Ok(Self::ReviewFeedback),
            "test_failures" => Ok(Self::TestFailures),
            "incomplete_criteria" => Ok(Self::IncompleteCriteria),
            "additional_task" => Ok(Self::AdditionalTask),
            "fix_request" => Ok(Self::FixRequest),
            "retry" => Ok(Self::Retry),
            _ => Err(crate::Error::Other(format!(
                "Unknown continuation reason: {}",
                s
            ))),
        }
    }

    /// Get a descriptive message prefix for this reason
    pub fn message_prefix(&self) -> &'static str {
        match self {
            Self::ReviewFeedback => "Code review feedback received:\n\n",
            Self::TestFailures => "Tests failed with the following errors:\n\n",
            Self::IncompleteCriteria => "The following acceptance criteria are not yet complete:\n\n",
            Self::AdditionalTask => "Additional task requested:\n\n",
            Self::FixRequest => "Fix requested:\n\n",
            Self::Retry => "Please retry the previous task. Issue:\n\n",
        }
    }
}

/// Status of a continuation request
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContinuationStatus {
    /// Request created, waiting to be processed
    Pending,
    /// Currently executing
    Executing,
    /// Successfully completed
    Completed,
    /// Failed during execution
    Failed,
    /// Cancelled before execution
    Cancelled,
}

impl ContinuationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "executing" => Ok(Self::Executing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(crate::Error::Other(format!(
                "Unknown continuation status: {}",
                s
            ))),
        }
    }

    /// Check if continuation is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

/// Result of a continuation execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuationResult {
    /// Whether the continuation succeeded
    pub success: bool,
    /// Summary of what was done
    pub summary: Option<String>,
    /// New agent state after continuation
    pub new_agent_state: Option<String>,
    /// Files changed during continuation
    #[serde(default)]
    pub files_changed: Vec<String>,
    /// Tests affected
    #[serde(default)]
    pub tests_affected: Vec<String>,
}

/// An agent continuation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentContinuation {
    /// Unique ID for this continuation request
    pub id: i64,
    /// ID of the agent to continue
    pub agent_id: String,
    /// Session ID for context continuity
    pub session_id: Option<String>,
    /// Reason for continuation
    pub reason: ContinuationReason,
    /// Message to send to the agent
    pub message: String,
    /// Additional context
    pub context: serde_json::Value,
    /// Current status
    pub status: ContinuationStatus,
    /// When the request was created
    pub created_at: DateTime<Utc>,
    /// When execution started
    pub started_at: Option<DateTime<Utc>>,
    /// When execution completed
    pub completed_at: Option<DateTime<Utc>>,
    /// Result of the continuation
    pub result: Option<ContinuationResult>,
    /// Error message if failed
    pub error_message: Option<String>,
}

impl AgentContinuation {
    /// Create a new continuation request
    pub fn new(
        agent_id: impl Into<String>,
        reason: ContinuationReason,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: 0, // Set by database
            agent_id: agent_id.into(),
            session_id: None,
            reason,
            message: message.into(),
            context: serde_json::Value::Null,
            status: ContinuationStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
            error_message: None,
        }
    }

    /// Add session ID for context continuity
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add additional context
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = context;
        self
    }

    /// Get the full message to send to the agent (including prefix)
    pub fn full_message(&self) -> String {
        format!("{}{}", self.reason.message_prefix(), self.message)
    }

    /// Mark as executing
    pub fn start_execution(&mut self) {
        self.status = ContinuationStatus::Executing;
        self.started_at = Some(Utc::now());
    }

    /// Mark as completed with result
    pub fn complete(&mut self, result: ContinuationResult) {
        self.status = if result.success {
            ContinuationStatus::Completed
        } else {
            ContinuationStatus::Failed
        };
        self.completed_at = Some(Utc::now());
        self.result = Some(result);
    }

    /// Mark as failed with error
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = ContinuationStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.error_message = Some(error.into());
    }

    /// Cancel the continuation
    pub fn cancel(&mut self) {
        self.status = ContinuationStatus::Cancelled;
        self.completed_at = Some(Utc::now());
    }

    /// Get execution duration (if completed)
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.completed_at) {
            (Some(start), Some(end)) => Some(end - start),
            (Some(start), None) => Some(Utc::now() - start),
            _ => None,
        }
    }
}

/// Builder for creating continuation requests from review feedback
pub struct ContinuationBuilder {
    agent_id: String,
    session_id: Option<String>,
    reason: ContinuationReason,
    message_parts: Vec<String>,
    context: serde_json::Value,
}

impl ContinuationBuilder {
    /// Start building a continuation for review feedback
    pub fn review_feedback(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: None,
            reason: ContinuationReason::ReviewFeedback,
            message_parts: Vec::new(),
            context: serde_json::Value::Null,
        }
    }

    /// Start building a continuation for test failures
    pub fn test_failures(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: None,
            reason: ContinuationReason::TestFailures,
            message_parts: Vec::new(),
            context: serde_json::Value::Null,
        }
    }

    /// Start building a continuation for incomplete criteria
    pub fn incomplete_criteria(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: None,
            reason: ContinuationReason::IncompleteCriteria,
            message_parts: Vec::new(),
            context: serde_json::Value::Null,
        }
    }

    /// Start building a continuation for additional task
    pub fn additional_task(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: None,
            reason: ContinuationReason::AdditionalTask,
            message_parts: Vec::new(),
            context: serde_json::Value::Null,
        }
    }

    /// Start building a continuation for fix request
    pub fn fix_request(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: None,
            reason: ContinuationReason::FixRequest,
            message_parts: Vec::new(),
            context: serde_json::Value::Null,
        }
    }

    /// Start building a retry continuation
    pub fn retry(agent_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: None,
            reason: ContinuationReason::Retry,
            message_parts: Vec::new(),
            context: serde_json::Value::Null,
        }
    }

    /// Set session ID
    pub fn session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add a line to the message
    pub fn add_line(mut self, line: impl Into<String>) -> Self {
        self.message_parts.push(line.into());
        self
    }

    /// Add multiple lines to the message
    pub fn add_lines(mut self, lines: impl IntoIterator<Item = impl Into<String>>) -> Self {
        for line in lines {
            self.message_parts.push(line.into());
        }
        self
    }

    /// Add review comment
    pub fn add_comment(self, file: &str, line: Option<u32>, comment: &str) -> Self {
        let location = if let Some(l) = line {
            format!("{}:{}", file, l)
        } else {
            file.to_string()
        };
        self.add_line(format!("- [{}] {}", location, comment))
    }

    /// Add failed test
    pub fn add_failed_test(self, test_name: &str, error: &str) -> Self {
        self.add_line(format!("- Test '{}' failed: {}", test_name, error))
    }

    /// Add incomplete criterion
    pub fn add_incomplete_criterion(self, criterion: &str) -> Self {
        self.add_line(format!("- [ ] {}", criterion))
    }

    /// Set additional context
    pub fn context(mut self, context: serde_json::Value) -> Self {
        self.context = context;
        self
    }

    /// Build the continuation request
    pub fn build(self) -> AgentContinuation {
        AgentContinuation {
            id: 0,
            agent_id: self.agent_id,
            session_id: self.session_id,
            reason: self.reason,
            message: self.message_parts.join("\n"),
            context: self.context,
            status: ContinuationStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
            error_message: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ContinuationReason Tests ====================

    #[test]
    fn test_continuation_reason_as_str() {
        assert_eq!(ContinuationReason::ReviewFeedback.as_str(), "review_feedback");
        assert_eq!(ContinuationReason::TestFailures.as_str(), "test_failures");
        assert_eq!(
            ContinuationReason::IncompleteCriteria.as_str(),
            "incomplete_criteria"
        );
        assert_eq!(ContinuationReason::AdditionalTask.as_str(), "additional_task");
        assert_eq!(ContinuationReason::FixRequest.as_str(), "fix_request");
        assert_eq!(ContinuationReason::Retry.as_str(), "retry");
    }

    #[test]
    fn test_continuation_reason_from_str() {
        assert_eq!(
            ContinuationReason::from_str("review_feedback").unwrap(),
            ContinuationReason::ReviewFeedback
        );
        assert_eq!(
            ContinuationReason::from_str("test_failures").unwrap(),
            ContinuationReason::TestFailures
        );
        assert!(ContinuationReason::from_str("invalid").is_err());
    }

    #[test]
    fn test_continuation_reason_message_prefix() {
        assert!(ContinuationReason::ReviewFeedback
            .message_prefix()
            .contains("review"));
        assert!(ContinuationReason::TestFailures
            .message_prefix()
            .contains("Tests failed"));
    }

    // ==================== ContinuationStatus Tests ====================

    #[test]
    fn test_continuation_status_as_str() {
        assert_eq!(ContinuationStatus::Pending.as_str(), "pending");
        assert_eq!(ContinuationStatus::Executing.as_str(), "executing");
        assert_eq!(ContinuationStatus::Completed.as_str(), "completed");
        assert_eq!(ContinuationStatus::Failed.as_str(), "failed");
        assert_eq!(ContinuationStatus::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn test_continuation_status_from_str() {
        assert_eq!(
            ContinuationStatus::from_str("pending").unwrap(),
            ContinuationStatus::Pending
        );
        assert_eq!(
            ContinuationStatus::from_str("completed").unwrap(),
            ContinuationStatus::Completed
        );
        assert!(ContinuationStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_continuation_status_is_terminal() {
        assert!(!ContinuationStatus::Pending.is_terminal());
        assert!(!ContinuationStatus::Executing.is_terminal());
        assert!(ContinuationStatus::Completed.is_terminal());
        assert!(ContinuationStatus::Failed.is_terminal());
        assert!(ContinuationStatus::Cancelled.is_terminal());
    }

    // ==================== AgentContinuation Tests ====================

    #[test]
    fn test_continuation_new() {
        let cont = AgentContinuation::new(
            "agent-123",
            ContinuationReason::ReviewFeedback,
            "Please fix the formatting",
        );

        assert_eq!(cont.agent_id, "agent-123");
        assert_eq!(cont.reason, ContinuationReason::ReviewFeedback);
        assert_eq!(cont.message, "Please fix the formatting");
        assert_eq!(cont.status, ContinuationStatus::Pending);
    }

    #[test]
    fn test_continuation_with_session() {
        let cont = AgentContinuation::new(
            "agent-123",
            ContinuationReason::TestFailures,
            "Fix test failures",
        )
        .with_session("session-456");

        assert_eq!(cont.session_id, Some("session-456".to_string()));
    }

    #[test]
    fn test_continuation_with_context() {
        let context = serde_json::json!({"file": "src/lib.rs", "line": 42});
        let cont = AgentContinuation::new(
            "agent-123",
            ContinuationReason::FixRequest,
            "Fix the bug",
        )
        .with_context(context.clone());

        assert_eq!(cont.context, context);
    }

    #[test]
    fn test_continuation_full_message() {
        let cont = AgentContinuation::new(
            "agent-123",
            ContinuationReason::ReviewFeedback,
            "Variable naming issue",
        );

        let full = cont.full_message();
        assert!(full.contains("Code review feedback received"));
        assert!(full.contains("Variable naming issue"));
    }

    #[test]
    fn test_continuation_start_execution() {
        let mut cont =
            AgentContinuation::new("agent-123", ContinuationReason::Retry, "Retry the task");

        cont.start_execution();

        assert_eq!(cont.status, ContinuationStatus::Executing);
        assert!(cont.started_at.is_some());
    }

    #[test]
    fn test_continuation_complete_success() {
        let mut cont =
            AgentContinuation::new("agent-123", ContinuationReason::TestFailures, "Fix tests");
        cont.start_execution();

        cont.complete(ContinuationResult {
            success: true,
            summary: Some("Fixed all tests".to_string()),
            new_agent_state: Some("completed".to_string()),
            files_changed: vec!["src/lib.rs".to_string()],
            tests_affected: vec!["test_feature".to_string()],
        });

        assert_eq!(cont.status, ContinuationStatus::Completed);
        assert!(cont.completed_at.is_some());
        assert!(cont.result.is_some());
        assert!(cont.result.as_ref().unwrap().success);
    }

    #[test]
    fn test_continuation_complete_failure() {
        let mut cont =
            AgentContinuation::new("agent-123", ContinuationReason::FixRequest, "Fix bug");
        cont.start_execution();

        cont.complete(ContinuationResult {
            success: false,
            summary: Some("Could not fix the bug".to_string()),
            new_agent_state: None,
            files_changed: vec![],
            tests_affected: vec![],
        });

        assert_eq!(cont.status, ContinuationStatus::Failed);
    }

    #[test]
    fn test_continuation_fail() {
        let mut cont =
            AgentContinuation::new("agent-123", ContinuationReason::Retry, "Retry");
        cont.start_execution();

        cont.fail("Agent crashed unexpectedly");

        assert_eq!(cont.status, ContinuationStatus::Failed);
        assert_eq!(
            cont.error_message,
            Some("Agent crashed unexpectedly".to_string())
        );
    }

    #[test]
    fn test_continuation_cancel() {
        let mut cont =
            AgentContinuation::new("agent-123", ContinuationReason::AdditionalTask, "New task");

        cont.cancel();

        assert_eq!(cont.status, ContinuationStatus::Cancelled);
        assert!(cont.completed_at.is_some());
    }

    #[test]
    fn test_continuation_duration() {
        let mut cont =
            AgentContinuation::new("agent-123", ContinuationReason::Retry, "Retry");

        // No duration before start
        assert!(cont.duration().is_none());

        cont.start_execution();
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Has duration while executing
        let dur = cont.duration();
        assert!(dur.is_some());
        assert!(dur.unwrap().num_milliseconds() >= 10);

        cont.complete(ContinuationResult {
            success: true,
            summary: None,
            new_agent_state: None,
            files_changed: vec![],
            tests_affected: vec![],
        });

        // Still has duration after completion
        assert!(cont.duration().is_some());
    }

    // ==================== ContinuationBuilder Tests ====================

    #[test]
    fn test_builder_review_feedback() {
        let cont = ContinuationBuilder::review_feedback("agent-123")
            .add_comment("src/lib.rs", Some(42), "Use snake_case for variables")
            .add_comment("src/main.rs", None, "Missing documentation")
            .build();

        assert_eq!(cont.reason, ContinuationReason::ReviewFeedback);
        assert!(cont.message.contains("src/lib.rs:42"));
        assert!(cont.message.contains("snake_case"));
    }

    #[test]
    fn test_builder_test_failures() {
        let cont = ContinuationBuilder::test_failures("agent-123")
            .add_failed_test("test_create", "assertion failed")
            .add_failed_test("test_update", "timeout")
            .build();

        assert_eq!(cont.reason, ContinuationReason::TestFailures);
        assert!(cont.message.contains("test_create"));
        assert!(cont.message.contains("assertion failed"));
    }

    #[test]
    fn test_builder_incomplete_criteria() {
        let cont = ContinuationBuilder::incomplete_criteria("agent-123")
            .add_incomplete_criterion("Implement database migration")
            .add_incomplete_criterion("Add unit tests")
            .build();

        assert_eq!(cont.reason, ContinuationReason::IncompleteCriteria);
        assert!(cont.message.contains("[ ] Implement database migration"));
        assert!(cont.message.contains("[ ] Add unit tests"));
    }

    #[test]
    fn test_builder_additional_task() {
        let cont = ContinuationBuilder::additional_task("agent-123")
            .add_line("Also implement the export feature")
            .session("session-456")
            .build();

        assert_eq!(cont.reason, ContinuationReason::AdditionalTask);
        assert_eq!(cont.session_id, Some("session-456".to_string()));
        assert!(cont.message.contains("export feature"));
    }

    #[test]
    fn test_builder_with_context() {
        let context = serde_json::json!({"pr_number": 123});
        let cont = ContinuationBuilder::fix_request("agent-123")
            .add_line("Fix CI failure")
            .context(context.clone())
            .build();

        assert_eq!(cont.context, context);
    }

    #[test]
    fn test_builder_add_lines() {
        let cont = ContinuationBuilder::retry("agent-123")
            .add_lines(vec![
                "First issue",
                "Second issue",
                "Third issue",
            ])
            .build();

        assert!(cont.message.contains("First issue"));
        assert!(cont.message.contains("Second issue"));
        assert!(cont.message.contains("Third issue"));
    }
}
