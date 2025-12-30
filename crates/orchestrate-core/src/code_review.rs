//! Code Review Integration
//!
//! Epic 016: Autonomous Epic Processing - Story 9
//!
//! Handles triggering and processing code reviews in the autonomous workflow:
//! - Auto-trigger code-reviewer after story completion
//! - Parse review output for machine-readable verdict
//! - Generate continuation messages from review feedback
//! - Track review iterations and handle escalation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::work_evaluation::{
    FeedbackItem, FeedbackType, ReviewIssue, ReviewIssueSeverity, ReviewResult, ReviewVerdict,
    WorkEvaluator,
};

/// Review request to trigger a code review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewRequest {
    /// Story being reviewed
    pub story_id: String,
    /// Agent that completed the work
    pub agent_id: String,
    /// Session ID for context
    pub session_id: Option<String>,
    /// Branch containing the changes
    pub branch: String,
    /// Base branch to compare against
    pub base_branch: String,
    /// PR number if PR exists
    pub pr_number: Option<u64>,
    /// Review iteration number
    pub iteration: u32,
    /// Files changed in this story
    pub changed_files: Vec<String>,
    /// Acceptance criteria to verify
    pub criteria: Vec<String>,
    /// Previous review issues (for context)
    pub previous_issues: Vec<ReviewIssue>,
    /// Created at timestamp
    pub created_at: DateTime<Utc>,
}

impl ReviewRequest {
    pub fn new(
        story_id: impl Into<String>,
        agent_id: impl Into<String>,
        branch: impl Into<String>,
        base_branch: impl Into<String>,
    ) -> Self {
        Self {
            story_id: story_id.into(),
            agent_id: agent_id.into(),
            session_id: None,
            branch: branch.into(),
            base_branch: base_branch.into(),
            pr_number: None,
            iteration: 1,
            changed_files: Vec::new(),
            criteria: Vec::new(),
            previous_issues: Vec::new(),
            created_at: Utc::now(),
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_pr(mut self, pr_number: u64) -> Self {
        self.pr_number = Some(pr_number);
        self
    }

    pub fn with_iteration(mut self, iteration: u32) -> Self {
        self.iteration = iteration;
        self
    }

    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.changed_files = files;
        self
    }

    pub fn with_criteria(mut self, criteria: Vec<String>) -> Self {
        self.criteria = criteria;
        self
    }

    pub fn with_previous_issues(mut self, issues: Vec<ReviewIssue>) -> Self {
        self.previous_issues = issues;
        self
    }
}

/// Type of code reviewer
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerType {
    /// Automated code-reviewer agent
    Automated,
    /// Human reviewer
    Human,
    /// GitHub Copilot review
    Copilot,
    /// External review tool
    External,
}

impl ReviewerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Automated => "automated",
            Self::Human => "human",
            Self::Copilot => "copilot",
            Self::External => "external",
        }
    }
}

impl std::str::FromStr for ReviewerType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "automated" | "auto" | "agent" => Ok(Self::Automated),
            "human" | "manual" => Ok(Self::Human),
            "copilot" | "github-copilot" => Ok(Self::Copilot),
            "external" | "tool" => Ok(Self::External),
            _ => Err(crate::Error::Other(format!("Invalid reviewer type: {}", s))),
        }
    }
}

impl std::fmt::Display for ReviewerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Review response from any reviewer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResponse {
    /// Review request ID
    pub request_id: Option<i64>,
    /// Story being reviewed
    pub story_id: String,
    /// Reviewer type
    pub reviewer_type: ReviewerType,
    /// Reviewer identifier
    pub reviewer: Option<String>,
    /// Review result
    pub result: ReviewResult,
    /// Raw review output/comments
    pub raw_output: Option<String>,
    /// Time taken for review (seconds)
    pub duration_secs: Option<u64>,
    /// Completed at timestamp
    pub completed_at: DateTime<Utc>,
}

impl ReviewResponse {
    pub fn new(story_id: impl Into<String>, reviewer_type: ReviewerType, result: ReviewResult) -> Self {
        Self {
            request_id: None,
            story_id: story_id.into(),
            reviewer_type,
            reviewer: None,
            result,
            raw_output: None,
            duration_secs: None,
            completed_at: Utc::now(),
        }
    }

    pub fn with_reviewer(mut self, reviewer: impl Into<String>) -> Self {
        self.reviewer = Some(reviewer.into());
        self
    }

    pub fn with_raw_output(mut self, output: impl Into<String>) -> Self {
        self.raw_output = Some(output.into());
        self
    }

    pub fn with_duration(mut self, secs: u64) -> Self {
        self.duration_secs = Some(secs);
        self
    }
}

/// Escalation level for review issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewEscalationLevel {
    /// No escalation needed
    None,
    /// Suggest human review
    SuggestHuman,
    /// Require human review
    RequireHuman,
    /// Escalate to senior reviewer
    Senior,
    /// Block until resolved
    Block,
}

impl ReviewEscalationLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::SuggestHuman => "suggest_human",
            Self::RequireHuman => "require_human",
            Self::Senior => "senior",
            Self::Block => "block",
        }
    }
}

/// Configuration for code review integration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeReviewConfig {
    /// Maximum review iterations before escalation
    pub max_iterations: u32,
    /// Auto-approve if only nitpick issues
    pub auto_approve_nitpicks: bool,
    /// Escalate after N failed iterations
    pub escalate_after_iterations: u32,
    /// Timeout for automated reviews (seconds)
    pub review_timeout_secs: u64,
    /// Timeout for human reviews (seconds)
    pub human_review_timeout_secs: u64,
    /// Reviewer type preference order
    pub preferred_reviewers: Vec<ReviewerType>,
    /// Require human review for critical issues
    pub require_human_for_critical: bool,
}

impl Default for CodeReviewConfig {
    fn default() -> Self {
        Self {
            max_iterations: 5,
            auto_approve_nitpicks: true,
            escalate_after_iterations: 3,
            review_timeout_secs: 300, // 5 minutes
            human_review_timeout_secs: 86400, // 24 hours
            preferred_reviewers: vec![ReviewerType::Automated, ReviewerType::Human],
            require_human_for_critical: true,
        }
    }
}

/// Code review coordinator
#[derive(Debug, Clone)]
pub struct CodeReviewCoordinator {
    config: CodeReviewConfig,
    work_evaluator: WorkEvaluator,
}

impl CodeReviewCoordinator {
    pub fn new() -> Self {
        Self {
            config: CodeReviewConfig::default(),
            work_evaluator: WorkEvaluator::new(),
        }
    }

    pub fn with_config(config: CodeReviewConfig) -> Self {
        Self {
            config,
            work_evaluator: WorkEvaluator::new(),
        }
    }

    /// Check if review is needed for completed work
    pub fn needs_review(&self, story_complete: bool, has_code_changes: bool) -> bool {
        story_complete && has_code_changes
    }

    /// Create a review request for a completed story
    pub fn create_review_request(
        &self,
        story_id: &str,
        agent_id: &str,
        branch: &str,
        base_branch: &str,
        changed_files: Vec<String>,
        criteria: Vec<String>,
        iteration: u32,
        previous_issues: Vec<ReviewIssue>,
    ) -> ReviewRequest {
        ReviewRequest::new(story_id, agent_id, branch, base_branch)
            .with_files(changed_files)
            .with_criteria(criteria)
            .with_iteration(iteration)
            .with_previous_issues(previous_issues)
    }

    /// Parse review output from code-reviewer agent
    pub fn parse_review_output(&self, output: &str) -> ReviewResult {
        self.work_evaluator.parse_review_output(output)
    }

    /// Check if review should be escalated
    pub fn should_escalate(&self, iteration: u32, result: &ReviewResult) -> ReviewEscalationLevel {
        // Check for critical issues requiring human review
        if self.config.require_human_for_critical && result.issues.iter().any(|i| matches!(i.severity, ReviewIssueSeverity::Critical)) {
            return ReviewEscalationLevel::RequireHuman;
        }

        // Check iteration count
        if iteration >= self.config.max_iterations {
            return ReviewEscalationLevel::Block;
        }

        if iteration >= self.config.escalate_after_iterations {
            if result.has_blocking_issues() {
                return ReviewEscalationLevel::RequireHuman;
            } else {
                return ReviewEscalationLevel::SuggestHuman;
            }
        }

        ReviewEscalationLevel::None
    }

    /// Check if review can be auto-approved
    pub fn can_auto_approve(&self, result: &ReviewResult) -> bool {
        // Can't auto-approve if not approved
        if !result.verdict.is_passing() {
            return false;
        }

        // Can't auto-approve if there are blocking issues
        if result.has_blocking_issues() {
            return false;
        }

        // If configured to auto-approve nitpicks, allow if only nitpicks
        if self.config.auto_approve_nitpicks {
            let only_nitpicks = result.issues.iter().all(|i| {
                matches!(i.severity, ReviewIssueSeverity::Nitpick | ReviewIssueSeverity::Low)
            });
            return only_nitpicks || result.issues.is_empty();
        }

        result.issues.is_empty()
    }

    /// Generate continuation message for addressing review feedback
    pub fn generate_continuation_message(&self, response: &ReviewResponse) -> String {
        let mut parts = Vec::new();

        // Add header based on verdict
        match response.result.verdict {
            ReviewVerdict::Approved => {
                if response.result.issues.is_empty() {
                    return "Code review approved! No changes needed.".to_string();
                }
                parts.push(format!(
                    "Code review approved with {} suggestions to consider:",
                    response.result.issues.len()
                ));
            }
            ReviewVerdict::ChangesRequested => {
                parts.push(format!(
                    "Code review requested changes. Please address {} issues:",
                    response.result.issues.len()
                ));
            }
            ReviewVerdict::NeedsDiscussion => {
                parts.push("Code review needs clarification on some points:".to_string());
            }
            ReviewVerdict::Pending => {
                parts.push("Review is still pending.".to_string());
            }
        }

        // Group issues by severity
        let mut critical: Vec<&ReviewIssue> = Vec::new();
        let mut high: Vec<&ReviewIssue> = Vec::new();
        let mut medium: Vec<&ReviewIssue> = Vec::new();
        let mut low: Vec<&ReviewIssue> = Vec::new();
        let mut nitpick: Vec<&ReviewIssue> = Vec::new();

        for issue in &response.result.issues {
            match issue.severity {
                ReviewIssueSeverity::Critical => critical.push(issue),
                ReviewIssueSeverity::High => high.push(issue),
                ReviewIssueSeverity::Medium => medium.push(issue),
                ReviewIssueSeverity::Low => low.push(issue),
                ReviewIssueSeverity::Nitpick => nitpick.push(issue),
            }
        }

        // Add issues by severity (highest first)
        if !critical.is_empty() {
            parts.push("\n## CRITICAL (must fix):".to_string());
            for issue in critical {
                let location = issue.file_path.as_ref().map(|f| {
                    if let Some(line) = issue.line_number {
                        format!(" ({f}:{line})")
                    } else {
                        format!(" ({f})")
                    }
                }).unwrap_or_default();
                parts.push(format!("- {}{}", issue.description, location));
                if let Some(suggestion) = &issue.suggestion {
                    parts.push(format!("  Suggestion: {suggestion}"));
                }
            }
        }

        if !high.is_empty() {
            parts.push("\n## HIGH (should fix):".to_string());
            for issue in high {
                let location = issue.file_path.as_ref().map(|f| {
                    if let Some(line) = issue.line_number {
                        format!(" ({f}:{line})")
                    } else {
                        format!(" ({f})")
                    }
                }).unwrap_or_default();
                parts.push(format!("- {}{}", issue.description, location));
                if let Some(suggestion) = &issue.suggestion {
                    parts.push(format!("  Suggestion: {suggestion}"));
                }
            }
        }

        if !medium.is_empty() {
            parts.push("\n## MEDIUM (recommended):".to_string());
            for issue in medium {
                parts.push(format!("- {}", issue.description));
            }
        }

        if !low.is_empty() {
            parts.push("\n## LOW (consider):".to_string());
            for issue in low {
                parts.push(format!("- {}", issue.description));
            }
        }

        if !nitpick.is_empty() {
            parts.push("\n## NITPICK (optional):".to_string());
            for issue in nitpick {
                parts.push(format!("- {}", issue.description));
            }
        }

        // Add action guidance
        if response.result.has_blocking_issues() {
            parts.push("\nPlease address CRITICAL and HIGH issues before requesting another review.".to_string());
        } else if !response.result.issues.is_empty() && response.result.verdict.is_passing() {
            parts.push("\nThese are suggestions - the review is approved. Consider addressing for improved code quality.".to_string());
        }

        parts.join("\n")
    }

    /// Generate feedback items from review response
    pub fn generate_feedback_items(&self, response: &ReviewResponse) -> Vec<FeedbackItem> {
        let mut items = Vec::new();

        for issue in &response.result.issues {
            let priority = match issue.severity {
                ReviewIssueSeverity::Critical => 100,
                ReviewIssueSeverity::High => 90,
                ReviewIssueSeverity::Medium => 70,
                ReviewIssueSeverity::Low => 50,
                ReviewIssueSeverity::Nitpick => 30,
            };

            let mut item = FeedbackItem::new(FeedbackType::ReviewIssue, &issue.description)
                .with_priority(priority);

            if let Some(suggestion) = &issue.suggestion {
                item = item.with_action(suggestion.clone());
            } else if let Some(file) = &issue.file_path {
                let action = if let Some(line) = issue.line_number {
                    format!("Fix issue at {file}:{line}")
                } else {
                    format!("Fix issue in {file}")
                };
                item = item.with_action(action);
            }

            items.push(item);
        }

        // Sort by priority
        items.sort_by(|a, b| b.priority.cmp(&a.priority));

        items
    }

    /// Determine next reviewer type based on current state
    pub fn next_reviewer_type(
        &self,
        current_iteration: u32,
        last_reviewer: Option<ReviewerType>,
        has_critical_issues: bool,
    ) -> ReviewerType {
        // Critical issues require human review if configured
        if has_critical_issues && self.config.require_human_for_critical {
            return ReviewerType::Human;
        }

        // After escalation threshold, prefer human
        if current_iteration >= self.config.escalate_after_iterations {
            return ReviewerType::Human;
        }

        // Use preference order
        if let Some(last) = last_reviewer {
            // Find next reviewer in preference list
            let mut found_last = false;
            for reviewer in &self.config.preferred_reviewers {
                if found_last {
                    return *reviewer;
                }
                if *reviewer == last {
                    found_last = true;
                }
            }
        }

        // Default to first preference
        self.config.preferred_reviewers.first().copied().unwrap_or(ReviewerType::Automated)
    }
}

impl Default for CodeReviewCoordinator {
    fn default() -> Self {
        Self::new()
    }
}

/// Review iteration tracker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIteration {
    pub id: i64,
    pub story_id: String,
    pub iteration: u32,
    pub reviewer_type: ReviewerType,
    pub reviewer: Option<String>,
    pub verdict: ReviewVerdict,
    pub issue_count: u32,
    pub blocking_issue_count: u32,
    pub escalation_level: ReviewEscalationLevel,
    pub duration_secs: Option<u64>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl ReviewIteration {
    pub fn start(
        story_id: impl Into<String>,
        iteration: u32,
        reviewer_type: ReviewerType,
    ) -> Self {
        Self {
            id: 0,
            story_id: story_id.into(),
            iteration,
            reviewer_type,
            reviewer: None,
            verdict: ReviewVerdict::Pending,
            issue_count: 0,
            blocking_issue_count: 0,
            escalation_level: ReviewEscalationLevel::None,
            duration_secs: None,
            started_at: Utc::now(),
            completed_at: None,
        }
    }

    pub fn complete(&mut self, response: &ReviewResponse, escalation: ReviewEscalationLevel) {
        self.verdict = response.result.verdict;
        self.issue_count = response.result.issues.len() as u32;
        self.blocking_issue_count = response.result.issues.iter()
            .filter(|i| i.severity.blocks_merge())
            .count() as u32;
        self.escalation_level = escalation;
        self.duration_secs = response.duration_secs;
        self.reviewer = response.reviewer.clone();
        self.completed_at = Some(response.completed_at);
    }

    pub fn is_complete(&self) -> bool {
        self.completed_at.is_some()
    }

    pub fn was_approved(&self) -> bool {
        self.verdict.is_passing() && self.blocking_issue_count == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ReviewRequest Tests ====================

    #[test]
    fn test_review_request_new() {
        let request = ReviewRequest::new("story-1", "agent-1", "feature/story-1", "main");

        assert_eq!(request.story_id, "story-1");
        assert_eq!(request.agent_id, "agent-1");
        assert_eq!(request.branch, "feature/story-1");
        assert_eq!(request.base_branch, "main");
        assert_eq!(request.iteration, 1);
        assert!(request.changed_files.is_empty());
    }

    #[test]
    fn test_review_request_builder() {
        let request = ReviewRequest::new("story-1", "agent-1", "feature/story-1", "main")
            .with_session("session-1")
            .with_pr(42)
            .with_iteration(2)
            .with_files(vec!["src/lib.rs".to_string()])
            .with_criteria(vec!["Implement feature".to_string()]);

        assert_eq!(request.session_id, Some("session-1".to_string()));
        assert_eq!(request.pr_number, Some(42));
        assert_eq!(request.iteration, 2);
        assert_eq!(request.changed_files.len(), 1);
        assert_eq!(request.criteria.len(), 1);
    }

    // ==================== ReviewerType Tests ====================

    #[test]
    fn test_reviewer_type_roundtrip() {
        let types = [
            ReviewerType::Automated,
            ReviewerType::Human,
            ReviewerType::Copilot,
            ReviewerType::External,
        ];

        for t in types {
            let s = t.as_str();
            let parsed: ReviewerType = s.parse().unwrap();
            assert_eq!(t, parsed);
        }
    }

    // ==================== CodeReviewCoordinator Tests ====================

    #[test]
    fn test_needs_review() {
        let coordinator = CodeReviewCoordinator::new();

        assert!(coordinator.needs_review(true, true));
        assert!(!coordinator.needs_review(false, true));
        assert!(!coordinator.needs_review(true, false));
        assert!(!coordinator.needs_review(false, false));
    }

    #[test]
    fn test_can_auto_approve_approved_no_issues() {
        let coordinator = CodeReviewCoordinator::new();
        let result = ReviewResult::new(ReviewVerdict::Approved);

        assert!(coordinator.can_auto_approve(&result));
    }

    #[test]
    fn test_can_auto_approve_with_nitpicks() {
        let coordinator = CodeReviewCoordinator::new();
        let result = ReviewResult::new(ReviewVerdict::Approved).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::Nitpick, "Minor formatting"),
        ]);

        assert!(coordinator.can_auto_approve(&result));
    }

    #[test]
    fn test_cannot_auto_approve_with_high_issues() {
        let coordinator = CodeReviewCoordinator::new();
        let result = ReviewResult::new(ReviewVerdict::Approved).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::High, "Security issue"),
        ]);

        assert!(!coordinator.can_auto_approve(&result));
    }

    #[test]
    fn test_cannot_auto_approve_changes_requested() {
        let coordinator = CodeReviewCoordinator::new();
        let result = ReviewResult::new(ReviewVerdict::ChangesRequested);

        assert!(!coordinator.can_auto_approve(&result));
    }

    #[test]
    fn test_should_escalate_after_max_iterations() {
        let coordinator = CodeReviewCoordinator::new();
        let result = ReviewResult::new(ReviewVerdict::ChangesRequested);

        let escalation = coordinator.should_escalate(5, &result);
        assert_eq!(escalation, ReviewEscalationLevel::Block);
    }

    #[test]
    fn test_should_escalate_critical_issues() {
        let coordinator = CodeReviewCoordinator::new();
        let result = ReviewResult::new(ReviewVerdict::ChangesRequested).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::Critical, "Security vulnerability"),
        ]);

        let escalation = coordinator.should_escalate(1, &result);
        assert_eq!(escalation, ReviewEscalationLevel::RequireHuman);
    }

    #[test]
    fn test_no_escalation_needed() {
        let coordinator = CodeReviewCoordinator::new();
        let result = ReviewResult::new(ReviewVerdict::ChangesRequested).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::Medium, "Consider refactoring"),
        ]);

        let escalation = coordinator.should_escalate(1, &result);
        assert_eq!(escalation, ReviewEscalationLevel::None);
    }

    #[test]
    fn test_generate_continuation_message_approved() {
        let coordinator = CodeReviewCoordinator::new();
        let response = ReviewResponse::new(
            "story-1",
            ReviewerType::Automated,
            ReviewResult::new(ReviewVerdict::Approved),
        );

        let message = coordinator.generate_continuation_message(&response);
        assert!(message.contains("approved"));
    }

    #[test]
    fn test_generate_continuation_message_with_issues() {
        let coordinator = CodeReviewCoordinator::new();
        let result = ReviewResult::new(ReviewVerdict::ChangesRequested).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::Critical, "Security issue"),
            ReviewIssue::new(ReviewIssueSeverity::High, "Missing validation"),
            ReviewIssue::new(ReviewIssueSeverity::Medium, "Add tests"),
        ]);
        let response = ReviewResponse::new("story-1", ReviewerType::Automated, result);

        let message = coordinator.generate_continuation_message(&response);

        assert!(message.contains("CRITICAL"));
        assert!(message.contains("Security issue"));
        assert!(message.contains("HIGH"));
        assert!(message.contains("Missing validation"));
        assert!(message.contains("MEDIUM"));
        assert!(message.contains("Add tests"));
    }

    #[test]
    fn test_generate_feedback_items() {
        let coordinator = CodeReviewCoordinator::new();
        let result = ReviewResult::new(ReviewVerdict::ChangesRequested).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::Critical, "Critical bug"),
            ReviewIssue::new(ReviewIssueSeverity::Low, "Minor improvement"),
        ]);
        let response = ReviewResponse::new("story-1", ReviewerType::Automated, result);

        let items = coordinator.generate_feedback_items(&response);

        assert_eq!(items.len(), 2);
        // Critical should come first (higher priority)
        assert_eq!(items[0].priority, 100);
        assert!(items[0].message.contains("Critical bug"));
    }

    #[test]
    fn test_next_reviewer_type_first_iteration() {
        let coordinator = CodeReviewCoordinator::new();

        let next = coordinator.next_reviewer_type(1, None, false);
        assert_eq!(next, ReviewerType::Automated);
    }

    #[test]
    fn test_next_reviewer_type_after_escalation() {
        let coordinator = CodeReviewCoordinator::new();

        let next = coordinator.next_reviewer_type(3, Some(ReviewerType::Automated), false);
        assert_eq!(next, ReviewerType::Human);
    }

    #[test]
    fn test_next_reviewer_type_critical_issues() {
        let coordinator = CodeReviewCoordinator::new();

        let next = coordinator.next_reviewer_type(1, Some(ReviewerType::Automated), true);
        assert_eq!(next, ReviewerType::Human);
    }

    // ==================== ReviewIteration Tests ====================

    #[test]
    fn test_review_iteration_start() {
        let iteration = ReviewIteration::start("story-1", 1, ReviewerType::Automated);

        assert_eq!(iteration.story_id, "story-1");
        assert_eq!(iteration.iteration, 1);
        assert_eq!(iteration.reviewer_type, ReviewerType::Automated);
        assert_eq!(iteration.verdict, ReviewVerdict::Pending);
        assert!(!iteration.is_complete());
    }

    #[test]
    fn test_review_iteration_complete() {
        let mut iteration = ReviewIteration::start("story-1", 1, ReviewerType::Automated);

        let result = ReviewResult::new(ReviewVerdict::Approved).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::Low, "Minor issue"),
        ]);
        let response = ReviewResponse::new("story-1", ReviewerType::Automated, result)
            .with_reviewer("code-reviewer")
            .with_duration(120);

        iteration.complete(&response, ReviewEscalationLevel::None);

        assert!(iteration.is_complete());
        assert!(iteration.was_approved());
        assert_eq!(iteration.issue_count, 1);
        assert_eq!(iteration.blocking_issue_count, 0);
        assert_eq!(iteration.reviewer, Some("code-reviewer".to_string()));
    }

    #[test]
    fn test_review_iteration_not_approved_with_blocking() {
        let mut iteration = ReviewIteration::start("story-1", 1, ReviewerType::Automated);

        let result = ReviewResult::new(ReviewVerdict::ChangesRequested).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::High, "Blocking issue"),
        ]);
        let response = ReviewResponse::new("story-1", ReviewerType::Automated, result);

        iteration.complete(&response, ReviewEscalationLevel::None);

        assert!(iteration.is_complete());
        assert!(!iteration.was_approved());
        assert_eq!(iteration.blocking_issue_count, 1);
    }

    // ==================== Config Tests ====================

    #[test]
    fn test_config_custom_max_iterations() {
        let config = CodeReviewConfig {
            max_iterations: 3,
            escalate_after_iterations: 2,
            ..Default::default()
        };
        let coordinator = CodeReviewCoordinator::with_config(config);

        let result = ReviewResult::new(ReviewVerdict::ChangesRequested);

        // Should block at iteration 3
        let escalation = coordinator.should_escalate(3, &result);
        assert_eq!(escalation, ReviewEscalationLevel::Block);

        // Should escalate at iteration 2
        let escalation = coordinator.should_escalate(2, &result);
        assert_eq!(escalation, ReviewEscalationLevel::SuggestHuman);
    }

    #[test]
    fn test_config_disable_auto_approve_nitpicks() {
        let config = CodeReviewConfig {
            auto_approve_nitpicks: false,
            ..Default::default()
        };
        let coordinator = CodeReviewCoordinator::with_config(config);

        let result = ReviewResult::new(ReviewVerdict::Approved).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::Nitpick, "Style suggestion"),
        ]);

        // Should not auto-approve when nitpick auto-approve is disabled
        assert!(!coordinator.can_auto_approve(&result));
    }
}
