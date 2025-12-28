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
            _ => Err(crate::Error::Other(format!("Unknown merge strategy: {}", s))),
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
