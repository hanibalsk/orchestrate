//! Shell state bridge
//!
//! Provides interoperability between the shell orchestrate script's file-based
//! state and the Rust CLI's database-backed state.
//!
//! Shell state files:
//! - `.orchestrate/pr-queue` - Queue of worktrees waiting for PR
//! - `.orchestrate/current-pr` - Currently open PR number
//! - `.orchestrate/shepherd-*.lock` - Lock files for shepherd processes

use anyhow::{anyhow, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Default state directory relative to project root
const DEFAULT_STATE_DIR: &str = ".orchestrate";

/// Entry in the PR queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    /// Worktree name
    pub worktree: String,
    /// PR title
    pub title: String,
    /// Timestamp when queued (Unix epoch seconds)
    pub queued_at: i64,
}

impl QueueEntry {
    pub fn new(worktree: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            worktree: worktree.into(),
            title: title.into(),
            queued_at: Utc::now().timestamp(),
        }
    }

    /// Parse from queue file line format: "worktree|title|timestamp"
    pub fn from_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 3 {
            Some(Self {
                worktree: parts[0].to_string(),
                title: parts[1].to_string(),
                queued_at: parts[2].parse().unwrap_or(0),
            })
        } else if parts.len() == 2 {
            // Old format without timestamp
            Some(Self {
                worktree: parts[0].to_string(),
                title: parts[1].to_string(),
                queued_at: 0,
            })
        } else {
            None
        }
    }

    /// Format as queue file line
    pub fn to_line(&self) -> String {
        format!("{}|{}|{}", self.worktree, self.title, self.queued_at)
    }
}

/// Shepherd process lock info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShepherdLock {
    /// PR number being shepherded
    pub pr_number: i32,
    /// Process ID
    pub pid: u32,
    /// Whether process is still running
    pub is_active: bool,
}

/// Shell state manager
#[derive(Debug, Clone)]
pub struct ShellState {
    /// Base directory for state files
    state_dir: PathBuf,
}

impl ShellState {
    /// Create a new shell state manager
    pub fn new(project_root: impl AsRef<Path>) -> Self {
        Self {
            state_dir: project_root.as_ref().join(DEFAULT_STATE_DIR),
        }
    }

    /// Create state directory if it doesn't exist
    pub fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.state_dir)?;
        Ok(())
    }

    // ==================== Queue Operations ====================

    fn queue_file(&self) -> PathBuf {
        self.state_dir.join("pr-queue")
    }

    /// Get all entries in the PR queue
    pub fn queue_list(&self) -> Result<Vec<QueueEntry>> {
        let path = self.queue_file();
        if !path.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&path)?;
        let entries: Vec<QueueEntry> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(QueueEntry::from_line)
            .collect();

        Ok(entries)
    }

    /// Add entry to queue
    pub fn queue_add(&self, entry: QueueEntry) -> Result<()> {
        self.ensure_dir()?;
        let path = self.queue_file();

        let mut content = if path.exists() {
            fs::read_to_string(&path)?
        } else {
            String::new()
        };

        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(&entry.to_line());
        content.push('\n');

        fs::write(&path, content)?;
        Ok(())
    }

    /// Get next entry from queue (without removing)
    pub fn queue_peek(&self) -> Result<Option<QueueEntry>> {
        let entries = self.queue_list()?;
        Ok(entries.into_iter().next())
    }

    /// Remove and return first entry from queue
    pub fn queue_pop(&self) -> Result<Option<QueueEntry>> {
        let path = self.queue_file();
        if !path.exists() {
            return Ok(None);
        }

        let entries = self.queue_list()?;
        if entries.is_empty() {
            return Ok(None);
        }

        let first = entries[0].clone();

        // Write remaining entries back
        let remaining: String = entries[1..]
            .iter()
            .map(|e| e.to_line())
            .collect::<Vec<_>>()
            .join("\n");

        if remaining.is_empty() {
            fs::remove_file(&path)?;
        } else {
            fs::write(&path, remaining + "\n")?;
        }

        Ok(Some(first))
    }

    /// Remove specific entry from queue
    pub fn queue_remove(&self, worktree: &str) -> Result<bool> {
        let path = self.queue_file();
        if !path.exists() {
            return Ok(false);
        }

        let entries = self.queue_list()?;
        let original_len = entries.len();

        let remaining: Vec<QueueEntry> = entries
            .into_iter()
            .filter(|e| e.worktree != worktree)
            .collect();

        if remaining.len() == original_len {
            return Ok(false);
        }

        if remaining.is_empty() {
            fs::remove_file(&path)?;
        } else {
            let content = remaining
                .iter()
                .map(|e| e.to_line())
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            fs::write(&path, content)?;
        }

        Ok(true)
    }

    /// Clear the entire queue
    pub fn queue_clear(&self) -> Result<()> {
        let path = self.queue_file();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    // ==================== Current PR ====================

    fn current_pr_file(&self) -> PathBuf {
        self.state_dir.join("current-pr")
    }

    /// Get the current PR number
    pub fn current_pr(&self) -> Result<Option<i32>> {
        let path = self.current_pr_file();
        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path)?;
        let pr_num: i32 = content
            .trim()
            .parse()
            .map_err(|_| anyhow!("Invalid PR number in current-pr file"))?;

        Ok(Some(pr_num))
    }

    /// Set the current PR number
    pub fn set_current_pr(&self, pr_number: i32) -> Result<()> {
        self.ensure_dir()?;
        fs::write(self.current_pr_file(), pr_number.to_string())?;
        Ok(())
    }

    /// Clear the current PR
    pub fn clear_current_pr(&self) -> Result<()> {
        let path = self.current_pr_file();
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    // ==================== Shepherd Locks ====================

    fn shepherd_lock_file(&self, pr_number: i32) -> PathBuf {
        self.state_dir.join(format!("shepherd-{}.lock", pr_number))
    }

    /// Get all shepherd locks
    pub fn shepherd_locks(&self) -> Result<Vec<ShepherdLock>> {
        self.ensure_dir()?;

        let mut locks = Vec::new();

        for entry in fs::read_dir(&self.state_dir)? {
            let entry = entry?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy();

            if name_str.starts_with("shepherd-") && name_str.ends_with(".lock") {
                // Extract PR number
                if let Some(num_str) = name_str
                    .strip_prefix("shepherd-")
                    .and_then(|s| s.strip_suffix(".lock"))
                {
                    if let Ok(pr_number) = num_str.parse::<i32>() {
                        let content = fs::read_to_string(entry.path())?;
                        if let Ok(pid) = content.trim().parse::<u32>() {
                            // Check if process is still running
                            let is_active = Self::is_pid_running(pid);
                            locks.push(ShepherdLock {
                                pr_number,
                                pid,
                                is_active,
                            });
                        }
                    }
                }
            }
        }

        Ok(locks)
    }

    /// Check if a shepherd is running for a PR
    pub fn is_shepherd_running(&self, pr_number: i32) -> Result<bool> {
        let path = self.shepherd_lock_file(pr_number);
        if !path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&path)?;
        if let Ok(pid) = content.trim().parse::<u32>() {
            return Ok(Self::is_pid_running(pid));
        }

        Ok(false)
    }

    /// Create shepherd lock
    pub fn create_shepherd_lock(&self, pr_number: i32, pid: u32) -> Result<()> {
        self.ensure_dir()?;
        fs::write(self.shepherd_lock_file(pr_number), pid.to_string())?;
        Ok(())
    }

    /// Remove shepherd lock
    pub fn remove_shepherd_lock(&self, pr_number: i32) -> Result<()> {
        let path = self.shepherd_lock_file(pr_number);
        if path.exists() {
            fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Clean up stale shepherd locks (processes no longer running)
    pub fn cleanup_stale_locks(&self) -> Result<Vec<i32>> {
        let mut cleaned = Vec::new();

        for lock in self.shepherd_locks()? {
            if !lock.is_active {
                self.remove_shepherd_lock(lock.pr_number)?;
                cleaned.push(lock.pr_number);
            }
        }

        Ok(cleaned)
    }

    // ==================== Utilities ====================

    /// Check if a process ID is still running
    #[cfg(unix)]
    fn is_pid_running(pid: u32) -> bool {
        use std::process::Command;
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    fn is_pid_running(_pid: u32) -> bool {
        // On non-Unix systems, assume process is running
        true
    }

    /// Get the state directory path
    pub fn state_dir(&self) -> &Path {
        &self.state_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_queue_operations() {
        let temp = TempDir::new().unwrap();
        let state = ShellState::new(temp.path());

        // Initially empty
        assert!(state.queue_list().unwrap().is_empty());
        assert!(state.queue_peek().unwrap().is_none());

        // Add entries
        state.queue_add(QueueEntry::new("wt-1", "Title 1")).unwrap();
        state.queue_add(QueueEntry::new("wt-2", "Title 2")).unwrap();

        let list = state.queue_list().unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].worktree, "wt-1");
        assert_eq!(list[1].worktree, "wt-2");

        // Peek doesn't remove
        let peeked = state.queue_peek().unwrap().unwrap();
        assert_eq!(peeked.worktree, "wt-1");
        assert_eq!(state.queue_list().unwrap().len(), 2);

        // Pop removes
        let popped = state.queue_pop().unwrap().unwrap();
        assert_eq!(popped.worktree, "wt-1");
        assert_eq!(state.queue_list().unwrap().len(), 1);

        // Remove specific
        state.queue_add(QueueEntry::new("wt-3", "Title 3")).unwrap();
        assert!(state.queue_remove("wt-2").unwrap());
        let list = state.queue_list().unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].worktree, "wt-3");

        // Clear
        state.queue_clear().unwrap();
        assert!(state.queue_list().unwrap().is_empty());
    }

    #[test]
    fn test_current_pr() {
        let temp = TempDir::new().unwrap();
        let state = ShellState::new(temp.path());

        assert!(state.current_pr().unwrap().is_none());

        state.set_current_pr(42).unwrap();
        assert_eq!(state.current_pr().unwrap(), Some(42));

        state.clear_current_pr().unwrap();
        assert!(state.current_pr().unwrap().is_none());
    }

    #[test]
    fn test_queue_entry_parsing() {
        let entry = QueueEntry::from_line("my-wt|My Title|1234567890").unwrap();
        assert_eq!(entry.worktree, "my-wt");
        assert_eq!(entry.title, "My Title");
        assert_eq!(entry.queued_at, 1234567890);

        // Old format
        let old = QueueEntry::from_line("wt|Title").unwrap();
        assert_eq!(old.worktree, "wt");
        assert_eq!(old.title, "Title");
        assert_eq!(old.queued_at, 0);
    }
}
