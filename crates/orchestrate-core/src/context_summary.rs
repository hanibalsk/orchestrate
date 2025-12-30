//! Context Summarization Protocol
//!
//! Token-efficient context handoffs between agents. Provides structured
//! summaries of agent work that can be passed to controllers or successor
//! agents without consuming excessive context tokens.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A structured summary of an agent's work
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextSummary {
    /// Agent ID that produced this summary
    pub agent_id: Option<String>,
    /// Session ID for continuity
    pub session_id: Option<String>,
    /// When the summary was created
    pub created_at: DateTime<Utc>,
    /// Key decisions made during the work
    pub key_decisions: Vec<KeyDecision>,
    /// Files that were changed
    pub files_changed: Vec<FileChange>,
    /// Tests that were added or modified
    pub tests_added: Vec<TestAdded>,
    /// Blockers or issues encountered
    pub blockers: Vec<Blocker>,
    /// Overall status of the work
    pub status: WorkStatus,
    /// Brief summary text (human-readable)
    pub summary_text: String,
    /// Original token count (before summarization)
    pub original_tokens: Option<u64>,
    /// Summary token count
    pub summary_tokens: Option<u64>,
}

impl ContextSummary {
    /// Create a new empty summary
    pub fn new() -> Self {
        Self {
            created_at: Utc::now(),
            status: WorkStatus::InProgress,
            ..Default::default()
        }
    }

    /// Set the agent ID
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set the session ID
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set the status
    pub fn with_status(mut self, status: WorkStatus) -> Self {
        self.status = status;
        self
    }

    /// Add a key decision
    pub fn add_decision(&mut self, decision: KeyDecision) {
        self.key_decisions.push(decision);
    }

    /// Add a file change
    pub fn add_file_change(&mut self, change: FileChange) {
        self.files_changed.push(change);
    }

    /// Add a test
    pub fn add_test(&mut self, test: TestAdded) {
        self.tests_added.push(test);
    }

    /// Add a blocker
    pub fn add_blocker(&mut self, blocker: Blocker) {
        self.blockers.push(blocker);
    }

    /// Set the summary text
    pub fn set_summary(&mut self, summary: impl Into<String>) {
        self.summary_text = summary.into();
    }

    /// Record token savings
    pub fn record_tokens(&mut self, original: u64, summary: u64) {
        self.original_tokens = Some(original);
        self.summary_tokens = Some(summary);
    }

    /// Calculate token savings percentage
    pub fn token_savings_percent(&self) -> Option<f64> {
        match (self.original_tokens, self.summary_tokens) {
            (Some(orig), Some(sum)) if orig > 0 => {
                Some(((orig - sum) as f64 / orig as f64) * 100.0)
            }
            _ => None,
        }
    }

    /// Convert to JSON for machine parsing
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    /// Convert to compact JSON
    pub fn to_compact_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    /// Convert to human-readable markdown format
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Header
        md.push_str("## Work Summary\n\n");

        // Status
        md.push_str(&format!("**Status:** {}\n\n", self.status.as_str()));

        // Summary text
        if !self.summary_text.is_empty() {
            md.push_str(&format!("{}\n\n", self.summary_text));
        }

        // Key decisions
        if !self.key_decisions.is_empty() {
            md.push_str("### Key Decisions\n\n");
            for decision in &self.key_decisions {
                md.push_str(&format!(
                    "- **{}**: {}\n",
                    decision.category.as_str(),
                    decision.description
                ));
                if let Some(ref rationale) = decision.rationale {
                    md.push_str(&format!("  - Rationale: {}\n", rationale));
                }
            }
            md.push('\n');
        }

        // Files changed
        if !self.files_changed.is_empty() {
            md.push_str("### Files Changed\n\n");
            for file in &self.files_changed {
                md.push_str(&format!(
                    "- `{}` ({})\n",
                    file.path,
                    file.change_type.as_str()
                ));
                if let Some(ref desc) = file.description {
                    md.push_str(&format!("  - {}\n", desc));
                }
            }
            md.push('\n');
        }

        // Tests
        if !self.tests_added.is_empty() {
            md.push_str("### Tests Added/Modified\n\n");
            for test in &self.tests_added {
                md.push_str(&format!("- `{}`", test.name));
                if let Some(ref desc) = test.description {
                    md.push_str(&format!(": {}", desc));
                }
                md.push('\n');
            }
            md.push('\n');
        }

        // Blockers
        if !self.blockers.is_empty() {
            md.push_str("### Blockers\n\n");
            for blocker in &self.blockers {
                md.push_str(&format!(
                    "- **[{}]** {}\n",
                    blocker.severity.as_str(),
                    blocker.description
                ));
                if let Some(ref suggestion) = blocker.suggested_action {
                    md.push_str(&format!("  - Suggested action: {}\n", suggestion));
                }
            }
            md.push('\n');
        }

        // Token savings
        if let Some(savings) = self.token_savings_percent() {
            md.push_str(&format!(
                "---\n*Token savings: {:.1}% ({} -> {} tokens)*\n",
                savings,
                self.original_tokens.unwrap_or(0),
                self.summary_tokens.unwrap_or(0)
            ));
        }

        md
    }

    /// Check if the work has blockers
    pub fn has_blockers(&self) -> bool {
        !self.blockers.is_empty()
    }

    /// Check if the work has critical blockers
    pub fn has_critical_blockers(&self) -> bool {
        self.blockers
            .iter()
            .any(|b| matches!(b.severity, BlockerSeverity::Critical))
    }

    /// Get all file paths that were changed
    pub fn changed_file_paths(&self) -> Vec<&str> {
        self.files_changed.iter().map(|f| f.path.as_str()).collect()
    }

    /// Get all test names
    pub fn test_names(&self) -> Vec<&str> {
        self.tests_added.iter().map(|t| t.name.as_str()).collect()
    }
}

/// Status of the work
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkStatus {
    /// Work is still in progress
    #[default]
    InProgress,
    /// Work completed successfully
    Completed,
    /// Work blocked by issues
    Blocked,
    /// Work failed
    Failed,
    /// Waiting for external event
    Waiting,
}

impl WorkStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InProgress => "In Progress",
            Self::Completed => "Completed",
            Self::Blocked => "Blocked",
            Self::Failed => "Failed",
            Self::Waiting => "Waiting",
        }
    }
}

/// A key decision made during work
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDecision {
    /// Category of decision
    pub category: DecisionCategory,
    /// Description of the decision
    pub description: String,
    /// Rationale (why this choice was made)
    pub rationale: Option<String>,
    /// Alternatives considered
    pub alternatives: Vec<String>,
}

impl KeyDecision {
    /// Create a new decision
    pub fn new(category: DecisionCategory, description: impl Into<String>) -> Self {
        Self {
            category,
            description: description.into(),
            rationale: None,
            alternatives: Vec::new(),
        }
    }

    /// Add rationale
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = Some(rationale.into());
        self
    }

    /// Add alternatives
    pub fn with_alternatives(mut self, alternatives: Vec<String>) -> Self {
        self.alternatives = alternatives;
        self
    }
}

/// Categories of decisions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionCategory {
    /// Architectural/design decision
    Architecture,
    /// Implementation approach
    Implementation,
    /// Technology/library choice
    Technology,
    /// API design decision
    Api,
    /// Performance trade-off
    Performance,
    /// Security decision
    Security,
    /// Testing strategy
    Testing,
    /// Other decision type
    Other,
}

impl DecisionCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Architecture => "Architecture",
            Self::Implementation => "Implementation",
            Self::Technology => "Technology",
            Self::Api => "API",
            Self::Performance => "Performance",
            Self::Security => "Security",
            Self::Testing => "Testing",
            Self::Other => "Other",
        }
    }
}

/// A file that was changed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    /// Path to the file
    pub path: String,
    /// Type of change
    pub change_type: FileChangeType,
    /// Brief description of change
    pub description: Option<String>,
    /// Lines added
    pub lines_added: Option<u32>,
    /// Lines removed
    pub lines_removed: Option<u32>,
}

impl FileChange {
    /// Create a new file change
    pub fn new(path: impl Into<String>, change_type: FileChangeType) -> Self {
        Self {
            path: path.into(),
            change_type,
            description: None,
            lines_added: None,
            lines_removed: None,
        }
    }

    /// Add description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set line counts
    pub fn with_lines(mut self, added: u32, removed: u32) -> Self {
        self.lines_added = Some(added);
        self.lines_removed = Some(removed);
        self
    }
}

/// Types of file changes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeType {
    /// New file created
    Created,
    /// Existing file modified
    Modified,
    /// File deleted
    Deleted,
    /// File renamed/moved
    Renamed,
}

impl FileChangeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Modified => "modified",
            Self::Deleted => "deleted",
            Self::Renamed => "renamed",
        }
    }
}

/// A test that was added or modified
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAdded {
    /// Test name/function
    pub name: String,
    /// Test file path
    pub file_path: Option<String>,
    /// Description of what it tests
    pub description: Option<String>,
    /// Test type
    pub test_type: TestType,
}

impl TestAdded {
    /// Create a new test record
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            file_path: None,
            description: None,
            test_type: TestType::Unit,
        }
    }

    /// Set the file path
    pub fn with_file(mut self, path: impl Into<String>) -> Self {
        self.file_path = Some(path.into());
        self
    }

    /// Set the description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set the test type
    pub fn with_type(mut self, test_type: TestType) -> Self {
        self.test_type = test_type;
        self
    }
}

/// Types of tests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    #[default]
    Unit,
    Integration,
    E2e,
    Property,
}

/// A blocker or issue encountered
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    /// Description of the blocker
    pub description: String,
    /// Severity level
    pub severity: BlockerSeverity,
    /// Blocker type/category
    pub blocker_type: BlockerType,
    /// Suggested action to resolve
    pub suggested_action: Option<String>,
}

impl Blocker {
    /// Create a new blocker
    pub fn new(
        description: impl Into<String>,
        severity: BlockerSeverity,
        blocker_type: BlockerType,
    ) -> Self {
        Self {
            description: description.into(),
            severity,
            blocker_type,
            suggested_action: None,
        }
    }

    /// Add suggested action
    pub fn with_suggestion(mut self, action: impl Into<String>) -> Self {
        self.suggested_action = Some(action.into());
        self
    }
}

/// Severity levels for blockers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerSeverity {
    /// Minor issue, can continue
    Low,
    /// Moderate issue, should address
    Medium,
    /// Significant issue, needs attention
    High,
    /// Critical blocker, cannot continue
    Critical,
}

impl BlockerSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "LOW",
            Self::Medium => "MEDIUM",
            Self::High => "HIGH",
            Self::Critical => "CRITICAL",
        }
    }
}

/// Types of blockers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerType {
    /// Missing dependency or requirement
    Dependency,
    /// Test failure
    TestFailure,
    /// Build/compilation error
    BuildError,
    /// Merge conflict
    MergeConflict,
    /// CI failure
    CiFailure,
    /// Review required
    ReviewRequired,
    /// Missing information
    MissingInfo,
    /// Permission issue
    Permission,
    /// Other blocker type
    Other,
}

/// Summarizer for extracting summaries from agent output
pub struct OutputSummarizer {
    /// Token estimator for tracking savings
    estimate_tokens: bool,
}

impl Default for OutputSummarizer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputSummarizer {
    /// Create a new summarizer
    pub fn new() -> Self {
        Self {
            estimate_tokens: true,
        }
    }

    /// Disable token estimation
    pub fn without_token_estimation(mut self) -> Self {
        self.estimate_tokens = false;
        self
    }

    /// Extract a summary from agent output text
    pub fn summarize_output(&self, output: &str) -> ContextSummary {
        let mut summary = ContextSummary::new();

        // Extract status
        summary.status = self.extract_status(output);

        // Extract files changed
        for file in self.extract_files_changed(output) {
            summary.add_file_change(file);
        }

        // Extract tests
        for test in self.extract_tests(output) {
            summary.add_test(test);
        }

        // Extract blockers
        for blocker in self.extract_blockers(output) {
            summary.add_blocker(blocker);
        }

        // Extract key decisions
        for decision in self.extract_decisions(output) {
            summary.add_decision(decision);
        }

        // Generate summary text
        summary.set_summary(self.generate_summary_text(output, &summary));

        // Estimate tokens
        if self.estimate_tokens {
            let original_tokens = self.estimate_token_count(output);
            let summary_tokens = self.estimate_token_count(&summary.to_compact_json().unwrap_or_default());
            summary.record_tokens(original_tokens, summary_tokens);
        }

        summary
    }

    /// Extract status from output
    fn extract_status(&self, output: &str) -> WorkStatus {
        if output.contains("STATUS: COMPLETE") {
            WorkStatus::Completed
        } else if output.contains("STATUS: BLOCKED") {
            WorkStatus::Blocked
        } else if output.contains("STATUS: WAITING") {
            WorkStatus::Waiting
        } else if output.contains("STATUS: ERROR") || output.contains("STATUS: FAILED") {
            WorkStatus::Failed
        } else {
            WorkStatus::InProgress
        }
    }

    /// Extract file changes from output
    fn extract_files_changed(&self, output: &str) -> Vec<FileChange> {
        let mut files = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Pattern for created files
        let created_patterns = [
            r"(?i)(?:Created|Wrote|Added)\s+(?:file\s+)?[`']?([^\s`']+\.\w+)[`']?",
            r"(?i)Write tool.*?[`']([^\s`']+\.\w+)[`']",
        ];

        for pattern in &created_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for caps in re.captures_iter(output) {
                    if let Some(m) = caps.get(1) {
                        let path = m.as_str().to_string();
                        if !seen.contains(&path) && self.is_valid_path(&path) {
                            seen.insert(path.clone());
                            files.push(FileChange::new(path, FileChangeType::Created));
                        }
                    }
                }
            }
        }

        // Pattern for modified files
        let modified_patterns = [
            r"(?i)(?:Modified|Updated|Edited|Changed)\s+(?:file\s+)?[`']?([^\s`']+\.\w+)[`']?",
            r"(?i)Edit tool.*?[`']([^\s`']+\.\w+)[`']",
        ];

        for pattern in &modified_patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for caps in re.captures_iter(output) {
                    if let Some(m) = caps.get(1) {
                        let path = m.as_str().to_string();
                        if !seen.contains(&path) && self.is_valid_path(&path) {
                            seen.insert(path.clone());
                            files.push(FileChange::new(path, FileChangeType::Modified));
                        }
                    }
                }
            }
        }

        files
    }

    /// Extract tests from output
    fn extract_tests(&self, output: &str) -> Vec<TestAdded> {
        let mut tests = Vec::new();
        let mut seen = std::collections::HashSet::new();

        let patterns = [
            r"(?:#\[test\]|#\[tokio::test\])\s*(?:async\s+)?fn\s+(\w+)",
            r"(?i)added test[s]?:?\s*[`']?(\w+)[`']?",
            r"test\s+(\w+)\s+\.\.\.\s+(?:ok|FAILED)",
        ];

        for pattern in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for caps in re.captures_iter(output) {
                    if let Some(m) = caps.get(1) {
                        let name = m.as_str().to_string();
                        if !seen.contains(&name) {
                            seen.insert(name.clone());
                            tests.push(TestAdded::new(name));
                        }
                    }
                }
            }
        }

        tests
    }

    /// Extract blockers from output
    fn extract_blockers(&self, output: &str) -> Vec<Blocker> {
        let mut blockers = Vec::new();

        // Check for explicit blocked status
        if let Some(pos) = output.find("STATUS: BLOCKED") {
            let after = &output[pos + "STATUS: BLOCKED".len()..];
            // Check for pattern like "STATUS: BLOCKED - reason" or "STATUS: BLOCKED: reason"
            let reason = after
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .trim_start_matches('-')
                .trim_start_matches(':')
                .trim();
            if !reason.is_empty() {
                blockers.push(Blocker::new(
                    reason,
                    BlockerSeverity::High,
                    BlockerType::Other,
                ));
            }
        }

        // Check for test failures
        if output.contains("FAILED") && output.contains("test") {
            let pattern = r"test\s+(\w+)\s+\.\.\.\s+FAILED";
            if let Ok(re) = regex::Regex::new(pattern) {
                for caps in re.captures_iter(output) {
                    if let Some(m) = caps.get(1) {
                        blockers.push(Blocker::new(
                            format!("Test '{}' failed", m.as_str()),
                            BlockerSeverity::High,
                            BlockerType::TestFailure,
                        ));
                    }
                }
            }
        }

        // Check for build errors
        if output.contains("error[E") || output.contains("error: ") {
            blockers.push(Blocker::new(
                "Build/compilation errors detected",
                BlockerSeverity::High,
                BlockerType::BuildError,
            ));
        }

        blockers
    }

    /// Extract key decisions from output
    fn extract_decisions(&self, output: &str) -> Vec<KeyDecision> {
        let mut decisions = Vec::new();

        // Look for decision patterns
        let patterns = [
            (r"(?i)decided to\s+(.+?)(?:\.|$)", DecisionCategory::Implementation),
            (r"(?i)chose\s+(.+?)\s+(?:because|for|to)", DecisionCategory::Technology),
            (r"(?i)implemented\s+(.+?)\s+(?:using|with|via)", DecisionCategory::Implementation),
        ];

        for (pattern, category) in &patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for caps in re.captures_iter(output) {
                    if let Some(m) = caps.get(1) {
                        let desc = m.as_str().trim().to_string();
                        if desc.len() > 10 && desc.len() < 200 {
                            decisions.push(KeyDecision::new(*category, desc));
                        }
                    }
                }
            }
        }

        // Limit to most relevant decisions
        decisions.truncate(5);
        decisions
    }

    /// Generate a summary text
    fn generate_summary_text(&self, _output: &str, summary: &ContextSummary) -> String {
        let mut text = String::new();

        let file_count = summary.files_changed.len();
        let test_count = summary.tests_added.len();
        let blocker_count = summary.blockers.len();

        text.push_str(&match summary.status {
            WorkStatus::Completed => "Work completed successfully.".to_string(),
            WorkStatus::Blocked => format!("Work blocked ({} issues).", blocker_count),
            WorkStatus::Failed => "Work failed.".to_string(),
            WorkStatus::Waiting => "Waiting for external event.".to_string(),
            WorkStatus::InProgress => "Work in progress.".to_string(),
        });

        if file_count > 0 {
            text.push_str(&format!(" {} file(s) changed.", file_count));
        }

        if test_count > 0 {
            text.push_str(&format!(" {} test(s) added/modified.", test_count));
        }

        text
    }

    /// Check if a path looks valid
    fn is_valid_path(&self, path: &str) -> bool {
        !path.is_empty()
            && path.len() < 500
            && path.contains('.')
            && !path.contains("```")
            && !path.contains("  ")
    }

    /// Estimate token count (rough approximation)
    fn estimate_token_count(&self, text: &str) -> u64 {
        // Rough estimate: ~4 characters per token for English text
        (text.len() as f64 / 4.0).ceil() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== ContextSummary Tests ====================

    #[test]
    fn test_context_summary_new() {
        let summary = ContextSummary::new();
        assert_eq!(summary.status, WorkStatus::InProgress);
        assert!(summary.key_decisions.is_empty());
        assert!(summary.files_changed.is_empty());
        assert!(summary.tests_added.is_empty());
        assert!(summary.blockers.is_empty());
    }

    #[test]
    fn test_context_summary_with_agent() {
        let summary = ContextSummary::new()
            .with_agent("agent-123")
            .with_session("session-456");

        assert_eq!(summary.agent_id, Some("agent-123".to_string()));
        assert_eq!(summary.session_id, Some("session-456".to_string()));
    }

    #[test]
    fn test_context_summary_add_items() {
        let mut summary = ContextSummary::new();

        summary.add_decision(KeyDecision::new(
            DecisionCategory::Architecture,
            "Use microservices",
        ));
        summary.add_file_change(FileChange::new("src/lib.rs", FileChangeType::Modified));
        summary.add_test(TestAdded::new("test_feature"));
        summary.add_blocker(Blocker::new(
            "Missing dependency",
            BlockerSeverity::Medium,
            BlockerType::Dependency,
        ));

        assert_eq!(summary.key_decisions.len(), 1);
        assert_eq!(summary.files_changed.len(), 1);
        assert_eq!(summary.tests_added.len(), 1);
        assert_eq!(summary.blockers.len(), 1);
    }

    #[test]
    fn test_context_summary_token_savings() {
        let mut summary = ContextSummary::new();
        summary.record_tokens(1000, 200);

        let savings = summary.token_savings_percent().unwrap();
        assert!((savings - 80.0).abs() < 0.1);
    }

    #[test]
    fn test_context_summary_to_json() {
        let summary = ContextSummary::new()
            .with_agent("agent-123")
            .with_status(WorkStatus::Completed);

        let json = summary.to_json().unwrap();
        assert!(json.contains("agent-123"));
        assert!(json.contains("completed"));
    }

    #[test]
    fn test_context_summary_to_markdown() {
        let mut summary = ContextSummary::new();
        summary.status = WorkStatus::Completed;
        summary.set_summary("Task completed successfully.");
        summary.add_file_change(
            FileChange::new("src/lib.rs", FileChangeType::Modified)
                .with_description("Added new function"),
        );
        summary.add_test(TestAdded::new("test_new_feature"));

        let md = summary.to_markdown();
        assert!(md.contains("## Work Summary"));
        assert!(md.contains("Completed"));
        assert!(md.contains("src/lib.rs"));
        assert!(md.contains("test_new_feature"));
    }

    #[test]
    fn test_context_summary_has_blockers() {
        let mut summary = ContextSummary::new();
        assert!(!summary.has_blockers());

        summary.add_blocker(Blocker::new(
            "Test failure",
            BlockerSeverity::Medium,
            BlockerType::TestFailure,
        ));
        assert!(summary.has_blockers());
        assert!(!summary.has_critical_blockers());

        summary.add_blocker(Blocker::new(
            "Critical error",
            BlockerSeverity::Critical,
            BlockerType::BuildError,
        ));
        assert!(summary.has_critical_blockers());
    }

    #[test]
    fn test_context_summary_file_paths() {
        let mut summary = ContextSummary::new();
        summary.add_file_change(FileChange::new("src/lib.rs", FileChangeType::Modified));
        summary.add_file_change(FileChange::new("src/main.rs", FileChangeType::Created));

        let paths = summary.changed_file_paths();
        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&"src/lib.rs"));
        assert!(paths.contains(&"src/main.rs"));
    }

    // ==================== KeyDecision Tests ====================

    #[test]
    fn test_key_decision() {
        let decision = KeyDecision::new(DecisionCategory::Architecture, "Use event sourcing")
            .with_rationale("Better audit trail")
            .with_alternatives(vec!["CRUD".to_string(), "Event-driven".to_string()]);

        assert_eq!(decision.category, DecisionCategory::Architecture);
        assert_eq!(decision.rationale, Some("Better audit trail".to_string()));
        assert_eq!(decision.alternatives.len(), 2);
    }

    // ==================== FileChange Tests ====================

    #[test]
    fn test_file_change() {
        let change = FileChange::new("src/lib.rs", FileChangeType::Modified)
            .with_description("Added new module")
            .with_lines(50, 10);

        assert_eq!(change.path, "src/lib.rs");
        assert_eq!(change.change_type, FileChangeType::Modified);
        assert_eq!(change.lines_added, Some(50));
        assert_eq!(change.lines_removed, Some(10));
    }

    // ==================== TestAdded Tests ====================

    #[test]
    fn test_test_added() {
        let test = TestAdded::new("test_feature")
            .with_file("tests/integration.rs")
            .with_description("Test the new feature")
            .with_type(TestType::Integration);

        assert_eq!(test.name, "test_feature");
        assert_eq!(test.file_path, Some("tests/integration.rs".to_string()));
        assert_eq!(test.test_type, TestType::Integration);
    }

    // ==================== Blocker Tests ====================

    #[test]
    fn test_blocker() {
        let blocker = Blocker::new(
            "Missing API key",
            BlockerSeverity::High,
            BlockerType::MissingInfo,
        )
        .with_suggestion("Add API key to environment variables");

        assert_eq!(blocker.description, "Missing API key");
        assert_eq!(blocker.severity, BlockerSeverity::High);
        assert!(blocker.suggested_action.is_some());
    }

    // ==================== OutputSummarizer Tests ====================

    #[test]
    fn test_summarizer_extract_status() {
        let summarizer = OutputSummarizer::new();

        let output1 = "Task done.\n\nSTATUS: COMPLETE";
        let summary1 = summarizer.summarize_output(output1);
        assert_eq!(summary1.status, WorkStatus::Completed);

        let output2 = "Cannot proceed.\n\nSTATUS: BLOCKED - Missing credentials";
        let summary2 = summarizer.summarize_output(output2);
        assert_eq!(summary2.status, WorkStatus::Blocked);

        let output3 = "Waiting for CI.\n\nSTATUS: WAITING";
        let summary3 = summarizer.summarize_output(output3);
        assert_eq!(summary3.status, WorkStatus::Waiting);
    }

    #[test]
    fn test_summarizer_extract_files() {
        let summarizer = OutputSummarizer::new();

        let output = r#"
I made the following changes:
- Created file `src/new_module.rs`
- Modified `src/lib.rs` to add exports
- Updated `Cargo.toml`
        "#;

        let summary = summarizer.summarize_output(output);
        assert!(!summary.files_changed.is_empty());

        let paths = summary.changed_file_paths();
        assert!(paths.contains(&"src/new_module.rs"));
    }

    #[test]
    fn test_summarizer_extract_tests() {
        let summarizer = OutputSummarizer::new();

        let output = r#"
Running tests:
test test_create ... ok
test test_update ... ok
test test_delete ... FAILED

#[test]
fn test_new_feature() {
    // ...
}
        "#;

        let summary = summarizer.summarize_output(output);
        assert!(!summary.tests_added.is_empty());

        let names = summary.test_names();
        assert!(names.contains(&"test_create"));
        assert!(names.contains(&"test_update"));
    }

    #[test]
    fn test_summarizer_extract_blockers() {
        let summarizer = OutputSummarizer::new();

        let output = "Cannot continue.\n\nSTATUS: BLOCKED - Missing API credentials";

        let summary = summarizer.summarize_output(output);
        assert!(summary.has_blockers());
        assert_eq!(summary.blockers[0].description, "Missing API credentials");
    }

    #[test]
    fn test_summarizer_token_estimation() {
        let summarizer = OutputSummarizer::new();

        let long_output = "a".repeat(4000); // ~1000 tokens
        let summary = summarizer.summarize_output(&long_output);

        assert!(summary.original_tokens.is_some());
        assert!(summary.summary_tokens.is_some());
        assert!(summary.original_tokens.unwrap() > summary.summary_tokens.unwrap());
    }

    #[test]
    fn test_summarizer_full_output() {
        let summarizer = OutputSummarizer::new();

        let output = r#"
I have implemented the feature as requested:

1. Created the new module in `src/feature.rs`
2. Modified `src/lib.rs` to export the module
3. Added comprehensive tests

All tests pass:
test test_feature_create ... ok
test test_feature_update ... ok
test test_feature_delete ... ok

STATUS: COMPLETE
        "#;

        let summary = summarizer.summarize_output(output);

        assert_eq!(summary.status, WorkStatus::Completed);
        assert!(!summary.files_changed.is_empty());
        assert!(!summary.tests_added.is_empty());
        assert!(!summary.summary_text.is_empty());
    }
}
