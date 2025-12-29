//! Git worktree management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use uuid::Uuid;

use crate::{Error, Result};

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

/// Create a git worktree for a PR branch
///
/// This function:
/// 1. Fetches the PR branch from origin
/// 2. Creates a worktree directory
/// 3. Adds the worktree via git worktree add
///
/// # Arguments
/// * `pr_number` - The PR number
/// * `branch_name` - The branch name to check out
/// * `worktree_dir` - The base directory for worktrees (e.g., ".worktrees")
///
/// # Returns
/// The created Worktree struct with path and metadata
pub fn create_pr_worktree(
    pr_number: i32,
    branch_name: &str,
    worktree_dir: &str,
) -> Result<Worktree> {
    let name = format!("pr-{}", pr_number);
    let worktree_path = PathBuf::from(worktree_dir).join(&name);

    // Ensure worktree directory exists
    std::fs::create_dir_all(worktree_dir)?;

    // Fetch the branch from origin
    let fetch_output = Command::new("git")
        .args(["fetch", "origin", branch_name])
        .output()?;

    if !fetch_output.status.success() {
        return Err(Error::Other(format!(
            "Failed to fetch branch {}: {}",
            branch_name,
            String::from_utf8_lossy(&fetch_output.stderr)
        )));
    }

    // Prune stale worktrees first
    let _ = Command::new("git").args(["worktree", "prune"]).output();

    // Try to add worktree
    let add_output = Command::new("git")
        .args([
            "worktree",
            "add",
            worktree_path.to_str().unwrap(),
            &format!("origin/{}", branch_name),
        ])
        .output()?;

    if !add_output.status.success() {
        // Try with -f flag if it already exists
        let force_output = Command::new("git")
            .args([
                "worktree",
                "add",
                "-f",
                worktree_path.to_str().unwrap(),
                &format!("origin/{}", branch_name),
            ])
            .output()?;

        if !force_output.status.success() {
            return Err(Error::Other(format!(
                "Failed to create worktree: {}",
                String::from_utf8_lossy(&force_output.stderr)
            )));
        }
    }

    // Create worktree record
    let worktree = Worktree::new(
        name,
        worktree_path.to_string_lossy().to_string(),
        branch_name.to_string(),
        "main".to_string(), // Default base branch
    );

    Ok(worktree)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worktree_new() {
        let wt = Worktree::new("test-wt", "/tmp/test", "feature/branch", "main");
        assert_eq!(wt.name, "test-wt");
        assert_eq!(wt.path, "/tmp/test");
        assert_eq!(wt.branch_name, "feature/branch");
        assert_eq!(wt.base_branch, "main");
        assert_eq!(wt.status, WorktreeStatus::Active);
        assert!(wt.is_usable());
    }

    #[test]
    fn test_worktree_with_agent() {
        let agent_id = Uuid::new_v4();
        let wt = Worktree::new("test-wt", "/tmp/test", "feature/branch", "main")
            .with_agent(agent_id);
        assert_eq!(wt.agent_id, Some(agent_id));
    }

    #[test]
    fn test_worktree_mark_stale() {
        let mut wt = Worktree::new("test-wt", "/tmp/test", "feature/branch", "main");
        wt.mark_stale();
        assert_eq!(wt.status, WorktreeStatus::Stale);
        assert!(!wt.is_usable());
    }

    #[test]
    fn test_worktree_mark_removed() {
        let mut wt = Worktree::new("test-wt", "/tmp/test", "feature/branch", "main");
        wt.mark_removed();
        assert_eq!(wt.status, WorktreeStatus::Removed);
        assert!(wt.removed_at.is_some());
        assert!(!wt.is_usable());
    }
}
