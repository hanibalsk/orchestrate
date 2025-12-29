//! Pull Request management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// PR status in the queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum PrStatus {
    /// PR is queued, waiting to be created
    Queued,
    /// PR is being created
    Creating,
    /// PR is open and waiting for review
    Open,
    /// PR is being reviewed
    Reviewing,
    /// PR needs fixes
    Fixing,
    /// PR is being merged
    Merging,
    /// PR has been merged
    Merged,
    /// PR failed (could not create/merge)
    Failed,
    /// PR was closed without merging
    Closed,
}

impl PrStatus {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            PrStatus::Queued => "queued",
            PrStatus::Creating => "creating",
            PrStatus::Open => "open",
            PrStatus::Reviewing => "reviewing",
            PrStatus::Fixing => "fixing",
            PrStatus::Merging => "merging",
            PrStatus::Merged => "merged",
            PrStatus::Failed => "failed",
            PrStatus::Closed => "closed",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "queued" => Ok(PrStatus::Queued),
            "creating" => Ok(PrStatus::Creating),
            "open" => Ok(PrStatus::Open),
            "reviewing" => Ok(PrStatus::Reviewing),
            "fixing" => Ok(PrStatus::Fixing),
            "merging" => Ok(PrStatus::Merging),
            "merged" => Ok(PrStatus::Merged),
            "failed" => Ok(PrStatus::Failed),
            "closed" => Ok(PrStatus::Closed),
            _ => Err(crate::Error::Other(format!("Unknown PR status: {}", s))),
        }
    }

    /// Check if PR is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, PrStatus::Merged | PrStatus::Failed | PrStatus::Closed)
    }

    /// Check if PR is active (not terminal)
    pub fn is_active(&self) -> bool {
        !self.is_terminal()
    }
}

/// Merge strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Squash all commits
    Squash,
    /// Rebase onto target
    Rebase,
    /// Merge commit
    Merge,
}

impl MergeStrategy {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            MergeStrategy::Squash => "squash",
            MergeStrategy::Rebase => "rebase",
            MergeStrategy::Merge => "merge",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s {
            "squash" => Ok(MergeStrategy::Squash),
            "rebase" => Ok(MergeStrategy::Rebase),
            "merge" => Ok(MergeStrategy::Merge),
            _ => Err(crate::Error::Other(format!(
                "Unknown merge strategy: {}",
                s
            ))),
        }
    }
}

impl Default for MergeStrategy {
    fn default() -> Self {
        MergeStrategy::Squash
    }
}

/// A pull request in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// Internal ID
    pub id: i64,
    /// Epic ID if associated
    pub epic_id: Option<String>,
    /// Worktree ID
    pub worktree_id: Option<String>,
    /// Branch name
    pub branch_name: String,
    /// PR title
    pub title: Option<String>,
    /// PR body/description
    pub body: Option<String>,
    /// GitHub PR number (once created)
    pub pr_number: Option<i32>,
    /// Current status
    pub status: PrStatus,
    /// Merge strategy
    pub merge_strategy: MergeStrategy,
    /// Agent handling this PR
    pub agent_id: Option<Uuid>,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
    /// Merge timestamp
    pub merged_at: Option<DateTime<Utc>>,
}

impl PullRequest {
    /// Create a new queued PR
    pub fn new(branch_name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: 0, // Set by database
            epic_id: None,
            worktree_id: None,
            branch_name: branch_name.into(),
            title: None,
            body: None,
            pr_number: None,
            status: PrStatus::Queued,
            merge_strategy: MergeStrategy::default(),
            agent_id: None,
            error_message: None,
            created_at: now,
            updated_at: now,
            merged_at: None,
        }
    }

    /// Set epic ID
    pub fn with_epic(mut self, epic_id: impl Into<String>) -> Self {
        self.epic_id = Some(epic_id.into());
        self
    }

    /// Set worktree ID
    pub fn with_worktree(mut self, worktree_id: impl Into<String>) -> Self {
        self.worktree_id = Some(worktree_id.into());
        self
    }

    /// Set title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Set merge strategy
    pub fn with_strategy(mut self, strategy: MergeStrategy) -> Self {
        self.merge_strategy = strategy;
        self
    }

    /// Update status
    pub fn set_status(&mut self, status: PrStatus) {
        self.status = status;
        self.updated_at = Utc::now();

        if status == PrStatus::Merged {
            self.merged_at = Some(Utc::now());
        }
    }

    /// Mark as failed
    pub fn fail(&mut self, error: impl Into<String>) {
        self.status = PrStatus::Failed;
        self.error_message = Some(error.into());
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== PrStatus Tests ====================

    #[test]
    fn test_pr_status_as_str() {
        assert_eq!(PrStatus::Queued.as_str(), "queued");
        assert_eq!(PrStatus::Open.as_str(), "open");
        assert_eq!(PrStatus::Merged.as_str(), "merged");
        assert_eq!(PrStatus::Failed.as_str(), "failed");
    }

    #[test]
    fn test_pr_status_from_str() {
        assert_eq!(PrStatus::from_str("queued").unwrap(), PrStatus::Queued);
        assert_eq!(PrStatus::from_str("open").unwrap(), PrStatus::Open);
        assert_eq!(PrStatus::from_str("merged").unwrap(), PrStatus::Merged);
        assert!(PrStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_pr_status_is_terminal() {
        assert!(PrStatus::Merged.is_terminal());
        assert!(PrStatus::Failed.is_terminal());
        assert!(PrStatus::Closed.is_terminal());
        assert!(!PrStatus::Queued.is_terminal());
        assert!(!PrStatus::Open.is_terminal());
        assert!(!PrStatus::Reviewing.is_terminal());
    }

    #[test]
    fn test_pr_status_is_active() {
        assert!(PrStatus::Queued.is_active());
        assert!(PrStatus::Open.is_active());
        assert!(PrStatus::Reviewing.is_active());
        assert!(!PrStatus::Merged.is_active());
        assert!(!PrStatus::Failed.is_active());
    }

    // ==================== MergeStrategy Tests ====================

    #[test]
    fn test_merge_strategy_as_str() {
        assert_eq!(MergeStrategy::Squash.as_str(), "squash");
        assert_eq!(MergeStrategy::Rebase.as_str(), "rebase");
        assert_eq!(MergeStrategy::Merge.as_str(), "merge");
    }

    #[test]
    fn test_merge_strategy_from_str() {
        assert_eq!(
            MergeStrategy::from_str("squash").unwrap(),
            MergeStrategy::Squash
        );
        assert_eq!(
            MergeStrategy::from_str("rebase").unwrap(),
            MergeStrategy::Rebase
        );
        assert_eq!(
            MergeStrategy::from_str("merge").unwrap(),
            MergeStrategy::Merge
        );
        assert!(MergeStrategy::from_str("invalid").is_err());
    }

    #[test]
    fn test_merge_strategy_default() {
        assert_eq!(MergeStrategy::default(), MergeStrategy::Squash);
    }

    // ==================== PullRequest Tests ====================

    #[test]
    fn test_pull_request_new() {
        let pr = PullRequest::new("feature/auth");

        assert_eq!(pr.branch_name, "feature/auth");
        assert_eq!(pr.status, PrStatus::Queued);
        assert_eq!(pr.merge_strategy, MergeStrategy::Squash);
        assert!(pr.title.is_none());
        assert!(pr.pr_number.is_none());
        assert!(pr.error_message.is_none());
    }

    #[test]
    fn test_pull_request_with_epic() {
        let pr = PullRequest::new("feature/auth").with_epic("epic-7A");

        assert_eq!(pr.epic_id, Some("epic-7A".to_string()));
    }

    #[test]
    fn test_pull_request_with_worktree() {
        let pr = PullRequest::new("feature/auth").with_worktree("wt-123");

        assert_eq!(pr.worktree_id, Some("wt-123".to_string()));
    }

    #[test]
    fn test_pull_request_with_title() {
        let pr = PullRequest::new("feature/auth").with_title("Add authentication");

        assert_eq!(pr.title, Some("Add authentication".to_string()));
    }

    #[test]
    fn test_pull_request_with_strategy() {
        let pr = PullRequest::new("feature/auth").with_strategy(MergeStrategy::Rebase);

        assert_eq!(pr.merge_strategy, MergeStrategy::Rebase);
    }

    #[test]
    fn test_pull_request_set_status() {
        let mut pr = PullRequest::new("feature/auth");
        let initial_updated = pr.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        pr.set_status(PrStatus::Open);

        assert_eq!(pr.status, PrStatus::Open);
        assert!(pr.updated_at > initial_updated);
        assert!(pr.merged_at.is_none());
    }

    #[test]
    fn test_pull_request_set_status_merged() {
        let mut pr = PullRequest::new("feature/auth");
        pr.set_status(PrStatus::Merged);

        assert_eq!(pr.status, PrStatus::Merged);
        assert!(pr.merged_at.is_some());
    }

    #[test]
    fn test_pull_request_fail() {
        let mut pr = PullRequest::new("feature/auth");
        pr.fail("Merge conflict");

        assert_eq!(pr.status, PrStatus::Failed);
        assert_eq!(pr.error_message, Some("Merge conflict".to_string()));
    }

    #[test]
    fn test_pull_request_builder_chain() {
        let pr = PullRequest::new("feature/auth")
            .with_epic("epic-1")
            .with_worktree("wt-1")
            .with_title("Auth feature")
            .with_strategy(MergeStrategy::Rebase);

        assert_eq!(pr.epic_id, Some("epic-1".to_string()));
        assert_eq!(pr.worktree_id, Some("wt-1".to_string()));
        assert_eq!(pr.title, Some("Auth feature".to_string()));
        assert_eq!(pr.merge_strategy, MergeStrategy::Rebase);
    }
}
