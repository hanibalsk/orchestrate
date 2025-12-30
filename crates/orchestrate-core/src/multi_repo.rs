//! Multi-Repository Orchestration Module
//!
//! Types and utilities for coordinating work across multiple repositories,
//! handling cross-repo dependencies, and synchronized releases.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Repository configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repository {
    pub name: String,
    pub url: String,
    pub local_path: Option<String>,
    pub default_branch: String,
    pub dependencies: Vec<String>,
    pub provider: RepoProvider,
    pub status: RepoStatus,
    pub last_synced: Option<DateTime<Utc>>,
    pub config: RepoConfig,
}

/// Repository provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoProvider {
    GitHub,
    GitLab,
    Bitbucket,
    Other,
}

impl RepoProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GitHub => "github",
            Self::GitLab => "gitlab",
            Self::Bitbucket => "bitbucket",
            Self::Other => "other",
        }
    }

    pub fn from_url(url: &str) -> Self {
        if url.contains("github.com") {
            Self::GitHub
        } else if url.contains("gitlab.com") {
            Self::GitLab
        } else if url.contains("bitbucket.org") {
            Self::Bitbucket
        } else {
            Self::Other
        }
    }
}

/// Repository status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RepoStatus {
    Active,
    Inactive,
    Error,
    Syncing,
}

impl RepoStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Error => "error",
            Self::Syncing => "syncing",
        }
    }
}

/// Repository-specific configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoConfig {
    pub auto_sync: bool,
    pub sync_interval_minutes: Option<u32>,
    pub package_manager: Option<String>,
    pub build_command: Option<String>,
    pub test_command: Option<String>,
}

impl Repository {
    /// Create a new repository configuration
    pub fn new(name: &str, url: &str) -> Self {
        Self {
            name: name.to_string(),
            url: url.to_string(),
            local_path: None,
            default_branch: "main".to_string(),
            dependencies: vec![],
            provider: RepoProvider::from_url(url),
            status: RepoStatus::Inactive,
            last_synced: None,
            config: RepoConfig::default(),
        }
    }

    /// Set the local path
    pub fn with_local_path(mut self, path: &str) -> Self {
        self.local_path = Some(path.to_string());
        self
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, repo_name: &str) {
        if !self.dependencies.contains(&repo_name.to_string()) {
            self.dependencies.push(repo_name.to_string());
        }
    }
}

/// Dependency graph for repositories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoDependencyGraph {
    pub repositories: HashMap<String, Vec<String>>,
    pub has_circular: bool,
    pub circular_paths: Vec<Vec<String>>,
}

impl RepoDependencyGraph {
    /// Create a new dependency graph
    pub fn new() -> Self {
        Self {
            repositories: HashMap::new(),
            has_circular: false,
            circular_paths: vec![],
        }
    }

    /// Add a repository and its dependencies
    pub fn add_repo(&mut self, name: &str, dependencies: Vec<String>) {
        self.repositories.insert(name.to_string(), dependencies);
    }

    /// Detect circular dependencies using DFS
    pub fn detect_circular(&mut self) {
        self.circular_paths.clear();
        let repos: Vec<String> = self.repositories.keys().cloned().collect();

        for repo in &repos {
            let mut visited = HashSet::new();
            let mut path = vec![];
            if self.has_cycle(repo, &mut visited, &mut path) {
                self.has_circular = true;
            }
        }
    }

    fn has_cycle(
        &mut self,
        repo: &str,
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if path.contains(&repo.to_string()) {
            // Found a cycle
            let cycle_start = path.iter().position(|r| r == repo).unwrap();
            let cycle: Vec<String> = path[cycle_start..].to_vec();
            self.circular_paths.push(cycle);
            return true;
        }

        if visited.contains(repo) {
            return false;
        }

        visited.insert(repo.to_string());
        path.push(repo.to_string());

        let mut has_cycle = false;
        if let Some(deps) = self.repositories.get(repo).cloned() {
            for dep in deps {
                if self.has_cycle(&dep, visited, path) {
                    has_cycle = true;
                }
            }
        }

        path.pop();
        has_cycle
    }

    /// Get topological order (build order)
    pub fn topological_order(&self) -> Result<Vec<String>, String> {
        if self.has_circular {
            return Err("Cannot determine order: circular dependencies exist".to_string());
        }

        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut result = vec![];

        // Initialize in-degrees
        for repo in self.repositories.keys() {
            in_degree.entry(repo.clone()).or_insert(0);
        }

        // Calculate in-degrees
        for deps in self.repositories.values() {
            for dep in deps {
                *in_degree.entry(dep.clone()).or_insert(0) += 1;
            }
        }

        // Start with nodes that have no dependencies
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &degree)| degree == 0)
            .map(|(repo, _)| repo.clone())
            .collect();
        queue.sort(); // Consistent ordering

        while let Some(repo) = queue.pop() {
            result.push(repo.clone());

            if let Some(deps) = self.repositories.get(&repo) {
                for dep in deps {
                    if let Some(degree) = in_degree.get_mut(dep) {
                        *degree = degree.saturating_sub(1);
                        if *degree == 0 {
                            queue.push(dep.clone());
                        }
                    }
                }
            }
        }

        result.reverse();
        Ok(result)
    }

    /// Generate a Mermaid diagram
    pub fn to_mermaid(&self) -> String {
        let mut output = String::from("graph TD\n");

        for (repo, deps) in &self.repositories {
            if deps.is_empty() {
                output.push_str(&format!("    {}[{}]\n", repo, repo));
            } else {
                for dep in deps {
                    output.push_str(&format!("    {} --> {}\n", repo, dep));
                }
            }
        }

        output
    }
}

impl Default for RepoDependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Cross-repository branch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossRepoBranch {
    pub name: String,
    pub repos: Vec<RepoBranchStatus>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Branch status in a specific repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoBranchStatus {
    pub repo_name: String,
    pub branch_exists: bool,
    pub commits_ahead: Option<u32>,
    pub commits_behind: Option<u32>,
    pub has_conflicts: bool,
    pub pr_number: Option<u32>,
    pub pr_status: Option<String>,
}

/// Linked pull request group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedPrGroup {
    pub id: String,
    pub name: String,
    pub prs: Vec<LinkedPr>,
    pub merge_order: Vec<String>,
    pub status: LinkedPrStatus,
    pub created_at: DateTime<Utc>,
}

/// Individual linked PR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkedPr {
    pub repo_name: String,
    pub pr_number: u32,
    pub title: String,
    pub status: String,
    pub mergeable: bool,
}

/// Status of linked PR group
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkedPrStatus {
    Open,
    ReadyToMerge,
    PartiallyMerged,
    Merged,
    Blocked,
}

impl LinkedPrStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::ReadyToMerge => "ready_to_merge",
            Self::PartiallyMerged => "partially_merged",
            Self::Merged => "merged",
            Self::Blocked => "blocked",
        }
    }
}

impl std::str::FromStr for LinkedPrStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(Self::Open),
            "ready_to_merge" => Ok(Self::ReadyToMerge),
            "partially_merged" => Ok(Self::PartiallyMerged),
            "merged" => Ok(Self::Merged),
            "blocked" => Ok(Self::Blocked),
            _ => Err(crate::Error::Other(format!("Invalid LinkedPrStatus: {}", s))),
        }
    }
}

/// Coordinated release
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinatedRelease {
    pub version: String,
    pub repos: Vec<RepoRelease>,
    pub status: ReleaseStatus,
    pub changelog: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

/// Release for a specific repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRelease {
    pub repo_name: String,
    pub version: String,
    pub status: ReleaseStatus,
    pub tag: Option<String>,
    pub release_url: Option<String>,
}

/// Release status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    RolledBack,
}

impl ReleaseStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::RolledBack => "rolled_back",
        }
    }
}

impl std::str::FromStr for ReleaseStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "rolled_back" => Ok(Self::RolledBack),
            _ => Err(crate::Error::Other(format!("Invalid ReleaseStatus: {}", s))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repository_creation() {
        let repo = Repository::new("api", "https://github.com/org/api");

        assert_eq!(repo.name, "api");
        assert_eq!(repo.url, "https://github.com/org/api");
        assert_eq!(repo.provider, RepoProvider::GitHub);
        assert_eq!(repo.status, RepoStatus::Inactive);
    }

    #[test]
    fn test_repo_provider_detection() {
        assert_eq!(
            RepoProvider::from_url("https://github.com/org/repo"),
            RepoProvider::GitHub
        );
        assert_eq!(
            RepoProvider::from_url("https://gitlab.com/org/repo"),
            RepoProvider::GitLab
        );
        assert_eq!(
            RepoProvider::from_url("https://bitbucket.org/org/repo"),
            RepoProvider::Bitbucket
        );
        assert_eq!(
            RepoProvider::from_url("https://custom.git.server/repo"),
            RepoProvider::Other
        );
    }

    #[test]
    fn test_dependency_graph_no_circular() {
        let mut graph = RepoDependencyGraph::new();
        graph.add_repo("api", vec!["core-lib".to_string()]);
        graph.add_repo("web", vec!["api".to_string()]);
        graph.add_repo("core-lib", vec![]);

        graph.detect_circular();

        assert!(!graph.has_circular);
        assert!(graph.circular_paths.is_empty());
    }

    #[test]
    fn test_dependency_graph_circular() {
        let mut graph = RepoDependencyGraph::new();
        graph.add_repo("a", vec!["b".to_string()]);
        graph.add_repo("b", vec!["c".to_string()]);
        graph.add_repo("c", vec!["a".to_string()]);

        graph.detect_circular();

        assert!(graph.has_circular);
        assert!(!graph.circular_paths.is_empty());
    }

    #[test]
    fn test_dependency_graph_mermaid() {
        let mut graph = RepoDependencyGraph::new();
        graph.add_repo("api", vec!["core".to_string()]);
        graph.add_repo("web", vec!["api".to_string()]);
        graph.add_repo("core", vec![]);

        let mermaid = graph.to_mermaid();

        assert!(mermaid.contains("graph TD"));
        assert!(mermaid.contains("api --> core"));
        assert!(mermaid.contains("web --> api"));
    }

    #[test]
    fn test_linked_pr_status() {
        assert_eq!(LinkedPrStatus::Open.as_str(), "open");
        assert_eq!(LinkedPrStatus::ReadyToMerge.as_str(), "ready_to_merge");
        assert_eq!(LinkedPrStatus::Merged.as_str(), "merged");
        assert_eq!(LinkedPrStatus::Blocked.as_str(), "blocked");
    }
}
