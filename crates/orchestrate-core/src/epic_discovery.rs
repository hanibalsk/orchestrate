//! Epic Discovery and Planning
//!
//! Epic 016: Autonomous Epic Processing - Story 11
//!
//! Discovers epics and creates prioritized work plans:
//! - Scan epic files in docs/bmad/epics/
//! - Parse epic markdown for stories
//! - Build dependency graph
//! - Create prioritized work queue

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// Pre-compiled regex for story heading parsing (hot path during epic parsing)
static STORY_HEADING_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"###\s+Story\s+(\d+)[:\s]+(.+)").unwrap()
});

/// Status of an epic in autonomous processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpicProcessingStatus {
    /// Not yet discovered/processed
    Pending,
    /// Being analyzed
    Analyzing,
    /// Stories discovered, ready to execute
    Planned,
    /// Currently being executed
    InProgress,
    /// All stories completed
    Completed,
    /// Blocked due to issues
    Blocked,
    /// Skipped (e.g., pattern didn't match)
    Skipped,
}

impl EpicProcessingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Analyzing => "analyzing",
            Self::Planned => "planned",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Blocked => "blocked",
            Self::Skipped => "skipped",
        }
    }
}

impl std::str::FromStr for EpicProcessingStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "analyzing" => Ok(Self::Analyzing),
            "planned" => Ok(Self::Planned),
            "in_progress" => Ok(Self::InProgress),
            "completed" => Ok(Self::Completed),
            "blocked" => Ok(Self::Blocked),
            "skipped" => Ok(Self::Skipped),
            _ => Err(crate::Error::Other(format!(
                "Invalid epic processing status: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for EpicProcessingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Story status in autonomous processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoryProcessingStatus {
    /// Not yet started
    Pending,
    /// Waiting for dependencies
    Waiting,
    /// Currently being executed
    InProgress,
    /// Awaiting code review
    AwaitingReview,
    /// Awaiting PR merge
    AwaitingMerge,
    /// Successfully completed
    Completed,
    /// Failed
    Failed,
    /// Blocked
    Blocked,
    /// Skipped (e.g., already done)
    Skipped,
}

impl StoryProcessingStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Waiting => "waiting",
            Self::InProgress => "in_progress",
            Self::AwaitingReview => "awaiting_review",
            Self::AwaitingMerge => "awaiting_merge",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Blocked => "blocked",
            Self::Skipped => "skipped",
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self, Self::Completed | Self::Skipped)
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Blocked | Self::Skipped)
    }

    pub fn can_start(&self) -> bool {
        matches!(self, Self::Pending)
    }
}

impl std::str::FromStr for StoryProcessingStatus {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "waiting" => Ok(Self::Waiting),
            "in_progress" => Ok(Self::InProgress),
            "awaiting_review" => Ok(Self::AwaitingReview),
            "awaiting_merge" => Ok(Self::AwaitingMerge),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "blocked" => Ok(Self::Blocked),
            "skipped" => Ok(Self::Skipped),
            _ => Err(crate::Error::Other(format!(
                "Invalid story processing status: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for StoryProcessingStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A discovered epic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredEpic {
    /// Epic ID (e.g., "epic-016")
    pub id: String,
    /// Epic title
    pub title: String,
    /// Epic file path
    pub file_path: PathBuf,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// Processing status
    pub status: EpicProcessingStatus,
    /// Discovered stories
    pub stories: Vec<DiscoveredStory>,
    /// Epic description/summary
    pub description: Option<String>,
    /// Dependencies on other epics
    pub epic_dependencies: Vec<String>,
    /// Discovered at timestamp
    pub discovered_at: DateTime<Utc>,
    /// Last updated
    pub updated_at: DateTime<Utc>,
}

impl DiscoveredEpic {
    pub fn new(id: impl Into<String>, title: impl Into<String>, file_path: PathBuf) -> Self {
        let now = Utc::now();
        Self {
            id: id.into(),
            title: title.into(),
            file_path,
            priority: 0,
            status: EpicProcessingStatus::Pending,
            stories: Vec::new(),
            description: None,
            epic_dependencies: Vec::new(),
            discovered_at: now,
            updated_at: now,
        }
    }

    pub fn with_priority(mut self, priority: u32) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_stories(mut self, stories: Vec<DiscoveredStory>) -> Self {
        self.stories = stories;
        self
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Get completed story count
    pub fn completed_count(&self) -> usize {
        self.stories.iter().filter(|s| s.status.is_complete()).count()
    }

    /// Get total story count
    pub fn total_count(&self) -> usize {
        self.stories.len()
    }

    /// Get completion percentage
    pub fn completion_percentage(&self) -> f64 {
        if self.stories.is_empty() {
            return 0.0;
        }
        (self.completed_count() as f64 / self.total_count() as f64) * 100.0
    }
}

/// A discovered story within an epic
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredStory {
    /// Story ID (e.g., "story-1")
    pub id: String,
    /// Story title
    pub title: String,
    /// Story number within epic
    pub number: u32,
    /// Processing status
    pub status: StoryProcessingStatus,
    /// Acceptance criteria
    pub acceptance_criteria: Vec<String>,
    /// Story dependencies (other story IDs)
    pub dependencies: Vec<String>,
    /// Estimated complexity (1-5)
    pub complexity: Option<u32>,
    /// Assigned agent ID
    pub assigned_agent: Option<String>,
    /// Started at
    pub started_at: Option<DateTime<Utc>>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
}

impl DiscoveredStory {
    pub fn new(id: impl Into<String>, title: impl Into<String>, number: u32) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            number,
            status: StoryProcessingStatus::Pending,
            acceptance_criteria: Vec::new(),
            dependencies: Vec::new(),
            complexity: None,
            assigned_agent: None,
            started_at: None,
            completed_at: None,
        }
    }

    pub fn with_criteria(mut self, criteria: Vec<String>) -> Self {
        self.acceptance_criteria = criteria;
        self
    }

    pub fn with_dependencies(mut self, deps: Vec<String>) -> Self {
        self.dependencies = deps;
        self
    }

    pub fn with_complexity(mut self, complexity: u32) -> Self {
        self.complexity = Some(complexity.clamp(1, 5));
        self
    }
}

/// Work queue item for autonomous processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkQueueItem {
    /// Epic ID
    pub epic_id: String,
    /// Story ID
    pub story_id: String,
    /// Full ID (epic-story)
    pub full_id: String,
    /// Title
    pub title: String,
    /// Priority (lower = higher priority)
    pub priority: u32,
    /// Dependencies (full IDs)
    pub dependencies: Vec<String>,
    /// Status
    pub status: StoryProcessingStatus,
    /// Queued at
    pub queued_at: DateTime<Utc>,
}

impl WorkQueueItem {
    pub fn from_story(epic_id: &str, story: &DiscoveredStory) -> Self {
        let full_id = format!("{}/story-{}", epic_id, story.number);
        Self {
            epic_id: epic_id.to_string(),
            story_id: story.id.clone(),
            full_id,
            title: story.title.clone(),
            priority: story.number, // Default priority by order
            dependencies: story
                .dependencies
                .iter()
                .map(|d| format!("{}/{}", epic_id, d))
                .collect(),
            status: story.status,
            queued_at: Utc::now(),
        }
    }

    pub fn can_execute(&self, completed: &HashSet<String>) -> bool {
        if !self.status.can_start() {
            return false;
        }
        self.dependencies.iter().all(|d| completed.contains(d))
    }
}

/// Dependency graph for stories
#[derive(Debug, Clone, Default)]
pub struct StoryDependencyGraph {
    /// Map of story full_id to its dependencies
    dependencies: HashMap<String, Vec<String>>,
    /// Map of story full_id to stories that depend on it
    dependents: HashMap<String, Vec<String>>,
}

impl StoryDependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a story with its dependencies
    pub fn add_story(&mut self, story_id: &str, dependencies: Vec<String>) {
        // Add forward dependencies
        self.dependencies.insert(story_id.to_string(), dependencies.clone());

        // Add reverse dependencies (dependents)
        for dep in dependencies {
            self.dependents
                .entry(dep)
                .or_default()
                .push(story_id.to_string());
        }
    }

    /// Get dependencies of a story
    pub fn get_dependencies(&self, story_id: &str) -> Vec<String> {
        self.dependencies.get(story_id).cloned().unwrap_or_default()
    }

    /// Get stories that depend on this story
    pub fn get_dependents(&self, story_id: &str) -> Vec<String> {
        self.dependents.get(story_id).cloned().unwrap_or_default()
    }

    /// Check if all dependencies are satisfied
    pub fn dependencies_satisfied(&self, story_id: &str, completed: &HashSet<String>) -> bool {
        self.get_dependencies(story_id)
            .iter()
            .all(|d| completed.contains(d))
    }

    /// Get stories that can be executed now
    pub fn get_executable(&self, completed: &HashSet<String>) -> Vec<String> {
        self.dependencies
            .keys()
            .filter(|id| {
                !completed.contains(*id) && self.dependencies_satisfied(id, completed)
            })
            .cloned()
            .collect()
    }

    /// Detect cycles in the dependency graph
    pub fn detect_cycles(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for node in self.dependencies.keys() {
            if let Some(cycle) = self.detect_cycle_dfs(node, &mut visited, &mut rec_stack, &mut path) {
                return Some(cycle);
            }
        }
        None
    }

    fn detect_cycle_dfs(
        &self,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        if rec_stack.contains(node) {
            // Found a cycle
            let cycle_start = path.iter().position(|n| n == node).unwrap_or(0);
            return Some(path[cycle_start..].to_vec());
        }

        if visited.contains(node) {
            return None;
        }

        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(deps) = self.dependencies.get(node) {
            for dep in deps {
                if let Some(cycle) = self.detect_cycle_dfs(dep, visited, rec_stack, path) {
                    return Some(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
        None
    }

    /// Topological sort of stories (Kahn's algorithm)
    pub fn topological_sort(&self) -> Result<Vec<String>, Vec<String>> {
        // Check for cycles first
        if let Some(cycle) = self.detect_cycles() {
            return Err(cycle);
        }

        // Kahn's algorithm for topological sort
        let mut in_degree: HashMap<String, usize> = HashMap::new();

        // Initialize in-degree for all nodes
        for node in self.dependencies.keys() {
            in_degree.entry(node.clone()).or_insert(0);
        }

        // Calculate in-degree (number of dependencies each node has)
        for (node, deps) in &self.dependencies {
            for dep in deps {
                // Only count if the dependency is a known node
                if self.dependencies.contains_key(dep) {
                    *in_degree.entry(node.clone()).or_insert(0) += 0; // ensure node exists
                }
            }
            *in_degree.entry(node.clone()).or_insert(0) = deps.iter()
                .filter(|d| self.dependencies.contains_key(*d))
                .count();
        }

        // Start with nodes that have no dependencies (in-degree 0)
        let mut queue: Vec<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(node, _)| node.clone())
            .collect();
        queue.sort(); // Deterministic order

        let mut result = Vec::new();

        while let Some(node) = queue.pop() {
            result.push(node.clone());

            // For each node that depends on this node, decrease its in-degree
            if let Some(dependents) = self.dependents.get(&node) {
                for dependent in dependents {
                    if let Some(deg) = in_degree.get_mut(dependent) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            queue.push(dependent.clone());
                            queue.sort(); // Keep sorted for deterministic order
                        }
                    }
                }
            }
        }

        Ok(result)
    }
}

/// Execution plan for autonomous processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionPlan {
    /// Epics to process (in order)
    pub epics: Vec<String>,
    /// Work queue (prioritized)
    pub work_queue: Vec<WorkQueueItem>,
    /// Total story count
    pub total_stories: u32,
    /// Estimated duration (based on complexity)
    pub estimated_minutes: Option<u32>,
    /// Dependency graph summary
    pub dependency_count: u32,
    /// Created at
    pub created_at: DateTime<Utc>,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            epics: Vec::new(),
            work_queue: Vec::new(),
            total_stories: 0,
            estimated_minutes: None,
            dependency_count: 0,
            created_at: Utc::now(),
        }
    }

    pub fn with_epics(mut self, epics: Vec<String>) -> Self {
        self.epics = epics;
        self
    }

    pub fn with_work_queue(mut self, queue: Vec<WorkQueueItem>) -> Self {
        self.total_stories = queue.len() as u32;
        self.dependency_count = queue.iter().map(|q| q.dependencies.len() as u32).sum();
        self.work_queue = queue;
        self
    }

    /// Get summary for display
    pub fn summary(&self) -> String {
        format!(
            "Execution Plan: {} epics, {} stories, {} dependencies",
            self.epics.len(),
            self.total_stories,
            self.dependency_count
        )
    }
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for epic discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpicDiscoveryConfig {
    /// Path to epics directory (relative to repo root)
    pub epics_dir: PathBuf,
    /// Pattern to match epic files (glob)
    pub file_pattern: String,
    /// Epic ID pattern (regex for extracting from filename)
    pub id_pattern: String,
    /// Skip epics matching these patterns
    pub skip_patterns: Vec<String>,
}

impl Default for EpicDiscoveryConfig {
    fn default() -> Self {
        Self {
            epics_dir: PathBuf::from("docs/bmad/epics"),
            file_pattern: "epic-*.md".to_string(),
            id_pattern: r"epic-(\d+)".to_string(),
            skip_patterns: Vec::new(),
        }
    }
}

/// Epic discovery service
#[derive(Debug, Clone)]
pub struct EpicDiscoveryService {
    config: EpicDiscoveryConfig,
}

impl EpicDiscoveryService {
    pub fn new() -> Self {
        Self {
            config: EpicDiscoveryConfig::default(),
        }
    }

    pub fn with_config(config: EpicDiscoveryConfig) -> Self {
        Self { config }
    }

    /// Parse epic markdown content
    pub fn parse_epic(&self, id: &str, content: &str, file_path: PathBuf) -> DiscoveredEpic {
        let mut epic = DiscoveredEpic::new(id, "", file_path);

        // Parse title from first heading
        if let Some(title) = self.extract_title(content) {
            epic.title = title;
        }

        // Parse description from overview section
        if let Some(desc) = self.extract_overview(content) {
            epic.description = Some(desc);
        }

        // Parse stories
        epic.stories = self.extract_stories(content, id);

        // Mark as analyzed
        epic.status = EpicProcessingStatus::Planned;
        epic.updated_at = Utc::now();

        epic
    }

    /// Extract title from epic content
    fn extract_title(&self, content: &str) -> Option<String> {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("# ") {
                return Some(trimmed[2..].trim().to_string());
            }
        }
        None
    }

    /// Extract overview section
    fn extract_overview(&self, content: &str) -> Option<String> {
        let mut in_overview = false;
        let mut overview_lines = Vec::new();

        for line in content.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("## Overview") {
                in_overview = true;
                continue;
            }

            if in_overview {
                if trimmed.starts_with("## ") {
                    break;
                }
                if !trimmed.is_empty() {
                    overview_lines.push(trimmed.to_string());
                }
            }
        }

        if overview_lines.is_empty() {
            None
        } else {
            Some(overview_lines.join(" "))
        }
    }

    /// Extract stories from epic content
    fn extract_stories(&self, content: &str, _epic_id: &str) -> Vec<DiscoveredStory> {
        let mut stories = Vec::new();
        let mut current_story: Option<(u32, String, Vec<String>)> = None;
        let mut in_criteria = false;

        for line in content.lines() {
            let trimmed = line.trim();

            // Detect story heading: "### Story N: Title"
            if trimmed.starts_with("### Story ") {
                // Save previous story if exists
                if let Some((num, title, criteria)) = current_story.take() {
                    let story_id = format!("story-{}", num);
                    let story = DiscoveredStory::new(&story_id, title, num)
                        .with_criteria(criteria);
                    stories.push(story);
                }

                // Parse new story
                if let Some((num, title)) = self.parse_story_heading(trimmed) {
                    current_story = Some((num, title, Vec::new()));
                    in_criteria = false;
                }
                continue;
            }

            // Detect acceptance criteria section
            if trimmed.starts_with("**Acceptance Criteria:**") || trimmed.starts_with("Acceptance Criteria:") {
                in_criteria = true;
                continue;
            }

            // Next heading ends criteria
            if trimmed.starts_with("### ") || trimmed.starts_with("## ") {
                in_criteria = false;
            }

            // Collect criteria
            if in_criteria && current_story.is_some() {
                if let Some(criterion) = self.extract_criterion(trimmed) {
                    if let Some((_, _, ref mut criteria)) = current_story {
                        criteria.push(criterion);
                    }
                }
            }
        }

        // Save last story
        if let Some((num, title, criteria)) = current_story {
            let story_id = format!("story-{}", num);
            let story = DiscoveredStory::new(&story_id, title, num)
                .with_criteria(criteria);
            stories.push(story);
        }

        stories
    }

    /// Parse story heading
    fn parse_story_heading(&self, line: &str) -> Option<(u32, String)> {
        // Match patterns like "### Story 1: Title" or "### Story 10: Title"
        // Uses pre-compiled regex for performance
        let caps = STORY_HEADING_REGEX.captures(line)?;

        let num = caps.get(1)?.as_str().parse().ok()?;
        let title = caps.get(2)?.as_str().trim().to_string();

        Some((num, title))
    }

    /// Extract a criterion from a line
    fn extract_criterion(&self, line: &str) -> Option<String> {
        // Match "- [ ] criterion" or "- [x] criterion"
        if line.starts_with("- [ ] ") {
            return Some(line[6..].trim().to_string());
        }
        if line.starts_with("- [x] ") || line.starts_with("- [X] ") {
            return Some(line[6..].trim().to_string());
        }
        // Also match plain "- criterion"
        if line.starts_with("- ") && !line.starts_with("- [") {
            return Some(line[2..].trim().to_string());
        }
        None
    }

    /// Match epic against pattern
    pub fn matches_pattern(&self, epic_id: &str, pattern: &str) -> bool {
        // Simple glob-like matching
        if pattern == "*" {
            return true;
        }

        // Exact match
        if pattern == epic_id {
            return true;
        }

        // Prefix match with *
        if pattern.ends_with('*') {
            let prefix = &pattern[..pattern.len() - 1];
            return epic_id.starts_with(prefix);
        }

        // Suffix match
        if pattern.starts_with('*') {
            let suffix = &pattern[1..];
            return epic_id.ends_with(suffix);
        }

        false
    }

    /// Build work queue from discovered epics
    pub fn build_work_queue(&self, epics: &[DiscoveredEpic]) -> (Vec<WorkQueueItem>, StoryDependencyGraph) {
        let mut queue = Vec::new();
        let mut graph = StoryDependencyGraph::new();

        for epic in epics {
            for story in &epic.stories {
                let item = WorkQueueItem::from_story(&epic.id, story);
                graph.add_story(&item.full_id, item.dependencies.clone());
                queue.push(item);
            }
        }

        // Sort by priority (topological order)
        if let Ok(sorted) = graph.topological_sort() {
            let order_map: HashMap<_, _> = sorted.iter().enumerate().map(|(i, id)| (id.clone(), i)).collect();
            queue.sort_by(|a, b| {
                let a_order = order_map.get(&a.full_id).unwrap_or(&usize::MAX);
                let b_order = order_map.get(&b.full_id).unwrap_or(&usize::MAX);
                a_order.cmp(b_order)
            });
        }

        (queue, graph)
    }

    /// Create execution plan from discovered epics
    pub fn create_execution_plan(&self, epics: &[DiscoveredEpic]) -> ExecutionPlan {
        let epic_ids: Vec<_> = epics.iter().map(|e| e.id.clone()).collect();
        let (queue, _graph) = self.build_work_queue(epics);

        // Note: Duration estimation could be improved by using complexity metrics
        // For now, we use a simple 5 minutes per story baseline

        ExecutionPlan::new()
            .with_epics(epic_ids)
            .with_work_queue(queue)
    }
}

impl Default for EpicDiscoveryService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== Status Tests ====================

    #[test]
    fn test_epic_processing_status_roundtrip() {
        let statuses = [
            EpicProcessingStatus::Pending,
            EpicProcessingStatus::Analyzing,
            EpicProcessingStatus::Planned,
            EpicProcessingStatus::InProgress,
            EpicProcessingStatus::Completed,
            EpicProcessingStatus::Blocked,
            EpicProcessingStatus::Skipped,
        ];

        for s in statuses {
            let str = s.as_str();
            let parsed: EpicProcessingStatus = str.parse().unwrap();
            assert_eq!(s, parsed);
        }
    }

    #[test]
    fn test_story_processing_status_roundtrip() {
        let statuses = [
            StoryProcessingStatus::Pending,
            StoryProcessingStatus::Waiting,
            StoryProcessingStatus::InProgress,
            StoryProcessingStatus::AwaitingReview,
            StoryProcessingStatus::AwaitingMerge,
            StoryProcessingStatus::Completed,
            StoryProcessingStatus::Failed,
            StoryProcessingStatus::Blocked,
            StoryProcessingStatus::Skipped,
        ];

        for s in statuses {
            let str = s.as_str();
            let parsed: StoryProcessingStatus = str.parse().unwrap();
            assert_eq!(s, parsed);
        }
    }

    // ==================== DiscoveredEpic Tests ====================

    #[test]
    fn test_discovered_epic_new() {
        let epic = DiscoveredEpic::new("epic-001", "Test Epic", PathBuf::from("test.md"));

        assert_eq!(epic.id, "epic-001");
        assert_eq!(epic.title, "Test Epic");
        assert!(epic.stories.is_empty());
        assert_eq!(epic.status, EpicProcessingStatus::Pending);
    }

    #[test]
    fn test_discovered_epic_completion() {
        let mut epic = DiscoveredEpic::new("epic-001", "Test Epic", PathBuf::from("test.md"));
        epic.stories = vec![
            DiscoveredStory::new("story-1", "Story 1", 1),
            {
                let mut s = DiscoveredStory::new("story-2", "Story 2", 2);
                s.status = StoryProcessingStatus::Completed;
                s
            },
            {
                let mut s = DiscoveredStory::new("story-3", "Story 3", 3);
                s.status = StoryProcessingStatus::Completed;
                s
            },
        ];

        assert_eq!(epic.completed_count(), 2);
        assert_eq!(epic.total_count(), 3);
        assert!((epic.completion_percentage() - 66.67).abs() < 1.0);
    }

    // ==================== StoryDependencyGraph Tests ====================

    #[test]
    fn test_dependency_graph_basic() {
        let mut graph = StoryDependencyGraph::new();
        graph.add_story("story-1", vec![]);
        graph.add_story("story-2", vec!["story-1".to_string()]);
        graph.add_story("story-3", vec!["story-1".to_string(), "story-2".to_string()]);

        assert!(graph.get_dependencies("story-1").is_empty());
        assert_eq!(graph.get_dependencies("story-2"), vec!["story-1"]);
        assert_eq!(graph.get_dependents("story-1").len(), 2);
    }

    #[test]
    fn test_dependency_graph_satisfied() {
        let mut graph = StoryDependencyGraph::new();
        graph.add_story("story-1", vec![]);
        graph.add_story("story-2", vec!["story-1".to_string()]);

        let mut completed = HashSet::new();

        // story-1 has no deps, so satisfied
        assert!(graph.dependencies_satisfied("story-1", &completed));
        // story-2 depends on story-1, not satisfied yet
        assert!(!graph.dependencies_satisfied("story-2", &completed));

        // Complete story-1
        completed.insert("story-1".to_string());
        assert!(graph.dependencies_satisfied("story-2", &completed));
    }

    #[test]
    fn test_dependency_graph_executable() {
        let mut graph = StoryDependencyGraph::new();
        graph.add_story("story-1", vec![]);
        graph.add_story("story-2", vec!["story-1".to_string()]);
        graph.add_story("story-3", vec!["story-2".to_string()]);

        let mut completed = HashSet::new();

        // Only story-1 can execute initially
        let exec = graph.get_executable(&completed);
        assert!(exec.contains(&"story-1".to_string()));
        assert!(!exec.contains(&"story-2".to_string()));

        // After completing story-1
        completed.insert("story-1".to_string());
        let exec = graph.get_executable(&completed);
        assert!(exec.contains(&"story-2".to_string()));
    }

    #[test]
    fn test_dependency_graph_no_cycle() {
        let mut graph = StoryDependencyGraph::new();
        graph.add_story("a", vec![]);
        graph.add_story("b", vec!["a".to_string()]);
        graph.add_story("c", vec!["b".to_string()]);

        assert!(graph.detect_cycles().is_none());
    }

    #[test]
    fn test_dependency_graph_cycle() {
        let mut graph = StoryDependencyGraph::new();
        graph.add_story("a", vec!["c".to_string()]);
        graph.add_story("b", vec!["a".to_string()]);
        graph.add_story("c", vec!["b".to_string()]);

        assert!(graph.detect_cycles().is_some());
    }

    #[test]
    fn test_dependency_graph_topological_sort() {
        let mut graph = StoryDependencyGraph::new();
        graph.add_story("story-3", vec!["story-2".to_string()]);
        graph.add_story("story-2", vec!["story-1".to_string()]);
        graph.add_story("story-1", vec![]);

        let sorted = graph.topological_sort().unwrap();

        // story-1 should come before story-2, story-2 before story-3
        let pos1 = sorted.iter().position(|s| s == "story-1").unwrap();
        let pos2 = sorted.iter().position(|s| s == "story-2").unwrap();
        let pos3 = sorted.iter().position(|s| s == "story-3").unwrap();

        assert!(pos1 < pos2);
        assert!(pos2 < pos3);
    }

    // ==================== EpicDiscoveryService Tests ====================

    #[test]
    fn test_parse_epic_title() {
        let service = EpicDiscoveryService::new();
        let content = r#"
# Epic 001: Test Epic

Some content here.
"#;

        let epic = service.parse_epic("epic-001", content, PathBuf::from("test.md"));
        assert_eq!(epic.title, "Epic 001: Test Epic");
    }

    #[test]
    fn test_parse_epic_stories() {
        let service = EpicDiscoveryService::new();
        let content = r#"
# Epic 001: Test Epic

## Overview

This is a test epic.

## Stories

### Story 1: First Story

Some description.

**Acceptance Criteria:**
- [ ] Criterion 1
- [ ] Criterion 2

### Story 2: Second Story

**Acceptance Criteria:**
- [ ] Criterion A
- [ ] Criterion B
- [ ] Criterion C
"#;

        let epic = service.parse_epic("epic-001", content, PathBuf::from("test.md"));

        assert_eq!(epic.stories.len(), 2);
        assert_eq!(epic.stories[0].title, "First Story");
        assert_eq!(epic.stories[0].acceptance_criteria.len(), 2);
        assert_eq!(epic.stories[1].title, "Second Story");
        assert_eq!(epic.stories[1].acceptance_criteria.len(), 3);
    }

    #[test]
    fn test_parse_epic_overview() {
        let service = EpicDiscoveryService::new();
        let content = r#"
# Epic 001: Test

## Overview

This is the overview content.
More details here.

## Stories
"#;

        let epic = service.parse_epic("epic-001", content, PathBuf::from("test.md"));

        assert!(epic.description.is_some());
        assert!(epic.description.unwrap().contains("overview content"));
    }

    #[test]
    fn test_matches_pattern() {
        let service = EpicDiscoveryService::new();

        assert!(service.matches_pattern("epic-001", "*"));
        assert!(service.matches_pattern("epic-001", "epic-001"));
        assert!(service.matches_pattern("epic-001", "epic-*"));
        assert!(service.matches_pattern("epic-001", "*-001"));
        assert!(!service.matches_pattern("epic-001", "epic-002"));
        assert!(!service.matches_pattern("epic-001", "other-*"));
    }

    #[test]
    fn test_build_work_queue() {
        let service = EpicDiscoveryService::new();

        let mut story1 = DiscoveredStory::new("story-1", "Story 1", 1);
        let mut story2 = DiscoveredStory::new("story-2", "Story 2", 2);
        story2.dependencies = vec!["story-1".to_string()];

        let epic = DiscoveredEpic::new("epic-001", "Test", PathBuf::from("test.md"))
            .with_stories(vec![story1, story2]);

        let (queue, graph) = service.build_work_queue(&[epic]);

        assert_eq!(queue.len(), 2);
        // First item should be story-1 (no deps)
        assert!(queue[0].full_id.contains("story-1"));
    }

    #[test]
    fn test_create_execution_plan() {
        let service = EpicDiscoveryService::new();

        let epic = DiscoveredEpic::new("epic-001", "Test", PathBuf::from("test.md"))
            .with_stories(vec![
                DiscoveredStory::new("story-1", "Story 1", 1),
                DiscoveredStory::new("story-2", "Story 2", 2),
            ]);

        let plan = service.create_execution_plan(&[epic]);

        assert_eq!(plan.epics.len(), 1);
        assert_eq!(plan.total_stories, 2);
    }

    // ==================== WorkQueueItem Tests ====================

    #[test]
    fn test_work_queue_item_can_execute() {
        let story = DiscoveredStory::new("story-2", "Story 2", 2)
            .with_dependencies(vec!["story-1".to_string()]);

        let item = WorkQueueItem::from_story("epic-001", &story);

        let mut completed = HashSet::new();
        assert!(!item.can_execute(&completed));

        completed.insert("epic-001/story-1".to_string());
        assert!(item.can_execute(&completed));
    }
}
