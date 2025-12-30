//! Work Evaluation System
//!
//! Epic 016: Autonomous Epic Processing - Story 8
//!
//! Evaluates if agent work is truly complete by checking:
//! - Acceptance criteria completion
//! - Test/CI status
//! - Code review status
//! - Build/lint status
//! - PR approval status
//! - Generates feedback for agent continuation

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::decision_engine::AgentStatus;

/// Review verdict from code review
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewVerdict {
    /// Code review approved
    Approved,
    /// Changes requested by reviewer
    ChangesRequested,
    /// Needs discussion or clarification
    NeedsDiscussion,
    /// Review pending
    Pending,
}

impl ReviewVerdict {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::ChangesRequested => "changes_requested",
            Self::NeedsDiscussion => "needs_discussion",
            Self::Pending => "pending",
        }
    }

    pub fn is_passing(&self) -> bool {
        matches!(self, Self::Approved)
    }
}

impl std::str::FromStr for ReviewVerdict {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "approved" | "approve" | "lgtm" => Ok(Self::Approved),
            "changes_requested" | "request_changes" | "reject" => Ok(Self::ChangesRequested),
            "needs_discussion" | "discuss" | "comment" => Ok(Self::NeedsDiscussion),
            "pending" | "awaiting" => Ok(Self::Pending),
            _ => Err(crate::Error::Other(format!(
                "Invalid review verdict: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for ReviewVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Severity of review issues
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReviewIssueSeverity {
    /// Just a style suggestion
    Nitpick,
    /// Minor issue, good to fix
    Low,
    /// Should be fixed
    Medium,
    /// Must be fixed before merge
    High,
    /// Critical blocker
    Critical,
}

impl ReviewIssueSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nitpick => "nitpick",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }

    /// Check if this severity blocks merge
    pub fn blocks_merge(&self) -> bool {
        matches!(self, Self::High | Self::Critical)
    }
}

impl std::str::FromStr for ReviewIssueSeverity {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "nitpick" | "nit" => Ok(Self::Nitpick),
            "low" | "minor" => Ok(Self::Low),
            "medium" | "moderate" => Ok(Self::Medium),
            "high" | "major" => Ok(Self::High),
            "critical" | "blocker" => Ok(Self::Critical),
            _ => Err(crate::Error::Other(format!(
                "Invalid review issue severity: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for ReviewIssueSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// An issue from code review
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewIssue {
    /// Issue severity
    pub severity: ReviewIssueSeverity,
    /// Issue description
    pub description: String,
    /// File path (if applicable)
    pub file_path: Option<String>,
    /// Line number (if applicable)
    pub line_number: Option<u32>,
    /// Suggested fix
    pub suggestion: Option<String>,
    /// Issue category
    pub category: Option<String>,
}

impl ReviewIssue {
    pub fn new(severity: ReviewIssueSeverity, description: impl Into<String>) -> Self {
        Self {
            severity,
            description: description.into(),
            file_path: None,
            line_number: None,
            suggestion: None,
            category: None,
        }
    }

    pub fn with_location(mut self, file_path: impl Into<String>, line: u32) -> Self {
        self.file_path = Some(file_path.into());
        self.line_number = Some(line);
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }
}

/// Code review result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewResult {
    /// Overall verdict
    pub verdict: ReviewVerdict,
    /// List of issues found
    pub issues: Vec<ReviewIssue>,
    /// Reviewer identifier
    pub reviewer: Option<String>,
    /// Review timestamp
    pub reviewed_at: DateTime<Utc>,
    /// Review iteration number
    pub iteration: u32,
    /// Raw review output (for debugging)
    pub raw_output: Option<String>,
}

impl ReviewResult {
    pub fn new(verdict: ReviewVerdict) -> Self {
        Self {
            verdict,
            issues: Vec::new(),
            reviewer: None,
            reviewed_at: Utc::now(),
            iteration: 1,
            raw_output: None,
        }
    }

    pub fn with_issues(mut self, issues: Vec<ReviewIssue>) -> Self {
        self.issues = issues;
        self
    }

    pub fn with_reviewer(mut self, reviewer: impl Into<String>) -> Self {
        self.reviewer = Some(reviewer.into());
        self
    }

    pub fn with_iteration(mut self, iteration: u32) -> Self {
        self.iteration = iteration;
        self
    }

    /// Check if review has blocking issues
    pub fn has_blocking_issues(&self) -> bool {
        self.issues.iter().any(|i| i.severity.blocks_merge())
    }

    /// Get issues by severity
    pub fn issues_by_severity(&self, severity: ReviewIssueSeverity) -> Vec<&ReviewIssue> {
        self.issues.iter().filter(|i| i.severity == severity).collect()
    }

    /// Get count of issues at each severity
    pub fn issue_counts(&self) -> std::collections::HashMap<ReviewIssueSeverity, usize> {
        let mut counts = std::collections::HashMap::new();
        for issue in &self.issues {
            *counts.entry(issue.severity).or_insert(0) += 1;
        }
        counts
    }
}

/// CI/Build status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiStatus {
    /// CI is running
    Running,
    /// CI passed
    Passed,
    /// CI failed
    Failed,
    /// CI cancelled
    Cancelled,
    /// CI timed out
    Timeout,
    /// CI pending (not started)
    Pending,
}

impl CiStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Timeout => "timeout",
            Self::Pending => "pending",
        }
    }

    pub fn is_passing(&self) -> bool {
        matches!(self, Self::Passed)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Passed | Self::Failed | Self::Cancelled | Self::Timeout)
    }
}

impl std::str::FromStr for CiStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "running" | "in_progress" | "queued" => Ok(Self::Running),
            "passed" | "success" | "completed" => Ok(Self::Passed),
            "failed" | "failure" | "error" => Ok(Self::Failed),
            "cancelled" | "canceled" | "skipped" => Ok(Self::Cancelled),
            "timeout" | "timed_out" => Ok(Self::Timeout),
            "pending" | "waiting" => Ok(Self::Pending),
            _ => Err(crate::Error::Other(format!("Invalid CI status: {}", s))),
        }
    }
}

impl std::fmt::Display for CiStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// CI check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiCheckResult {
    /// Check name (e.g., "build", "test", "lint")
    pub name: String,
    /// Check status
    pub status: CiStatus,
    /// Check URL (if available)
    pub url: Option<String>,
    /// Failure details
    pub failure_details: Option<String>,
    /// Duration in seconds
    pub duration_secs: Option<u64>,
}

impl CiCheckResult {
    pub fn new(name: impl Into<String>, status: CiStatus) -> Self {
        Self {
            name: name.into(),
            status,
            url: None,
            failure_details: None,
            duration_secs: None,
        }
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_failure(mut self, details: impl Into<String>) -> Self {
        self.failure_details = Some(details.into());
        self
    }
}

/// PR merge status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrMergeStatus {
    /// PR is mergeable
    Mergeable,
    /// PR has merge conflicts
    Conflicts,
    /// PR is blocked (reviews, checks, etc.)
    Blocked,
    /// PR is draft
    Draft,
    /// PR is already merged
    Merged,
    /// PR is closed
    Closed,
    /// Status unknown
    Unknown,
}

impl PrMergeStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Mergeable => "mergeable",
            Self::Conflicts => "conflicts",
            Self::Blocked => "blocked",
            Self::Draft => "draft",
            Self::Merged => "merged",
            Self::Closed => "closed",
            Self::Unknown => "unknown",
        }
    }

    pub fn can_merge(&self) -> bool {
        matches!(self, Self::Mergeable)
    }
}

impl std::str::FromStr for PrMergeStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "mergeable" | "clean" => Ok(Self::Mergeable),
            "conflicts" | "conflicting" | "dirty" => Ok(Self::Conflicts),
            "blocked" | "behind" => Ok(Self::Blocked),
            "draft" => Ok(Self::Draft),
            "merged" => Ok(Self::Merged),
            "closed" => Ok(Self::Closed),
            "unknown" | "unstable" => Ok(Self::Unknown),
            _ => Err(crate::Error::Other(format!(
                "Invalid PR merge status: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for PrMergeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Acceptance criterion check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriterionCheck {
    /// Criterion description
    pub criterion: String,
    /// Whether it's met
    pub is_met: bool,
    /// Evidence for the check
    pub evidence: Option<String>,
    /// Confidence level (0.0 - 1.0)
    pub confidence: f64,
}

impl CriterionCheck {
    pub fn met(criterion: impl Into<String>) -> Self {
        Self {
            criterion: criterion.into(),
            is_met: true,
            evidence: None,
            confidence: 1.0,
        }
    }

    pub fn unmet(criterion: impl Into<String>) -> Self {
        Self {
            criterion: criterion.into(),
            is_met: false,
            evidence: None,
            confidence: 1.0,
        }
    }

    pub fn with_evidence(mut self, evidence: impl Into<String>) -> Self {
        self.evidence = Some(evidence.into());
        self
    }

    pub fn with_confidence(mut self, confidence: f64) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

/// Overall work completion status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkCompletionStatus {
    /// Work is complete and ready
    Complete,
    /// Work is in progress
    InProgress,
    /// Work is blocked
    Blocked,
    /// Work has failed
    Failed,
    /// Needs code review
    NeedsReview,
    /// Needs review fixes
    NeedsReviewFixes,
    /// Needs CI fixes
    NeedsCiFixes,
    /// Needs PR approval
    NeedsPrApproval,
    /// Ready to merge
    ReadyToMerge,
}

impl WorkCompletionStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::NeedsReview => "needs_review",
            Self::NeedsReviewFixes => "needs_review_fixes",
            Self::NeedsCiFixes => "needs_ci_fixes",
            Self::NeedsPrApproval => "needs_pr_approval",
            Self::ReadyToMerge => "ready_to_merge",
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Complete | Self::ReadyToMerge)
    }

    pub fn needs_action(&self) -> bool {
        matches!(
            self,
            Self::NeedsReview | Self::NeedsReviewFixes | Self::NeedsCiFixes | Self::NeedsPrApproval
        )
    }
}

impl std::str::FromStr for WorkCompletionStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "complete" => Ok(Self::Complete),
            "in_progress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "failed" => Ok(Self::Failed),
            "needs_review" => Ok(Self::NeedsReview),
            "needs_review_fixes" => Ok(Self::NeedsReviewFixes),
            "needs_ci_fixes" => Ok(Self::NeedsCiFixes),
            "needs_pr_approval" => Ok(Self::NeedsPrApproval),
            "ready_to_merge" => Ok(Self::ReadyToMerge),
            _ => Err(crate::Error::Other(format!(
                "Invalid work completion status: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for WorkCompletionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Comprehensive work evaluation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkEvaluationResult {
    /// Overall completion status
    pub status: WorkCompletionStatus,
    /// Agent status signal (if detected)
    pub agent_status: Option<AgentStatus>,
    /// Acceptance criteria checks
    pub criteria_checks: Vec<CriterionCheck>,
    /// CI check results
    pub ci_checks: Vec<CiCheckResult>,
    /// Code review result (if available)
    pub review_result: Option<ReviewResult>,
    /// PR merge status (if PR exists)
    pub pr_status: Option<PrMergeStatus>,
    /// Overall CI status
    pub ci_status: CiStatus,
    /// Build status
    pub build_status: CiStatus,
    /// Lint status
    pub lint_status: CiStatus,
    /// Test status
    pub test_status: CiStatus,
    /// Generated feedback for agent continuation
    pub feedback: Option<String>,
    /// Detailed feedback items
    pub feedback_items: Vec<FeedbackItem>,
    /// Evaluation timestamp
    pub evaluated_at: DateTime<Utc>,
    /// Story ID being evaluated
    pub story_id: Option<String>,
    /// Agent ID being evaluated
    pub agent_id: Option<String>,
}

impl WorkEvaluationResult {
    pub fn new(status: WorkCompletionStatus) -> Self {
        Self {
            status,
            agent_status: None,
            criteria_checks: Vec::new(),
            ci_checks: Vec::new(),
            review_result: None,
            pr_status: None,
            ci_status: CiStatus::Pending,
            build_status: CiStatus::Pending,
            lint_status: CiStatus::Pending,
            test_status: CiStatus::Pending,
            feedback: None,
            feedback_items: Vec::new(),
            evaluated_at: Utc::now(),
            story_id: None,
            agent_id: None,
        }
    }

    /// Check if all acceptance criteria are met
    pub fn all_criteria_met(&self) -> bool {
        !self.criteria_checks.is_empty() && self.criteria_checks.iter().all(|c| c.is_met)
    }

    /// Get percentage of criteria met
    pub fn criteria_met_percentage(&self) -> f64 {
        if self.criteria_checks.is_empty() {
            return 0.0;
        }
        let met = self.criteria_checks.iter().filter(|c| c.is_met).count();
        (met as f64 / self.criteria_checks.len() as f64) * 100.0
    }

    /// Check if all CI checks pass (or are not configured/pending)
    pub fn all_ci_passing(&self) -> bool {
        // A CI check is considered "passing" if it either explicitly passed
        // or is not configured (pending). We only fail if any check explicitly failed.
        let build_ok = !matches!(self.build_status, CiStatus::Failed | CiStatus::Timeout | CiStatus::Cancelled);
        let lint_ok = !matches!(self.lint_status, CiStatus::Failed | CiStatus::Timeout | CiStatus::Cancelled);
        let test_ok = !matches!(self.test_status, CiStatus::Failed | CiStatus::Timeout | CiStatus::Cancelled);

        // At least one check should be explicitly passing if we have any checks
        let any_running = matches!(self.build_status, CiStatus::Running)
            || matches!(self.lint_status, CiStatus::Running)
            || matches!(self.test_status, CiStatus::Running);

        // If still running, not yet passing
        if any_running {
            return false;
        }

        build_ok && lint_ok && test_ok
    }

    /// Check if review is approved
    pub fn review_approved(&self) -> bool {
        self.review_result
            .as_ref()
            .map(|r| r.verdict.is_passing() && !r.has_blocking_issues())
            .unwrap_or(false)
    }

    /// Check if PR is ready to merge
    pub fn pr_ready(&self) -> bool {
        self.pr_status
            .as_ref()
            .map(|s| s.can_merge())
            .unwrap_or(false)
    }

    /// Check if agent reported blocked status
    pub fn is_blocked(&self) -> bool {
        matches!(self.agent_status, Some(AgentStatus::Blocked))
    }

    /// Generate a summary of what's incomplete
    pub fn incomplete_summary(&self) -> Vec<String> {
        let mut items = Vec::new();

        if !self.all_criteria_met() {
            let unmet: Vec<_> = self
                .criteria_checks
                .iter()
                .filter(|c| !c.is_met)
                .map(|c| c.criterion.clone())
                .collect();
            if !unmet.is_empty() {
                items.push(format!("Unmet criteria: {}", unmet.join(", ")));
            }
        }

        if !self.build_status.is_passing() {
            items.push(format!("Build: {}", self.build_status.as_str()));
        }

        if !self.lint_status.is_passing() {
            items.push(format!("Lint: {}", self.lint_status.as_str()));
        }

        if !self.test_status.is_passing() {
            items.push(format!("Tests: {}", self.test_status.as_str()));
        }

        if let Some(review) = &self.review_result {
            if !review.verdict.is_passing() {
                items.push(format!("Review: {}", review.verdict.as_str()));
            }
            let blocking: Vec<_> = review
                .issues
                .iter()
                .filter(|i| i.severity.blocks_merge())
                .map(|i| i.description.clone())
                .collect();
            if !blocking.is_empty() {
                items.push(format!("Blocking issues: {}", blocking.join("; ")));
            }
        }

        if let Some(pr) = &self.pr_status {
            if !pr.can_merge() {
                items.push(format!("PR status: {}", pr.as_str()));
            }
        }

        items
    }
}

/// A feedback item for agent continuation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackItem {
    /// Type of feedback
    pub feedback_type: FeedbackType,
    /// Feedback message
    pub message: String,
    /// Priority (higher = more important)
    pub priority: u8,
    /// Actionable suggestion
    pub action: Option<String>,
}

impl FeedbackItem {
    pub fn new(feedback_type: FeedbackType, message: impl Into<String>) -> Self {
        Self {
            feedback_type,
            message: message.into(),
            priority: 50,
            action: None,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_action(mut self, action: impl Into<String>) -> Self {
        self.action = Some(action.into());
        self
    }
}

/// Type of feedback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FeedbackType {
    /// Missing acceptance criterion
    MissingCriterion,
    /// Test failure
    TestFailure,
    /// Build failure
    BuildFailure,
    /// Lint issue
    LintIssue,
    /// Review issue
    ReviewIssue,
    /// Merge conflict
    MergeConflict,
    /// General suggestion
    Suggestion,
    /// Blocker
    Blocker,
}

impl FeedbackType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MissingCriterion => "missing_criterion",
            Self::TestFailure => "test_failure",
            Self::BuildFailure => "build_failure",
            Self::LintIssue => "lint_issue",
            Self::ReviewIssue => "review_issue",
            Self::MergeConflict => "merge_conflict",
            Self::Suggestion => "suggestion",
            Self::Blocker => "blocker",
        }
    }
}

/// Configuration for work evaluator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkEvaluatorConfig {
    /// Require all CI checks to pass
    pub require_ci_pass: bool,
    /// Require code review approval
    pub require_review_approval: bool,
    /// Allow merge with only low/nitpick issues
    pub allow_merge_with_minor_issues: bool,
    /// Maximum review iterations before escalation
    pub max_review_iterations: u32,
    /// Minimum confidence for criterion check
    pub min_criterion_confidence: f64,
}

impl Default for WorkEvaluatorConfig {
    fn default() -> Self {
        Self {
            require_ci_pass: true,
            require_review_approval: true,
            allow_merge_with_minor_issues: true,
            max_review_iterations: 3,
            min_criterion_confidence: 0.5,
        }
    }
}

/// Work evaluator
#[derive(Debug, Clone)]
pub struct WorkEvaluator {
    config: WorkEvaluatorConfig,
}

impl WorkEvaluator {
    pub fn new() -> Self {
        Self {
            config: WorkEvaluatorConfig::default(),
        }
    }

    pub fn with_config(config: WorkEvaluatorConfig) -> Self {
        Self { config }
    }

    /// Evaluate work completion from various inputs
    pub fn evaluate(
        &self,
        agent_status: Option<AgentStatus>,
        criteria_checks: Vec<CriterionCheck>,
        ci_checks: Vec<CiCheckResult>,
        review_result: Option<ReviewResult>,
        pr_status: Option<PrMergeStatus>,
    ) -> WorkEvaluationResult {
        // Extract CI statuses
        let build_status = self.extract_ci_status(&ci_checks, &["build", "compile"]);
        let lint_status = self.extract_ci_status(&ci_checks, &["lint", "clippy", "eslint", "fmt", "format"]);
        let test_status = self.extract_ci_status(&ci_checks, &["test", "tests", "pytest", "cargo-test"]);
        let ci_status = self.aggregate_ci_status(&ci_checks);

        // Determine overall status
        let status = self.determine_status(
            agent_status,
            &criteria_checks,
            ci_status,
            build_status,
            lint_status,
            test_status,
            review_result.as_ref(),
            pr_status,
        );

        // Generate feedback
        let (feedback, feedback_items) = self.generate_feedback(
            &status,
            &criteria_checks,
            &ci_checks,
            review_result.as_ref(),
            pr_status,
        );

        WorkEvaluationResult {
            status,
            agent_status,
            criteria_checks,
            ci_checks,
            review_result,
            pr_status,
            ci_status,
            build_status,
            lint_status,
            test_status,
            feedback: Some(feedback),
            feedback_items,
            evaluated_at: Utc::now(),
            story_id: None,
            agent_id: None,
        }
    }

    /// Extract CI status for a specific check type
    fn extract_ci_status(&self, checks: &[CiCheckResult], names: &[&str]) -> CiStatus {
        for check in checks {
            let check_name_lower = check.name.to_lowercase();
            for name in names {
                if check_name_lower.contains(name) {
                    return check.status;
                }
            }
        }
        CiStatus::Pending
    }

    /// Aggregate overall CI status from all checks
    fn aggregate_ci_status(&self, checks: &[CiCheckResult]) -> CiStatus {
        if checks.is_empty() {
            return CiStatus::Pending;
        }

        let has_failed = checks.iter().any(|c| matches!(c.status, CiStatus::Failed));
        let has_running = checks.iter().any(|c| matches!(c.status, CiStatus::Running));
        let all_passed = checks.iter().all(|c| c.status.is_passing());

        if has_failed {
            CiStatus::Failed
        } else if has_running {
            CiStatus::Running
        } else if all_passed {
            CiStatus::Passed
        } else {
            CiStatus::Pending
        }
    }

    /// Determine overall work completion status
    #[allow(clippy::too_many_arguments)]
    fn determine_status(
        &self,
        agent_status: Option<AgentStatus>,
        criteria_checks: &[CriterionCheck],
        ci_status: CiStatus,
        build_status: CiStatus,
        lint_status: CiStatus,
        test_status: CiStatus,
        review_result: Option<&ReviewResult>,
        pr_status: Option<PrMergeStatus>,
    ) -> WorkCompletionStatus {
        // Check for blocked agent
        if matches!(agent_status, Some(AgentStatus::Blocked)) {
            return WorkCompletionStatus::Blocked;
        }

        // Check for error
        if matches!(agent_status, Some(AgentStatus::Error)) {
            return WorkCompletionStatus::Failed;
        }

        // Check CI status
        if self.config.require_ci_pass {
            if matches!(build_status, CiStatus::Failed) {
                return WorkCompletionStatus::NeedsCiFixes;
            }
            if matches!(lint_status, CiStatus::Failed) {
                return WorkCompletionStatus::NeedsCiFixes;
            }
            if matches!(test_status, CiStatus::Failed) {
                return WorkCompletionStatus::NeedsCiFixes;
            }
        }

        // Check review status
        if self.config.require_review_approval {
            if let Some(review) = review_result {
                if review.has_blocking_issues() {
                    return WorkCompletionStatus::NeedsReviewFixes;
                }
                if !review.verdict.is_passing() {
                    if matches!(review.verdict, ReviewVerdict::ChangesRequested) {
                        return WorkCompletionStatus::NeedsReviewFixes;
                    }
                    return WorkCompletionStatus::NeedsReview;
                }
            } else {
                // No review yet but agent says complete
                if matches!(agent_status, Some(AgentStatus::Complete)) {
                    return WorkCompletionStatus::NeedsReview;
                }
            }
        }

        // Check PR status
        if let Some(status) = pr_status {
            match status {
                PrMergeStatus::Conflicts => return WorkCompletionStatus::Blocked,
                PrMergeStatus::Blocked => return WorkCompletionStatus::NeedsPrApproval,
                PrMergeStatus::Mergeable => {
                    // All checks passed, ready to merge
                    if ci_status.is_passing() {
                        return WorkCompletionStatus::ReadyToMerge;
                    }
                }
                _ => {}
            }
        }

        // Check criteria
        let all_criteria_met =
            !criteria_checks.is_empty() && criteria_checks.iter().all(|c| c.is_met);

        // Check if agent reported complete
        if matches!(agent_status, Some(AgentStatus::Complete)) {
            let ci_ok = !self.config.require_ci_pass || ci_status.is_passing() || matches!(ci_status, CiStatus::Pending | CiStatus::Running);

            if all_criteria_met && ci_ok {
                if review_result.is_none() && self.config.require_review_approval {
                    return WorkCompletionStatus::NeedsReview;
                }
                return WorkCompletionStatus::Complete;
            }
            // Agent says complete but CI checks don't pass (and CI is required)
            if self.config.require_ci_pass && !ci_status.is_passing() && !matches!(ci_status, CiStatus::Pending | CiStatus::Running) {
                return WorkCompletionStatus::NeedsCiFixes;
            }
        }

        // Default to in progress
        WorkCompletionStatus::InProgress
    }

    /// Generate feedback for agent continuation
    fn generate_feedback(
        &self,
        status: &WorkCompletionStatus,
        criteria_checks: &[CriterionCheck],
        ci_checks: &[CiCheckResult],
        review_result: Option<&ReviewResult>,
        pr_status: Option<PrMergeStatus>,
    ) -> (String, Vec<FeedbackItem>) {
        let mut items = Vec::new();
        let mut messages = Vec::new();

        // Add feedback based on status
        match status {
            WorkCompletionStatus::Complete | WorkCompletionStatus::ReadyToMerge => {
                messages.push("Work is complete and ready.".to_string());
            }
            WorkCompletionStatus::InProgress => {
                messages.push("Work is still in progress.".to_string());
            }
            WorkCompletionStatus::Blocked => {
                messages.push("Work is blocked and needs intervention.".to_string());
                items.push(FeedbackItem::new(FeedbackType::Blocker, "Agent is blocked"));
            }
            WorkCompletionStatus::Failed => {
                messages.push("Work has failed.".to_string());
                items.push(FeedbackItem::new(FeedbackType::Blocker, "Agent reported error"));
            }
            _ => {}
        }

        // Add unmet criteria feedback
        for check in criteria_checks {
            if !check.is_met {
                let msg = format!("Criterion not met: {}", check.criterion);
                messages.push(msg.clone());
                items.push(
                    FeedbackItem::new(FeedbackType::MissingCriterion, msg)
                        .with_priority(80)
                        .with_action(format!("Implement: {}", check.criterion)),
                );
            }
        }

        // Add CI failure feedback
        for check in ci_checks {
            if matches!(check.status, CiStatus::Failed) {
                let feedback_type = if check.name.to_lowercase().contains("test") {
                    FeedbackType::TestFailure
                } else if check.name.to_lowercase().contains("build") {
                    FeedbackType::BuildFailure
                } else if check.name.to_lowercase().contains("lint") {
                    FeedbackType::LintIssue
                } else {
                    FeedbackType::BuildFailure
                };

                let msg = if let Some(details) = &check.failure_details {
                    format!("{} failed: {}", check.name, details)
                } else {
                    format!("{} failed", check.name)
                };
                messages.push(msg.clone());
                items.push(
                    FeedbackItem::new(feedback_type, msg)
                        .with_priority(90)
                        .with_action(format!("Fix {} failures", check.name)),
                );
            }
        }

        // Add review feedback
        if let Some(review) = review_result {
            for issue in &review.issues {
                if issue.severity.blocks_merge() {
                    let msg = format!("[{}] {}", issue.severity.as_str().to_uppercase(), issue.description);
                    messages.push(msg.clone());
                    let mut item = FeedbackItem::new(FeedbackType::ReviewIssue, msg).with_priority(85);
                    if let Some(suggestion) = &issue.suggestion {
                        item = item.with_action(suggestion.clone());
                    }
                    items.push(item);
                }
            }

            if matches!(review.verdict, ReviewVerdict::ChangesRequested) {
                messages.push("Code review requested changes.".to_string());
            }
        }

        // Add PR status feedback
        if let Some(pr) = pr_status {
            if matches!(pr, PrMergeStatus::Conflicts) {
                let msg = "PR has merge conflicts that need resolution.".to_string();
                messages.push(msg.clone());
                items.push(
                    FeedbackItem::new(FeedbackType::MergeConflict, msg)
                        .with_priority(95)
                        .with_action("Resolve merge conflicts"),
                );
            }
        }

        // Sort items by priority
        items.sort_by(|a, b| b.priority.cmp(&a.priority));

        let feedback = if messages.is_empty() {
            "No feedback available.".to_string()
        } else {
            messages.join("\n")
        };

        (feedback, items)
    }

    /// Parse review output for machine-readable verdict and issues
    pub fn parse_review_output(&self, output: &str) -> ReviewResult {
        let verdict = self.extract_review_verdict(output);
        let issues = self.extract_review_issues(output);

        ReviewResult::new(verdict).with_issues(issues)
    }

    /// Extract review verdict from output
    fn extract_review_verdict(&self, output: &str) -> ReviewVerdict {
        let output_lower = output.to_lowercase();

        // Look for explicit verdict markers
        let verdict_patterns = [
            (r"(?i)\*\*verdict\*\*:\s*(\w+)", 1),
            (r"(?i)verdict:\s*(\w+)", 1),
            (r"(?i)review\s+status:\s*(\w+)", 1),
            (r"(?i)overall:\s*(\w+)", 1),
        ];

        for (pattern, group) in verdict_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                if let Some(captures) = re.captures(output) {
                    if let Some(verdict_match) = captures.get(group) {
                        if let Ok(verdict) = verdict_match.as_str().parse() {
                            return verdict;
                        }
                    }
                }
            }
        }

        // Infer from content
        if output_lower.contains("approved") || output_lower.contains("lgtm") {
            ReviewVerdict::Approved
        } else if output_lower.contains("changes requested")
            || output_lower.contains("request changes")
            || output_lower.contains("needs changes")
        {
            ReviewVerdict::ChangesRequested
        } else if output_lower.contains("needs discussion")
            || output_lower.contains("discuss")
        {
            ReviewVerdict::NeedsDiscussion
        } else {
            ReviewVerdict::Pending
        }
    }

    /// Extract review issues from output
    fn extract_review_issues(&self, output: &str) -> Vec<ReviewIssue> {
        let mut issues = Vec::new();
        let mut matched_ranges: Vec<(usize, usize)> = Vec::new();

        // First, look for inline comments with file:line format (more specific pattern)
        let location_pattern = r"(?mi)^([^\s:]+\.[a-zA-Z]+):(\d+):\s*\[?(critical|high|medium|low|nitpick|nit)\]?\s*[-:]?\s*(.+?)$";

        if let Ok(re) = regex::Regex::new(location_pattern) {
            for captures in re.captures_iter(output) {
                if let (Some(file), Some(line), Some(severity_match), Some(desc)) = (
                    captures.get(1),
                    captures.get(2),
                    captures.get(3),
                    captures.get(4),
                ) {
                    if let (Ok(severity), Ok(line_num)) =
                        (severity_match.as_str().parse(), line.as_str().parse())
                    {
                        let description = desc.as_str().trim().to_string();
                        if !description.is_empty() {
                            issues.push(
                                ReviewIssue::new(severity, description)
                                    .with_location(file.as_str(), line_num),
                            );
                            // Track the range so we don't double-match
                            if let Some(m) = captures.get(0) {
                                matched_ranges.push((m.start(), m.end()));
                            }
                        }
                    }
                }
            }
        }

        // Then look for standalone issue patterns like "[CRITICAL]", "[HIGH]", etc.
        let issue_pattern =
            r"(?mi)^\s*\[?(critical|high|medium|low|nitpick|nit)\]?\s*[-:]?\s*(.+?)$";

        if let Ok(re) = regex::Regex::new(issue_pattern) {
            for captures in re.captures_iter(output) {
                // Skip if this range was already matched by location pattern
                if let Some(m) = captures.get(0) {
                    let is_already_matched = matched_ranges.iter().any(|(start, end)| {
                        m.start() >= *start && m.end() <= *end
                    });
                    if is_already_matched {
                        continue;
                    }
                }

                if let (Some(severity_match), Some(desc_match)) =
                    (captures.get(1), captures.get(2))
                {
                    if let Ok(severity) = severity_match.as_str().parse() {
                        let description = desc_match.as_str().trim().to_string();
                        // Skip if description looks like a file path (already handled)
                        if !description.is_empty() && !description.contains(':') {
                            issues.push(ReviewIssue::new(severity, description));
                        }
                    }
                }
            }
        }

        issues
    }
}

impl Default for WorkEvaluator {
    fn default() -> Self {
        Self::new()
    }
}

/// Story evaluation record for tracking history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoryEvaluationRecord {
    pub id: i64,
    pub story_id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub status: WorkCompletionStatus,
    pub criteria_met_count: u32,
    pub criteria_total_count: u32,
    pub ci_passed: bool,
    pub review_passed: bool,
    pub review_iteration: u32,
    pub pr_mergeable: bool,
    pub feedback: Option<String>,
    pub details: serde_json::Value,
    pub evaluated_at: DateTime<Utc>,
}

impl StoryEvaluationRecord {
    pub fn from_result(
        story_id: impl Into<String>,
        agent_id: impl Into<String>,
        result: &WorkEvaluationResult,
    ) -> Self {
        let criteria_met = result.criteria_checks.iter().filter(|c| c.is_met).count() as u32;
        let criteria_total = result.criteria_checks.len() as u32;

        Self {
            id: 0,
            story_id: story_id.into(),
            agent_id: agent_id.into(),
            session_id: None,
            status: result.status,
            criteria_met_count: criteria_met,
            criteria_total_count: criteria_total,
            ci_passed: result.all_ci_passing(),
            review_passed: result.review_approved(),
            review_iteration: result
                .review_result
                .as_ref()
                .map(|r| r.iteration)
                .unwrap_or(0),
            pr_mergeable: result.pr_ready(),
            feedback: result.feedback.clone(),
            details: serde_json::to_value(result).unwrap_or(serde_json::json!({})),
            evaluated_at: result.evaluated_at,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ReviewVerdict Tests ====================

    #[test]
    fn test_review_verdict_roundtrip() {
        let verdicts = [
            ReviewVerdict::Approved,
            ReviewVerdict::ChangesRequested,
            ReviewVerdict::NeedsDiscussion,
            ReviewVerdict::Pending,
        ];

        for v in verdicts {
            let s = v.as_str();
            let parsed: ReviewVerdict = s.parse().unwrap();
            assert_eq!(v, parsed);
        }
    }

    #[test]
    fn test_review_verdict_is_passing() {
        assert!(ReviewVerdict::Approved.is_passing());
        assert!(!ReviewVerdict::ChangesRequested.is_passing());
        assert!(!ReviewVerdict::NeedsDiscussion.is_passing());
        assert!(!ReviewVerdict::Pending.is_passing());
    }

    // ==================== ReviewIssueSeverity Tests ====================

    #[test]
    fn test_review_issue_severity_ordering() {
        assert!(ReviewIssueSeverity::Nitpick < ReviewIssueSeverity::Low);
        assert!(ReviewIssueSeverity::Low < ReviewIssueSeverity::Medium);
        assert!(ReviewIssueSeverity::Medium < ReviewIssueSeverity::High);
        assert!(ReviewIssueSeverity::High < ReviewIssueSeverity::Critical);
    }

    #[test]
    fn test_review_issue_severity_blocks_merge() {
        assert!(!ReviewIssueSeverity::Nitpick.blocks_merge());
        assert!(!ReviewIssueSeverity::Low.blocks_merge());
        assert!(!ReviewIssueSeverity::Medium.blocks_merge());
        assert!(ReviewIssueSeverity::High.blocks_merge());
        assert!(ReviewIssueSeverity::Critical.blocks_merge());
    }

    // ==================== ReviewResult Tests ====================

    #[test]
    fn test_review_result_has_blocking_issues() {
        let mut result = ReviewResult::new(ReviewVerdict::ChangesRequested);

        // No issues
        assert!(!result.has_blocking_issues());

        // Add non-blocking issue
        result.issues.push(ReviewIssue::new(
            ReviewIssueSeverity::Low,
            "Minor formatting",
        ));
        assert!(!result.has_blocking_issues());

        // Add blocking issue
        result.issues.push(ReviewIssue::new(
            ReviewIssueSeverity::High,
            "Security vulnerability",
        ));
        assert!(result.has_blocking_issues());
    }

    #[test]
    fn test_review_result_issue_counts() {
        let mut result = ReviewResult::new(ReviewVerdict::ChangesRequested);
        result.issues.push(ReviewIssue::new(ReviewIssueSeverity::Low, "Issue 1"));
        result.issues.push(ReviewIssue::new(ReviewIssueSeverity::Low, "Issue 2"));
        result.issues.push(ReviewIssue::new(ReviewIssueSeverity::High, "Issue 3"));

        let counts = result.issue_counts();
        assert_eq!(counts.get(&ReviewIssueSeverity::Low), Some(&2));
        assert_eq!(counts.get(&ReviewIssueSeverity::High), Some(&1));
    }

    // ==================== CiStatus Tests ====================

    #[test]
    fn test_ci_status_roundtrip() {
        let statuses = [
            CiStatus::Running,
            CiStatus::Passed,
            CiStatus::Failed,
            CiStatus::Cancelled,
            CiStatus::Timeout,
            CiStatus::Pending,
        ];

        for s in statuses {
            let str = s.as_str();
            let parsed: CiStatus = str.parse().unwrap();
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn test_ci_status_is_passing() {
        assert!(CiStatus::Passed.is_passing());
        assert!(!CiStatus::Failed.is_passing());
        assert!(!CiStatus::Running.is_passing());
    }

    #[test]
    fn test_ci_status_is_terminal() {
        assert!(CiStatus::Passed.is_terminal());
        assert!(CiStatus::Failed.is_terminal());
        assert!(CiStatus::Cancelled.is_terminal());
        assert!(CiStatus::Timeout.is_terminal());
        assert!(!CiStatus::Running.is_terminal());
        assert!(!CiStatus::Pending.is_terminal());
    }

    // ==================== WorkCompletionStatus Tests ====================

    #[test]
    fn test_work_completion_status_roundtrip() {
        let statuses = [
            WorkCompletionStatus::Complete,
            WorkCompletionStatus::InProgress,
            WorkCompletionStatus::Blocked,
            WorkCompletionStatus::Failed,
            WorkCompletionStatus::NeedsReview,
            WorkCompletionStatus::NeedsReviewFixes,
            WorkCompletionStatus::NeedsCiFixes,
            WorkCompletionStatus::NeedsPrApproval,
            WorkCompletionStatus::ReadyToMerge,
        ];

        for s in statuses {
            let str = s.as_str();
            let parsed: WorkCompletionStatus = str.parse().unwrap();
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn test_work_completion_status_is_complete() {
        assert!(WorkCompletionStatus::Complete.is_complete());
        assert!(WorkCompletionStatus::ReadyToMerge.is_complete());
        assert!(!WorkCompletionStatus::InProgress.is_complete());
        assert!(!WorkCompletionStatus::Blocked.is_complete());
    }

    // ==================== WorkEvaluator Tests ====================

    #[test]
    fn test_evaluator_complete_work() {
        let evaluator = WorkEvaluator::new();

        let result = evaluator.evaluate(
            Some(AgentStatus::Complete),
            vec![CriterionCheck::met("Implement feature")],
            vec![
                CiCheckResult::new("build", CiStatus::Passed),
                CiCheckResult::new("test", CiStatus::Passed),
                CiCheckResult::new("lint", CiStatus::Passed),
            ],
            Some(ReviewResult::new(ReviewVerdict::Approved)),
            Some(PrMergeStatus::Mergeable),
        );

        assert_eq!(result.status, WorkCompletionStatus::ReadyToMerge);
        assert!(result.all_criteria_met());
        assert!(result.all_ci_passing());
        assert!(result.review_approved());
    }

    #[test]
    fn test_evaluator_needs_review() {
        let evaluator = WorkEvaluator::new();

        let result = evaluator.evaluate(
            Some(AgentStatus::Complete),
            vec![CriterionCheck::met("Implement feature")],
            vec![
                CiCheckResult::new("build", CiStatus::Passed),
                CiCheckResult::new("test", CiStatus::Passed),
            ],
            None, // No review
            None,
        );

        assert_eq!(result.status, WorkCompletionStatus::NeedsReview);
    }

    #[test]
    fn test_evaluator_needs_ci_fixes() {
        let evaluator = WorkEvaluator::new();

        let result = evaluator.evaluate(
            Some(AgentStatus::Complete),
            vec![CriterionCheck::met("Implement feature")],
            vec![
                CiCheckResult::new("build", CiStatus::Passed),
                CiCheckResult::new("test", CiStatus::Failed)
                    .with_failure("3 tests failed"),
            ],
            None,
            None,
        );

        assert_eq!(result.status, WorkCompletionStatus::NeedsCiFixes);
        assert!(!result.test_status.is_passing());
    }

    #[test]
    fn test_evaluator_needs_review_fixes() {
        let evaluator = WorkEvaluator::new();

        let review = ReviewResult::new(ReviewVerdict::ChangesRequested).with_issues(vec![
            ReviewIssue::new(ReviewIssueSeverity::High, "Security issue found"),
        ]);

        let result = evaluator.evaluate(
            Some(AgentStatus::Complete),
            vec![CriterionCheck::met("Implement feature")],
            vec![CiCheckResult::new("build", CiStatus::Passed)],
            Some(review),
            None,
        );

        assert_eq!(result.status, WorkCompletionStatus::NeedsReviewFixes);
    }

    #[test]
    fn test_evaluator_blocked() {
        let evaluator = WorkEvaluator::new();

        let result = evaluator.evaluate(
            Some(AgentStatus::Blocked),
            vec![],
            vec![],
            None,
            None,
        );

        assert_eq!(result.status, WorkCompletionStatus::Blocked);
        assert!(result.is_blocked());
    }

    #[test]
    fn test_evaluator_merge_conflict() {
        let evaluator = WorkEvaluator::new();

        let result = evaluator.evaluate(
            Some(AgentStatus::Complete),
            vec![CriterionCheck::met("Implement feature")],
            vec![CiCheckResult::new("build", CiStatus::Passed)],
            Some(ReviewResult::new(ReviewVerdict::Approved)),
            Some(PrMergeStatus::Conflicts),
        );

        assert_eq!(result.status, WorkCompletionStatus::Blocked);
    }

    #[test]
    fn test_evaluator_feedback_generation() {
        let evaluator = WorkEvaluator::new();

        let result = evaluator.evaluate(
            Some(AgentStatus::Complete),
            vec![
                CriterionCheck::met("Implement feature A"),
                CriterionCheck::unmet("Add tests for feature A"),
            ],
            vec![
                CiCheckResult::new("build", CiStatus::Passed),
                CiCheckResult::new("test", CiStatus::Failed)
                    .with_failure("2 tests failed"),
            ],
            Some(
                ReviewResult::new(ReviewVerdict::ChangesRequested).with_issues(vec![
                    ReviewIssue::new(ReviewIssueSeverity::High, "Missing error handling"),
                ]),
            ),
            None,
        );

        assert!(!result.feedback_items.is_empty());

        // Should have feedback for unmet criterion
        let has_criterion_feedback = result
            .feedback_items
            .iter()
            .any(|f| matches!(f.feedback_type, FeedbackType::MissingCriterion));
        assert!(has_criterion_feedback);

        // Should have feedback for test failure
        let has_test_feedback = result
            .feedback_items
            .iter()
            .any(|f| matches!(f.feedback_type, FeedbackType::TestFailure));
        assert!(has_test_feedback);

        // Should have feedback for review issue
        let has_review_feedback = result
            .feedback_items
            .iter()
            .any(|f| matches!(f.feedback_type, FeedbackType::ReviewIssue));
        assert!(has_review_feedback);
    }

    #[test]
    fn test_evaluator_criteria_percentage() {
        let evaluator = WorkEvaluator::new();

        let result = evaluator.evaluate(
            None,
            vec![
                CriterionCheck::met("Criterion 1"),
                CriterionCheck::met("Criterion 2"),
                CriterionCheck::unmet("Criterion 3"),
                CriterionCheck::unmet("Criterion 4"),
            ],
            vec![],
            None,
            None,
        );

        assert_eq!(result.criteria_met_percentage(), 50.0);
    }

    #[test]
    fn test_evaluator_incomplete_summary() {
        let evaluator = WorkEvaluator::new();

        let result = evaluator.evaluate(
            None,
            vec![
                CriterionCheck::met("Done criterion"),
                CriterionCheck::unmet("Not done criterion"),
            ],
            vec![
                CiCheckResult::new("build", CiStatus::Failed),
            ],
            Some(
                ReviewResult::new(ReviewVerdict::ChangesRequested).with_issues(vec![
                    ReviewIssue::new(ReviewIssueSeverity::Critical, "Blocker issue"),
                ]),
            ),
            Some(PrMergeStatus::Conflicts),
        );

        let summary = result.incomplete_summary();
        assert!(!summary.is_empty());

        // Should mention unmet criterion
        assert!(summary.iter().any(|s| s.contains("Not done criterion")));

        // Should mention build
        assert!(summary.iter().any(|s| s.contains("Build")));

        // Should mention blocker
        assert!(summary.iter().any(|s| s.contains("Blocker issue")));

        // Should mention PR conflicts
        assert!(summary.iter().any(|s| s.contains("conflicts")));
    }

    // ==================== Review Parsing Tests ====================

    #[test]
    fn test_parse_review_verdict_approved() {
        let evaluator = WorkEvaluator::new();

        let output = "Code looks good!\n\n**Verdict**: APPROVED\n\nLGTM!";
        let result = evaluator.parse_review_output(output);

        assert_eq!(result.verdict, ReviewVerdict::Approved);
    }

    #[test]
    fn test_parse_review_verdict_changes_requested() {
        let evaluator = WorkEvaluator::new();

        let output = "Found some issues.\n\nVerdict: CHANGES_REQUESTED\n\nPlease fix.";
        let result = evaluator.parse_review_output(output);

        assert_eq!(result.verdict, ReviewVerdict::ChangesRequested);
    }

    #[test]
    fn test_parse_review_issues() {
        let evaluator = WorkEvaluator::new();

        let output = r#"
Review complete.

[CRITICAL] Security vulnerability in auth flow
[HIGH] Missing input validation
[MEDIUM] Consider adding more tests
[LOW] Typo in variable name
[NITPICK] Formatting inconsistency

Verdict: CHANGES_REQUESTED
        "#;

        let result = evaluator.parse_review_output(output);

        assert_eq!(result.verdict, ReviewVerdict::ChangesRequested);
        assert_eq!(result.issues.len(), 5);

        let critical_count = result
            .issues
            .iter()
            .filter(|i| matches!(i.severity, ReviewIssueSeverity::Critical))
            .count();
        assert_eq!(critical_count, 1);

        let blocking_count = result.issues.iter().filter(|i| i.severity.blocks_merge()).count();
        assert_eq!(blocking_count, 2); // CRITICAL and HIGH
    }

    #[test]
    fn test_parse_review_issues_with_location() {
        let evaluator = WorkEvaluator::new();

        let output = r#"
src/lib.rs:42: [HIGH] - Unsafe unwrap call
src/main.rs:100: [MEDIUM] - Consider error handling
        "#;

        let result = evaluator.parse_review_output(output);

        assert!(result.issues.len() >= 2);

        let high_issue = result
            .issues
            .iter()
            .find(|i| matches!(i.severity, ReviewIssueSeverity::High));
        assert!(high_issue.is_some());
        let high = high_issue.unwrap();
        assert_eq!(high.file_path, Some("src/lib.rs".to_string()));
        assert_eq!(high.line_number, Some(42));
    }

    // ==================== StoryEvaluationRecord Tests ====================

    #[test]
    fn test_story_evaluation_record_from_result() {
        let evaluator = WorkEvaluator::new();

        let result = evaluator.evaluate(
            Some(AgentStatus::Complete),
            vec![
                CriterionCheck::met("Criterion 1"),
                CriterionCheck::unmet("Criterion 2"),
            ],
            vec![CiCheckResult::new("build", CiStatus::Passed)],
            Some(ReviewResult::new(ReviewVerdict::Approved).with_iteration(2)),
            Some(PrMergeStatus::Mergeable),
        );

        let record = StoryEvaluationRecord::from_result("story-1", "agent-1", &result);

        assert_eq!(record.story_id, "story-1");
        assert_eq!(record.agent_id, "agent-1");
        assert_eq!(record.criteria_met_count, 1);
        assert_eq!(record.criteria_total_count, 2);
        assert!(record.ci_passed);
        assert!(record.review_passed);
        assert_eq!(record.review_iteration, 2);
        assert!(record.pr_mergeable);
    }

    // ==================== CriterionCheck Tests ====================

    #[test]
    fn test_criterion_check_met() {
        let check = CriterionCheck::met("Implement feature")
            .with_evidence("Feature implemented in src/lib.rs")
            .with_confidence(0.9);

        assert!(check.is_met);
        assert_eq!(check.criterion, "Implement feature");
        assert_eq!(check.evidence, Some("Feature implemented in src/lib.rs".to_string()));
        assert_eq!(check.confidence, 0.9);
    }

    #[test]
    fn test_criterion_check_unmet() {
        let check = CriterionCheck::unmet("Add tests")
            .with_confidence(0.8);

        assert!(!check.is_met);
        assert_eq!(check.confidence, 0.8);
    }

    #[test]
    fn test_criterion_check_confidence_clamped() {
        let check = CriterionCheck::met("Test").with_confidence(1.5);
        assert_eq!(check.confidence, 1.0);

        let check2 = CriterionCheck::met("Test").with_confidence(-0.5);
        assert_eq!(check2.confidence, 0.0);
    }

    // ==================== FeedbackItem Tests ====================

    #[test]
    fn test_feedback_item_new() {
        let item = FeedbackItem::new(FeedbackType::TestFailure, "Tests failed")
            .with_priority(90)
            .with_action("Fix failing tests");

        assert_eq!(item.feedback_type, FeedbackType::TestFailure);
        assert_eq!(item.message, "Tests failed");
        assert_eq!(item.priority, 90);
        assert_eq!(item.action, Some("Fix failing tests".to_string()));
    }

    // ==================== Config Tests ====================

    #[test]
    fn test_evaluator_config_no_review_required() {
        let config = WorkEvaluatorConfig {
            require_review_approval: false,
            ..Default::default()
        };
        let evaluator = WorkEvaluator::with_config(config);

        let result = evaluator.evaluate(
            Some(AgentStatus::Complete),
            vec![CriterionCheck::met("Feature")],
            vec![
                CiCheckResult::new("build", CiStatus::Passed),
                CiCheckResult::new("test", CiStatus::Passed),
                CiCheckResult::new("lint", CiStatus::Passed),
            ],
            None, // No review
            None,
        );

        // Should be complete without review when not required
        assert_eq!(result.status, WorkCompletionStatus::Complete);
    }

    #[test]
    fn test_evaluator_config_no_ci_required() {
        let config = WorkEvaluatorConfig {
            require_ci_pass: false,
            require_review_approval: false,
            ..Default::default()
        };
        let evaluator = WorkEvaluator::with_config(config);

        let result = evaluator.evaluate(
            Some(AgentStatus::Complete),
            vec![CriterionCheck::met("Feature")],
            vec![CiCheckResult::new("build", CiStatus::Failed)],
            None,
            None,
        );

        // Should not be blocked by CI when not required
        assert_ne!(result.status, WorkCompletionStatus::NeedsCiFixes);
    }
}
