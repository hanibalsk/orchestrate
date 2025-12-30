//! PR Workflow Management
//!
//! Epic 016: Autonomous Epic Processing - Story 10
//!
//! Manages the complete PR lifecycle in autonomous mode:
//! - Create PR with structured description
//! - Monitor CI checks
//! - Handle reviews and comments
//! - Manage merge conflicts
//! - Execute merge and cleanup

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::work_evaluation::{CiCheckResult, CiStatus, ReviewVerdict};

/// PR workflow state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrWorkflowState {
    /// PR is being created
    Creating,
    /// Waiting for CI checks
    AwaitingCi,
    /// Waiting for code review
    AwaitingReview,
    /// CI checks failed, fixing
    FixingCi,
    /// Review requested changes, fixing
    FixingReview,
    /// Resolving merge conflicts
    ResolvingConflicts,
    /// Ready to merge
    ReadyToMerge,
    /// Merging PR
    Merging,
    /// Cleaning up after merge
    CleaningUp,
    /// Completed successfully
    Completed,
    /// Failed/blocked
    Failed,
}

impl PrWorkflowState {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Creating => "creating",
            Self::AwaitingCi => "awaiting_ci",
            Self::AwaitingReview => "awaiting_review",
            Self::FixingCi => "fixing_ci",
            Self::FixingReview => "fixing_review",
            Self::ResolvingConflicts => "resolving_conflicts",
            Self::ReadyToMerge => "ready_to_merge",
            Self::Merging => "merging",
            Self::CleaningUp => "cleaning_up",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed)
    }

    pub fn needs_action(&self) -> bool {
        matches!(
            self,
            Self::FixingCi | Self::FixingReview | Self::ResolvingConflicts
        )
    }
}

impl std::str::FromStr for PrWorkflowState {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "creating" => Ok(Self::Creating),
            "awaiting_ci" => Ok(Self::AwaitingCi),
            "awaiting_review" => Ok(Self::AwaitingReview),
            "fixing_ci" => Ok(Self::FixingCi),
            "fixing_review" => Ok(Self::FixingReview),
            "resolving_conflicts" => Ok(Self::ResolvingConflicts),
            "ready_to_merge" => Ok(Self::ReadyToMerge),
            "merging" => Ok(Self::Merging),
            "cleaning_up" => Ok(Self::CleaningUp),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            _ => Err(crate::Error::Other(format!(
                "Invalid PR workflow state: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for PrWorkflowState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Merge strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMethod {
    /// Regular merge commit
    Merge,
    /// Squash and merge
    Squash,
    /// Rebase and merge
    Rebase,
}

impl MergeMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Merge => "merge",
            Self::Squash => "squash",
            Self::Rebase => "rebase",
        }
    }
}

impl std::str::FromStr for MergeMethod {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "merge" => Ok(Self::Merge),
            "squash" => Ok(Self::Squash),
            "rebase" => Ok(Self::Rebase),
            _ => Err(crate::Error::Other(format!(
                "Invalid merge method: {}",
                s
            ))),
        }
    }
}

impl Default for MergeMethod {
    fn default() -> Self {
        Self::Squash
    }
}

/// PR description template data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrDescription {
    /// PR title
    pub title: String,
    /// Summary of changes
    pub summary: String,
    /// List of stories implemented
    pub stories: Vec<String>,
    /// Test plan / verification steps
    pub test_plan: Vec<String>,
    /// Related issues/tickets
    pub related_issues: Vec<String>,
    /// Breaking changes
    pub breaking_changes: Vec<String>,
    /// Files changed (optional, for large PRs)
    pub files_changed: Option<Vec<String>>,
}

impl PrDescription {
    pub fn new(title: impl Into<String>, summary: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            summary: summary.into(),
            stories: Vec::new(),
            test_plan: Vec::new(),
            related_issues: Vec::new(),
            breaking_changes: Vec::new(),
            files_changed: None,
        }
    }

    pub fn with_stories(mut self, stories: Vec<String>) -> Self {
        self.stories = stories;
        self
    }

    pub fn with_test_plan(mut self, plan: Vec<String>) -> Self {
        self.test_plan = plan;
        self
    }

    pub fn with_related_issues(mut self, issues: Vec<String>) -> Self {
        self.related_issues = issues;
        self
    }

    pub fn with_breaking_changes(mut self, changes: Vec<String>) -> Self {
        self.breaking_changes = changes;
        self
    }

    /// Generate markdown body for PR
    pub fn to_markdown(&self) -> String {
        let mut parts = Vec::new();

        // Summary
        parts.push(format!("## Summary\n\n{}", self.summary));

        // Stories
        if !self.stories.is_empty() {
            parts.push("\n## Stories Implemented".to_string());
            for story in &self.stories {
                parts.push(format!("- {story}"));
            }
        }

        // Breaking changes
        if !self.breaking_changes.is_empty() {
            parts.push("\n## Breaking Changes".to_string());
            for change in &self.breaking_changes {
                parts.push(format!("- {change}"));
            }
        }

        // Test plan
        if !self.test_plan.is_empty() {
            parts.push("\n## Test Plan".to_string());
            for step in &self.test_plan {
                parts.push(format!("- [ ] {step}"));
            }
        }

        // Related issues
        if !self.related_issues.is_empty() {
            parts.push("\n## Related Issues".to_string());
            for issue in &self.related_issues {
                parts.push(format!("- {issue}"));
            }
        }

        parts.join("\n")
    }
}

/// CI check aggregate status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiAggregateStatus {
    /// Total number of checks
    pub total: u32,
    /// Checks still running
    pub running: u32,
    /// Checks passed
    pub passed: u32,
    /// Checks failed
    pub failed: u32,
    /// Checks pending
    pub pending: u32,
    /// Overall status
    pub overall: CiStatus,
    /// Failed check details
    pub failures: Vec<CiCheckResult>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl CiAggregateStatus {
    pub fn from_checks(checks: &[CiCheckResult]) -> Self {
        let total = checks.len() as u32;
        let running = checks
            .iter()
            .filter(|c| matches!(c.status, CiStatus::Running))
            .count() as u32;
        let passed = checks
            .iter()
            .filter(|c| matches!(c.status, CiStatus::Passed))
            .count() as u32;
        let failed = checks
            .iter()
            .filter(|c| matches!(c.status, CiStatus::Failed | CiStatus::Timeout))
            .count() as u32;
        let pending = checks
            .iter()
            .filter(|c| matches!(c.status, CiStatus::Pending))
            .count() as u32;

        let overall = if failed > 0 {
            CiStatus::Failed
        } else if running > 0 {
            CiStatus::Running
        } else if passed == total && total > 0 {
            CiStatus::Passed
        } else {
            CiStatus::Pending
        };

        let failures = checks
            .iter()
            .filter(|c| matches!(c.status, CiStatus::Failed | CiStatus::Timeout))
            .cloned()
            .collect();

        Self {
            total,
            running,
            passed,
            failed,
            pending,
            overall,
            failures,
            updated_at: Utc::now(),
        }
    }

    pub fn is_all_passed(&self) -> bool {
        self.overall.is_passing()
    }

    pub fn is_still_running(&self) -> bool {
        matches!(self.overall, CiStatus::Running)
    }

    pub fn has_failures(&self) -> bool {
        self.failed > 0
    }
}

/// PR workflow context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrWorkflowContext {
    /// PR number
    pub pr_number: u64,
    /// Story ID this PR implements
    pub story_id: String,
    /// Agent handling the PR
    pub agent_id: String,
    /// Session ID
    pub session_id: Option<String>,
    /// Source branch
    pub head_branch: String,
    /// Target branch
    pub base_branch: String,
    /// Current workflow state
    pub state: PrWorkflowState,
    /// CI status
    pub ci_status: Option<CiAggregateStatus>,
    /// Review verdict
    pub review_verdict: Option<ReviewVerdict>,
    /// Review iteration count
    pub review_iterations: u32,
    /// Has merge conflicts
    pub has_conflicts: bool,
    /// Merge method to use
    pub merge_method: MergeMethod,
    /// PR URL
    pub url: Option<String>,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Updated at
    pub updated_at: DateTime<Utc>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
    /// State history
    pub state_history: Vec<PrStateTransition>,
}

impl PrWorkflowContext {
    pub fn new(
        pr_number: u64,
        story_id: impl Into<String>,
        agent_id: impl Into<String>,
        head_branch: impl Into<String>,
        base_branch: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            pr_number,
            story_id: story_id.into(),
            agent_id: agent_id.into(),
            session_id: None,
            head_branch: head_branch.into(),
            base_branch: base_branch.into(),
            state: PrWorkflowState::Creating,
            ci_status: None,
            review_verdict: None,
            review_iterations: 0,
            has_conflicts: false,
            merge_method: MergeMethod::default(),
            url: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            state_history: vec![PrStateTransition {
                from_state: None,
                to_state: PrWorkflowState::Creating,
                reason: "PR created".to_string(),
                timestamp: now,
            }],
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn transition(&mut self, new_state: PrWorkflowState, reason: impl Into<String>) {
        let now = Utc::now();
        self.state_history.push(PrStateTransition {
            from_state: Some(self.state),
            to_state: new_state,
            reason: reason.into(),
            timestamp: now,
        });
        self.state = new_state;
        self.updated_at = now;

        if new_state.is_terminal() {
            self.completed_at = Some(now);
        }
    }

    pub fn update_ci_status(&mut self, checks: &[CiCheckResult]) {
        self.ci_status = Some(CiAggregateStatus::from_checks(checks));
        self.updated_at = Utc::now();
    }

    pub fn update_review(&mut self, verdict: ReviewVerdict, iteration: u32) {
        self.review_verdict = Some(verdict);
        self.review_iterations = iteration;
        self.updated_at = Utc::now();
    }

    pub fn set_has_conflicts(&mut self, has_conflicts: bool) {
        self.has_conflicts = has_conflicts;
        self.updated_at = Utc::now();
    }

    pub fn duration(&self) -> Duration {
        let end = self.completed_at.unwrap_or_else(Utc::now);
        end - self.created_at
    }
}

/// State transition record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrStateTransition {
    pub from_state: Option<PrWorkflowState>,
    pub to_state: PrWorkflowState,
    pub reason: String,
    pub timestamp: DateTime<Utc>,
}

/// Conflict resolution info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictInfo {
    /// Files with conflicts
    pub conflicting_files: Vec<String>,
    /// Detected at
    pub detected_at: DateTime<Utc>,
    /// Resolution attempted
    pub resolution_attempted: bool,
    /// Resolution strategy used
    pub strategy: Option<ConflictResolutionStrategy>,
    /// Resolved at
    pub resolved_at: Option<DateTime<Utc>>,
}

impl ConflictInfo {
    pub fn new(conflicting_files: Vec<String>) -> Self {
        Self {
            conflicting_files,
            detected_at: Utc::now(),
            resolution_attempted: false,
            strategy: None,
            resolved_at: None,
        }
    }

    pub fn mark_resolved(&mut self, strategy: ConflictResolutionStrategy) {
        self.resolution_attempted = true;
        self.strategy = Some(strategy);
        self.resolved_at = Some(Utc::now());
    }
}

/// Strategy for resolving conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictResolutionStrategy {
    /// Rebase on target branch
    Rebase,
    /// Merge target into source
    MergeFrom,
    /// Manual resolution
    Manual,
    /// Accept ours
    AcceptOurs,
    /// Accept theirs
    AcceptTheirs,
}

impl ConflictResolutionStrategy {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rebase => "rebase",
            Self::MergeFrom => "merge_from",
            Self::Manual => "manual",
            Self::AcceptOurs => "accept_ours",
            Self::AcceptTheirs => "accept_theirs",
        }
    }
}

/// Configuration for PR workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrWorkflowConfig {
    /// Default merge method
    pub default_merge_method: MergeMethod,
    /// Timeout for CI checks (seconds)
    pub ci_timeout_secs: u64,
    /// Timeout for human review (seconds)
    pub human_review_timeout_secs: u64,
    /// Auto-merge when ready
    pub auto_merge: bool,
    /// Delete branch after merge
    pub delete_branch_after_merge: bool,
    /// Clean up worktree after merge
    pub cleanup_worktree: bool,
    /// Maximum conflict resolution attempts
    pub max_conflict_resolution_attempts: u32,
    /// Require CI to pass before merge
    pub require_ci_pass: bool,
    /// Require review approval before merge
    pub require_review_approval: bool,
}

impl Default for PrWorkflowConfig {
    fn default() -> Self {
        Self {
            default_merge_method: MergeMethod::Squash,
            ci_timeout_secs: 1800, // 30 minutes
            human_review_timeout_secs: 86400, // 24 hours
            auto_merge: true,
            delete_branch_after_merge: true,
            cleanup_worktree: true,
            max_conflict_resolution_attempts: 3,
            require_ci_pass: true,
            require_review_approval: true,
        }
    }
}

/// PR workflow manager
#[derive(Debug, Clone)]
pub struct PrWorkflowManager {
    config: PrWorkflowConfig,
}

impl PrWorkflowManager {
    pub fn new() -> Self {
        Self {
            config: PrWorkflowConfig::default(),
        }
    }

    pub fn with_config(config: PrWorkflowConfig) -> Self {
        Self { config }
    }

    /// Determine next state based on current context
    pub fn determine_next_state(&self, context: &PrWorkflowContext) -> Option<PrWorkflowState> {
        match context.state {
            PrWorkflowState::Creating => {
                // PR created, wait for CI
                Some(PrWorkflowState::AwaitingCi)
            }
            PrWorkflowState::AwaitingCi => {
                // Check CI status
                if let Some(ci) = &context.ci_status {
                    if ci.has_failures() {
                        return Some(PrWorkflowState::FixingCi);
                    }
                    if ci.is_all_passed() {
                        if self.config.require_review_approval {
                            return Some(PrWorkflowState::AwaitingReview);
                        }
                        return Some(PrWorkflowState::ReadyToMerge);
                    }
                }
                // Still waiting
                None
            }
            PrWorkflowState::AwaitingReview => {
                // Check review status
                if let Some(verdict) = context.review_verdict {
                    match verdict {
                        ReviewVerdict::Approved => {
                            if context.has_conflicts {
                                return Some(PrWorkflowState::ResolvingConflicts);
                            }
                            return Some(PrWorkflowState::ReadyToMerge);
                        }
                        ReviewVerdict::ChangesRequested => {
                            return Some(PrWorkflowState::FixingReview);
                        }
                        ReviewVerdict::NeedsDiscussion | ReviewVerdict::Pending => {
                            // Keep waiting
                        }
                    }
                }
                None
            }
            PrWorkflowState::FixingCi => {
                // After CI fix, check status again
                if let Some(ci) = &context.ci_status {
                    if ci.is_all_passed() {
                        if self.config.require_review_approval
                            && !matches!(context.review_verdict, Some(ReviewVerdict::Approved))
                        {
                            return Some(PrWorkflowState::AwaitingReview);
                        }
                        return Some(PrWorkflowState::ReadyToMerge);
                    }
                }
                // Still fixing
                None
            }
            PrWorkflowState::FixingReview => {
                // After review fix, check review again
                if matches!(context.review_verdict, Some(ReviewVerdict::Approved)) {
                    // Need to re-check CI
                    if let Some(ci) = &context.ci_status {
                        if ci.is_all_passed() {
                            if context.has_conflicts {
                                return Some(PrWorkflowState::ResolvingConflicts);
                            }
                            return Some(PrWorkflowState::ReadyToMerge);
                        }
                    }
                    return Some(PrWorkflowState::AwaitingCi);
                }
                None
            }
            PrWorkflowState::ResolvingConflicts => {
                if !context.has_conflicts {
                    return Some(PrWorkflowState::ReadyToMerge);
                }
                None
            }
            PrWorkflowState::ReadyToMerge => {
                if self.config.auto_merge {
                    return Some(PrWorkflowState::Merging);
                }
                // Wait for manual merge
                None
            }
            PrWorkflowState::Merging => {
                // Merge complete, cleanup
                if self.config.cleanup_worktree || self.config.delete_branch_after_merge {
                    return Some(PrWorkflowState::CleaningUp);
                }
                return Some(PrWorkflowState::Completed);
            }
            PrWorkflowState::CleaningUp => {
                return Some(PrWorkflowState::Completed);
            }
            PrWorkflowState::Completed | PrWorkflowState::Failed => {
                // Terminal states
                None
            }
        }
    }

    /// Check if PR is ready to merge
    pub fn is_ready_to_merge(&self, context: &PrWorkflowContext) -> bool {
        // Must have CI passing if required
        if self.config.require_ci_pass {
            if let Some(ci) = &context.ci_status {
                if !ci.is_all_passed() {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Must have review approval if required
        if self.config.require_review_approval {
            if !matches!(context.review_verdict, Some(ReviewVerdict::Approved)) {
                return false;
            }
        }

        // Must not have conflicts
        if context.has_conflicts {
            return false;
        }

        true
    }

    /// Check if CI has timed out
    pub fn is_ci_timed_out(&self, context: &PrWorkflowContext) -> bool {
        if let Some(ci) = &context.ci_status {
            if ci.is_still_running() {
                let elapsed = (Utc::now() - ci.updated_at).num_seconds() as u64;
                return elapsed > self.config.ci_timeout_secs;
            }
        }
        false
    }

    /// Generate squash commit message
    pub fn generate_squash_message(
        &self,
        context: &PrWorkflowContext,
        description: &PrDescription,
    ) -> String {
        let mut parts = Vec::new();

        // Title
        parts.push(description.title.clone());
        parts.push(String::new());

        // Summary
        parts.push(description.summary.clone());

        // Stories
        if !description.stories.is_empty() {
            parts.push(String::new());
            parts.push("Stories:".to_string());
            for story in &description.stories {
                parts.push(format!("- {story}"));
            }
        }

        // PR reference
        parts.push(String::new());
        if let Some(url) = &context.url {
            parts.push(format!("PR: {url}"));
        } else {
            parts.push(format!("PR: #{}", context.pr_number));
        }

        parts.join("\n")
    }

    /// Get action needed for current state
    pub fn get_needed_action(&self, context: &PrWorkflowContext) -> Option<PrWorkflowAction> {
        match context.state {
            PrWorkflowState::AwaitingCi => Some(PrWorkflowAction::WaitForCi),
            PrWorkflowState::AwaitingReview => Some(PrWorkflowAction::WaitForReview),
            PrWorkflowState::FixingCi => {
                if let Some(ci) = &context.ci_status {
                    if !ci.failures.is_empty() {
                        return Some(PrWorkflowAction::FixCiFailures(
                            ci.failures.iter().map(|f| f.name.clone()).collect(),
                        ));
                    }
                }
                None
            }
            PrWorkflowState::FixingReview => Some(PrWorkflowAction::AddressReviewFeedback),
            PrWorkflowState::ResolvingConflicts => {
                Some(PrWorkflowAction::ResolveConflicts)
            }
            PrWorkflowState::ReadyToMerge => Some(PrWorkflowAction::Merge),
            PrWorkflowState::Merging => Some(PrWorkflowAction::ExecuteMerge),
            PrWorkflowState::CleaningUp => Some(PrWorkflowAction::Cleanup),
            _ => None,
        }
    }
}

impl Default for PrWorkflowManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Actions that can be taken in PR workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PrWorkflowAction {
    /// Wait for CI to complete
    WaitForCi,
    /// Wait for review
    WaitForReview,
    /// Fix CI failures
    FixCiFailures(Vec<String>),
    /// Address review feedback
    AddressReviewFeedback,
    /// Resolve merge conflicts
    ResolveConflicts,
    /// Merge the PR
    Merge,
    /// Execute the merge operation
    ExecuteMerge,
    /// Clean up after merge
    Cleanup,
}

impl PrWorkflowAction {
    pub fn description(&self) -> String {
        match self {
            Self::WaitForCi => "Waiting for CI checks to complete".to_string(),
            Self::WaitForReview => "Waiting for code review".to_string(),
            Self::FixCiFailures(checks) => {
                format!("Fix CI failures: {}", checks.join(", "))
            }
            Self::AddressReviewFeedback => "Address review feedback".to_string(),
            Self::ResolveConflicts => "Resolve merge conflicts".to_string(),
            Self::Merge => "Ready to merge PR".to_string(),
            Self::ExecuteMerge => "Executing merge".to_string(),
            Self::Cleanup => "Cleaning up branches and worktrees".to_string(),
        }
    }
}

/// PR workflow record for database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrWorkflowRecord {
    pub id: i64,
    pub pr_number: u64,
    pub story_id: String,
    pub agent_id: String,
    pub session_id: Option<String>,
    pub head_branch: String,
    pub base_branch: String,
    pub state: PrWorkflowState,
    pub ci_passed: bool,
    pub review_approved: bool,
    pub review_iterations: u32,
    pub has_conflicts: bool,
    pub merge_method: MergeMethod,
    pub url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

impl PrWorkflowRecord {
    pub fn from_context(context: &PrWorkflowContext) -> Self {
        Self {
            id: 0,
            pr_number: context.pr_number,
            story_id: context.story_id.clone(),
            agent_id: context.agent_id.clone(),
            session_id: context.session_id.clone(),
            head_branch: context.head_branch.clone(),
            base_branch: context.base_branch.clone(),
            state: context.state,
            ci_passed: context
                .ci_status
                .as_ref()
                .map(|c| c.is_all_passed())
                .unwrap_or(false),
            review_approved: matches!(context.review_verdict, Some(ReviewVerdict::Approved)),
            review_iterations: context.review_iterations,
            has_conflicts: context.has_conflicts,
            merge_method: context.merge_method,
            url: context.url.clone(),
            created_at: context.created_at,
            updated_at: context.updated_at,
            completed_at: context.completed_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== PrWorkflowState Tests ====================

    #[test]
    fn test_pr_workflow_state_roundtrip() {
        let states = [
            PrWorkflowState::Creating,
            PrWorkflowState::AwaitingCi,
            PrWorkflowState::AwaitingReview,
            PrWorkflowState::FixingCi,
            PrWorkflowState::FixingReview,
            PrWorkflowState::ResolvingConflicts,
            PrWorkflowState::ReadyToMerge,
            PrWorkflowState::Merging,
            PrWorkflowState::CleaningUp,
            PrWorkflowState::Completed,
            PrWorkflowState::Failed,
        ];

        for s in states {
            let str = s.as_str();
            let parsed: PrWorkflowState = str.parse().unwrap();
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn test_pr_workflow_state_is_terminal() {
        assert!(PrWorkflowState::Completed.is_terminal());
        assert!(PrWorkflowState::Failed.is_terminal());
        assert!(!PrWorkflowState::AwaitingCi.is_terminal());
        assert!(!PrWorkflowState::Merging.is_terminal());
    }

    // ==================== PrDescription Tests ====================

    #[test]
    fn test_pr_description_to_markdown() {
        let desc = PrDescription::new("Add feature X", "This PR adds feature X")
            .with_stories(vec!["Story 1".to_string(), "Story 2".to_string()])
            .with_test_plan(vec!["Run tests".to_string(), "Manual verification".to_string()])
            .with_related_issues(vec!["#123".to_string()]);

        let md = desc.to_markdown();

        assert!(md.contains("## Summary"));
        assert!(md.contains("This PR adds feature X"));
        assert!(md.contains("## Stories Implemented"));
        assert!(md.contains("Story 1"));
        assert!(md.contains("## Test Plan"));
        assert!(md.contains("- [ ] Run tests"));
        assert!(md.contains("## Related Issues"));
        assert!(md.contains("#123"));
    }

    // ==================== CiAggregateStatus Tests ====================

    #[test]
    fn test_ci_aggregate_all_passed() {
        let checks = vec![
            CiCheckResult::new("build", CiStatus::Passed),
            CiCheckResult::new("test", CiStatus::Passed),
            CiCheckResult::new("lint", CiStatus::Passed),
        ];

        let agg = CiAggregateStatus::from_checks(&checks);

        assert_eq!(agg.total, 3);
        assert_eq!(agg.passed, 3);
        assert_eq!(agg.failed, 0);
        assert!(agg.is_all_passed());
        assert!(!agg.is_still_running());
    }

    #[test]
    fn test_ci_aggregate_with_failures() {
        let checks = vec![
            CiCheckResult::new("build", CiStatus::Passed),
            CiCheckResult::new("test", CiStatus::Failed),
            CiCheckResult::new("lint", CiStatus::Passed),
        ];

        let agg = CiAggregateStatus::from_checks(&checks);

        assert_eq!(agg.failed, 1);
        assert!(!agg.is_all_passed());
        assert!(agg.has_failures());
        assert_eq!(agg.failures.len(), 1);
    }

    #[test]
    fn test_ci_aggregate_running() {
        let checks = vec![
            CiCheckResult::new("build", CiStatus::Passed),
            CiCheckResult::new("test", CiStatus::Running),
        ];

        let agg = CiAggregateStatus::from_checks(&checks);

        assert_eq!(agg.running, 1);
        assert!(agg.is_still_running());
        assert!(!agg.is_all_passed());
    }

    // ==================== PrWorkflowContext Tests ====================

    #[test]
    fn test_pr_workflow_context_new() {
        let ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");

        assert_eq!(ctx.pr_number, 42);
        assert_eq!(ctx.story_id, "story-1");
        assert_eq!(ctx.state, PrWorkflowState::Creating);
        assert_eq!(ctx.state_history.len(), 1);
    }

    #[test]
    fn test_pr_workflow_context_transition() {
        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");

        ctx.transition(PrWorkflowState::AwaitingCi, "CI started");
        assert_eq!(ctx.state, PrWorkflowState::AwaitingCi);
        assert_eq!(ctx.state_history.len(), 2);

        ctx.transition(PrWorkflowState::Completed, "Merged");
        assert!(ctx.completed_at.is_some());
    }

    #[test]
    fn test_pr_workflow_context_update_ci() {
        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");

        let checks = vec![
            CiCheckResult::new("build", CiStatus::Passed),
            CiCheckResult::new("test", CiStatus::Passed),
        ];
        ctx.update_ci_status(&checks);

        assert!(ctx.ci_status.is_some());
        assert!(ctx.ci_status.as_ref().unwrap().is_all_passed());
    }

    // ==================== PrWorkflowManager Tests ====================

    #[test]
    fn test_determine_next_state_creating() {
        let manager = PrWorkflowManager::new();
        let ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");

        let next = manager.determine_next_state(&ctx);
        assert_eq!(next, Some(PrWorkflowState::AwaitingCi));
    }

    #[test]
    fn test_determine_next_state_ci_passed_needs_review() {
        let manager = PrWorkflowManager::new();
        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");
        ctx.state = PrWorkflowState::AwaitingCi;
        ctx.update_ci_status(&[CiCheckResult::new("build", CiStatus::Passed)]);

        let next = manager.determine_next_state(&ctx);
        assert_eq!(next, Some(PrWorkflowState::AwaitingReview));
    }

    #[test]
    fn test_determine_next_state_ci_failed() {
        let manager = PrWorkflowManager::new();
        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");
        ctx.state = PrWorkflowState::AwaitingCi;
        ctx.update_ci_status(&[CiCheckResult::new("build", CiStatus::Failed)]);

        let next = manager.determine_next_state(&ctx);
        assert_eq!(next, Some(PrWorkflowState::FixingCi));
    }

    #[test]
    fn test_determine_next_state_review_approved() {
        let manager = PrWorkflowManager::new();
        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");
        ctx.state = PrWorkflowState::AwaitingReview;
        ctx.update_review(ReviewVerdict::Approved, 1);

        let next = manager.determine_next_state(&ctx);
        assert_eq!(next, Some(PrWorkflowState::ReadyToMerge));
    }

    #[test]
    fn test_determine_next_state_review_changes_requested() {
        let manager = PrWorkflowManager::new();
        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");
        ctx.state = PrWorkflowState::AwaitingReview;
        ctx.update_review(ReviewVerdict::ChangesRequested, 1);

        let next = manager.determine_next_state(&ctx);
        assert_eq!(next, Some(PrWorkflowState::FixingReview));
    }

    #[test]
    fn test_determine_next_state_ready_to_merge() {
        let manager = PrWorkflowManager::new();
        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");
        ctx.state = PrWorkflowState::ReadyToMerge;

        let next = manager.determine_next_state(&ctx);
        assert_eq!(next, Some(PrWorkflowState::Merging));
    }

    #[test]
    fn test_is_ready_to_merge() {
        let manager = PrWorkflowManager::new();
        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");

        // Not ready - no CI
        assert!(!manager.is_ready_to_merge(&ctx));

        // Add CI passed
        ctx.update_ci_status(&[CiCheckResult::new("build", CiStatus::Passed)]);
        assert!(!manager.is_ready_to_merge(&ctx)); // Still no review

        // Add review approved
        ctx.update_review(ReviewVerdict::Approved, 1);
        assert!(manager.is_ready_to_merge(&ctx));

        // Add conflicts
        ctx.set_has_conflicts(true);
        assert!(!manager.is_ready_to_merge(&ctx));
    }

    #[test]
    fn test_get_needed_action() {
        let manager = PrWorkflowManager::new();
        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");

        ctx.state = PrWorkflowState::AwaitingCi;
        let action = manager.get_needed_action(&ctx);
        assert!(matches!(action, Some(PrWorkflowAction::WaitForCi)));

        ctx.state = PrWorkflowState::FixingReview;
        let action = manager.get_needed_action(&ctx);
        assert!(matches!(action, Some(PrWorkflowAction::AddressReviewFeedback)));

        ctx.state = PrWorkflowState::ReadyToMerge;
        let action = manager.get_needed_action(&ctx);
        assert!(matches!(action, Some(PrWorkflowAction::Merge)));
    }

    // ==================== ConflictInfo Tests ====================

    #[test]
    fn test_conflict_info_new() {
        let info = ConflictInfo::new(vec!["src/lib.rs".to_string()]);

        assert_eq!(info.conflicting_files.len(), 1);
        assert!(!info.resolution_attempted);
        assert!(info.resolved_at.is_none());
    }

    #[test]
    fn test_conflict_info_mark_resolved() {
        let mut info = ConflictInfo::new(vec!["src/lib.rs".to_string()]);
        info.mark_resolved(ConflictResolutionStrategy::Rebase);

        assert!(info.resolution_attempted);
        assert_eq!(info.strategy, Some(ConflictResolutionStrategy::Rebase));
        assert!(info.resolved_at.is_some());
    }

    // ==================== Config Tests ====================

    #[test]
    fn test_config_no_review_required() {
        let config = PrWorkflowConfig {
            require_review_approval: false,
            ..Default::default()
        };
        let manager = PrWorkflowManager::with_config(config);

        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");
        ctx.state = PrWorkflowState::AwaitingCi;
        ctx.update_ci_status(&[CiCheckResult::new("build", CiStatus::Passed)]);

        // Should go straight to ready to merge
        let next = manager.determine_next_state(&ctx);
        assert_eq!(next, Some(PrWorkflowState::ReadyToMerge));
    }

    #[test]
    fn test_config_no_auto_merge() {
        let config = PrWorkflowConfig {
            auto_merge: false,
            ..Default::default()
        };
        let manager = PrWorkflowManager::with_config(config);

        let mut ctx = PrWorkflowContext::new(42, "story-1", "agent-1", "feature/x", "main");
        ctx.state = PrWorkflowState::ReadyToMerge;

        // Should stay at ready to merge
        let next = manager.determine_next_state(&ctx);
        assert_eq!(next, None);
    }
}
