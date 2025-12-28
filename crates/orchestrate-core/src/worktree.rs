//! Git worktree management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Worktree status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "snake_case")]
#[serde(rename_all = "snake_case")]
pub enum WorktreeStatus {
    /// Worktree is active and in use
    Active,
    /// Worktree is stale (agent completed/failed)
    Stale,
    /// Worktree has been removed
    Removed,
}

/// A git worktree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    /// Unique worktree ID
    pub id: String,
    /// Worktree name (e.g., "epic-7A")
    pub name: String,
    /// Filesystem path
    pub path: String,
    /// Branch name
    pub branch_name: String,
    /// Base branch (e.g., "main")
    pub base_branch: String,
    /// Current status
    pub status: WorktreeStatus,
    /// Agent using this worktree
    pub agent_id: Option<Uuid>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Removal timestamp
    pub removed_at: Option<DateTime<Utc>>,
}

impl Worktree {
    /// Create a new worktree
    pub fn new(
        name: impl Into<String>,
        path: impl Into<String>,
        branch_name: impl Into<String>,
        base_branch: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            path: path.into(),
            branch_name: branch_name.into(),
            base_branch: base_branch.into(),
            status: WorktreeStatus::Active,
            agent_id: None,
            created_at: Utc::now(),
            removed_at: None,
        }
    }

    /// Associate with an agent
    pub fn with_agent(mut self, agent_id: Uuid) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Mark as stale
    pub fn mark_stale(&mut self) {
        self.status = WorktreeStatus::Stale;
    }

    /// Mark as removed
    pub fn mark_removed(&mut self) {
        self.status = WorktreeStatus::Removed;
        self.removed_at = Some(Utc::now());
    }

    /// Check if worktree is usable
    pub fn is_usable(&self) -> bool {
        self.status == WorktreeStatus::Active
    }
}
