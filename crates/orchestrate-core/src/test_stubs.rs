//! Stub types for incomplete features
//!
//! These are placeholder types for features that were planned but not fully implemented.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Test framework type (stub)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestFramework {
    CargoTest,
    Jest,
    Pytest,
    Mocha,
    Vitest,
}

impl TestFramework {
    /// Get framework from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(TestFramework::CargoTest),
            "ts" | "tsx" | "js" | "jsx" => Some(TestFramework::Jest),
            "py" => Some(TestFramework::Pytest),
            _ => None,
        }
    }

    /// Get framework name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            TestFramework::CargoTest => "cargo test",
            TestFramework::Jest => "jest",
            TestFramework::Pytest => "pytest",
            TestFramework::Mocha => "mocha",
            TestFramework::Vitest => "vitest",
        }
    }
}

/// Generated test (stub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTest {
    pub name: String,
    pub test_type: crate::test_generation::TestType,
    pub framework: TestFramework,
    pub target_file: String,
    pub test_code: String,
}

impl GeneratedTest {
    pub fn new(
        name: impl Into<String>,
        test_type: crate::test_generation::TestType,
        framework: TestFramework,
        target_file: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            test_type,
            framework,
            target_file: target_file.into(),
            test_code: String::new(),
        }
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.test_code = code.into();
        self
    }
}

/// Coverage report (stub)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageReport {
    pub project: String,
    pub target_percentage: f64,
    pub modules: Vec<ModuleCoverage>,
}

impl CoverageReport {
    pub fn new(project: impl Into<String>) -> Self {
        Self {
            project: project.into(),
            target_percentage: 80.0,
            modules: Vec::new(),
        }
    }

    pub fn add_module(&mut self, module: ModuleCoverage) {
        self.modules.push(module);
    }

    pub fn meets_target(&self) -> bool {
        self.overall_percentage() >= self.target_percentage
    }

    pub fn overall_percentage(&self) -> f64 {
        if self.modules.is_empty() {
            return 0.0;
        }
        let total_lines: u64 = self.modules.iter().map(|m| m.total_lines).sum();
        let covered_lines: u64 = self.modules.iter().map(|m| m.covered_lines).sum();
        if total_lines == 0 {
            return 0.0;
        }
        (covered_lines as f64 / total_lines as f64) * 100.0
    }

    pub fn to_summary(&self) -> String {
        let mut result = String::new();
        result.push_str(&format!("Coverage Report: {}\n", self.project));
        result.push_str(&format!("Target: {:.1}%\n\n", self.target_percentage));
        for module in &self.modules {
            result.push_str(&format!("  {} ({:.1}%)\n", module.name, module.percentage()));
        }
        result.push_str(&format!("\nOverall: {:.1}%\n", self.overall_percentage()));
        result
    }
}

/// Module coverage (stub)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModuleCoverage {
    pub name: String,
    pub path: String,
    pub files: Vec<FileCoverage>,
    pub total_lines: u64,
    pub covered_lines: u64,
}

impl ModuleCoverage {
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            files: Vec::new(),
            total_lines: 0,
            covered_lines: 0,
        }
    }

    pub fn add_file(&mut self, file: FileCoverage) {
        self.total_lines += file.total_lines;
        self.covered_lines += file.covered_lines;
        self.files.push(file);
    }

    pub fn percentage(&self) -> f64 {
        if self.total_lines == 0 {
            0.0
        } else {
            (self.covered_lines as f64 / self.total_lines as f64) * 100.0
        }
    }
}

/// File coverage (stub)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileCoverage {
    pub path: String,
    pub total_lines: u64,
    pub covered_lines: u64,
}

impl FileCoverage {
    pub fn new(path: impl Into<String>, total_lines: u64, covered_lines: u64) -> Self {
        Self {
            path: path.into(),
            total_lines,
            covered_lines,
        }
    }
}

/// Test run (stub)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestRun {
    pub id: String,
    pub status: TestRunStatus,
    pub results: Vec<TestResult>,
    pub test_results: Vec<TestResult>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub total_tests: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_seconds: f64,
}

impl TestRun {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status: TestRunStatus::Pending,
            results: Vec::new(),
            test_results: Vec::new(),
            started_at: None,
            completed_at: None,
            total_tests: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_seconds: 0.0,
        }
    }

    pub fn add_result(&mut self, result: TestResult) {
        self.total_tests += 1;
        match result.status {
            TestResultStatus::Passed => self.passed += 1,
            TestResultStatus::Failed => self.failed += 1,
            TestResultStatus::Skipped => self.skipped += 1,
        }
        self.results.push(result.clone());
        self.test_results.push(result);
    }

    pub fn complete(&mut self) {
        self.status = TestRunStatus::Completed;
        self.completed_at = Some(chrono::Utc::now());
    }
}

/// Test run status (stub)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestRunStatus {
    #[default]
    Pending,
    Running,
    Passed,
    Failed,
    Completed,
}

/// Test result (stub)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub status: TestResultStatus,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub error_message: Option<String>,
    pub stack_trace: Option<String>,
}

impl TestResult {
    pub fn new(name: impl Into<String>, status: TestResultStatus) -> Self {
        Self {
            name: name.into(),
            status,
            duration_ms: 0,
            error: None,
            error_message: None,
            stack_trace: None,
        }
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    pub fn with_error(mut self, msg: impl Into<String>) -> Self {
        let msg = msg.into();
        self.error = Some(msg.clone());
        self.error_message = Some(msg);
        self
    }
}

/// Test result status (stub)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TestResultStatus {
    #[default]
    Passed,
    Failed,
    Skipped,
}

/// Test quality report (stub)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TestQualityReport {
    pub issues: Vec<TestQualityIssue>,
    pub suggestions: Vec<String>,
    pub score: f64,
    pub total_tests: u32,
    pub mutation_score: f64,
}

impl TestQualityReport {
    pub fn new() -> Self {
        Self {
            issues: Vec::new(),
            suggestions: Vec::new(),
            score: 100.0,
            total_tests: 0,
            mutation_score: 0.0,
        }
    }

    pub fn add_issue(&mut self, issue: TestQualityIssue) {
        self.issues.push(issue);
    }

    pub fn add_suggestion(&mut self, suggestion: impl Into<String>) {
        self.suggestions.push(suggestion.into());
    }
}

/// Test quality issue (stub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestQualityIssue {
    pub issue_type: TestQualityIssueType,
    pub severity: IssueSeverity,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u64>,
    pub test_name: Option<String>,
    pub description: Option<String>,
}

impl TestQualityIssue {
    pub fn new(issue_type: TestQualityIssueType, severity: IssueSeverity, message: impl Into<String>) -> Self {
        Self {
            issue_type,
            severity,
            message: message.into(),
            file: None,
            line: None,
            test_name: None,
            description: None,
        }
    }

    pub fn with_test_name(mut self, name: impl Into<String>) -> Self {
        self.test_name = Some(name.into());
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Test quality issue type (stub)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestQualityIssueType {
    Flaky,
    SlowTest,
    NoAssertions,
    Duplicate,
    HardcodedData,
    WeakAssertion,
}

/// Issue severity (stub)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueSeverity {
    Info,
    Low,
    Medium,
    Warning,
    High,
    Error,
    Critical,
}

/// Slack service (stub)
#[derive(Debug, Clone)]
pub struct SlackService {
    // Placeholder
}

impl SlackService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn get_active_connection(&self) -> Result<crate::slack::SlackConnection, crate::Error> {
        Err(crate::Error::Other("Slack not configured".to_string()))
    }

    pub async fn get_channel_config(&self, _id: &str) -> Result<crate::slack::ChannelConfig, crate::Error> {
        Err(crate::Error::Other("Slack not configured".to_string()))
    }

    pub async fn save_connection(&self, _conn: &crate::slack::SlackConnection) -> Result<(), crate::Error> {
        Err(crate::Error::Other("Slack not configured".to_string()))
    }

    pub async fn update_channel_config(&self, _config: &crate::slack::ChannelConfig) -> Result<(), crate::Error> {
        Err(crate::Error::Other("Slack not configured".to_string()))
    }
}

impl Default for SlackService {
    fn default() -> Self {
        Self::new()
    }
}

/// Slack user service (stub)
#[derive(Debug, Clone)]
pub struct SlackUserService {
    // Placeholder
}

impl SlackUserService {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn list_user_mappings(&self) -> Result<Vec<crate::slack::UserMapping>, crate::Error> {
        Ok(Vec::new())
    }
}

impl Default for SlackUserService {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export TestType for easy access
pub use crate::test_generation::TestType;

impl TestType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TestType::Unit => "unit",
            TestType::Integration => "integration",
            TestType::E2e => "e2e",
            TestType::Property => "property",
        }
    }
}

impl FromStr for TestType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unit" => Ok(TestType::Unit),
            "integration" => Ok(TestType::Integration),
            "e2e" | "end-to-end" => Ok(TestType::E2e),
            "property" => Ok(TestType::Property),
            _ => Err(format!("Unknown test type: {}", s)),
        }
    }
}
