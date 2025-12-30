//! Test Generation Module
//!
//! Types and utilities for automated test generation, coverage tracking,
//! and test quality validation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Test type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestType {
    Unit,
    Integration,
    EndToEnd,
    Property,
}

impl TestType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Unit => "unit",
            Self::Integration => "integration",
            Self::EndToEnd => "e2e",
            Self::Property => "property",
        }
    }
}

impl std::str::FromStr for TestType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "unit" => Ok(Self::Unit),
            "integration" | "int" => Ok(Self::Integration),
            "e2e" | "end-to-end" | "endtoend" => Ok(Self::EndToEnd),
            "property" | "prop" => Ok(Self::Property),
            _ => Err(format!("Unknown test type: {}", s)),
        }
    }
}

/// Test framework
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestFramework {
    CargoTest,    // Rust
    Jest,         // JavaScript/TypeScript
    Vitest,       // JavaScript/TypeScript
    Pytest,       // Python
    Playwright,   // E2E
    Cypress,      // E2E
}

impl TestFramework {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::CargoTest => "cargo_test",
            Self::Jest => "jest",
            Self::Vitest => "vitest",
            Self::Pytest => "pytest",
            Self::Playwright => "playwright",
            Self::Cypress => "cypress",
        }
    }

    /// Detect framework from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::CargoTest),
            "ts" | "tsx" | "js" | "jsx" => Some(Self::Vitest),
            "py" => Some(Self::Pytest),
            _ => None,
        }
    }
}

/// Generated test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedTest {
    pub name: String,
    pub test_type: TestType,
    pub framework: TestFramework,
    pub target_file: String,
    pub target_function: Option<String>,
    pub test_code: String,
    pub description: String,
    pub generated_at: DateTime<Utc>,
}

impl GeneratedTest {
    /// Create a new generated test
    pub fn new(
        name: &str,
        test_type: TestType,
        framework: TestFramework,
        target_file: &str,
    ) -> Self {
        Self {
            name: name.to_string(),
            test_type,
            framework,
            target_file: target_file.to_string(),
            target_function: None,
            test_code: String::new(),
            description: String::new(),
            generated_at: Utc::now(),
        }
    }

    /// Set the test code
    pub fn with_code(mut self, code: &str) -> Self {
        self.test_code = code.to_string();
        self
    }

    /// Set target function
    pub fn with_function(mut self, function: &str) -> Self {
        self.target_function = Some(function.to_string());
        self
    }
}

/// Test generation request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestGenerationRequest {
    pub target_path: String,
    pub test_type: TestType,
    pub framework: Option<TestFramework>,
    pub story_id: Option<String>,
    pub include_edge_cases: bool,
    pub include_error_cases: bool,
    pub mock_dependencies: bool,
}

impl Default for TestGenerationRequest {
    fn default() -> Self {
        Self {
            target_path: String::new(),
            test_type: TestType::Unit,
            framework: None,
            story_id: None,
            include_edge_cases: true,
            include_error_cases: true,
            mock_dependencies: true,
        }
    }
}

/// Test coverage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub project_name: String,
    pub overall_percentage: f64,
    pub target_percentage: f64,
    pub modules: Vec<ModuleCoverage>,
    pub generated_at: DateTime<Utc>,
    pub trends: Vec<CoverageTrend>,
}

impl CoverageReport {
    /// Create a new coverage report
    pub fn new(project_name: &str) -> Self {
        Self {
            project_name: project_name.to_string(),
            overall_percentage: 0.0,
            target_percentage: 80.0,
            modules: vec![],
            generated_at: Utc::now(),
            trends: vec![],
        }
    }

    /// Add a module
    pub fn add_module(&mut self, module: ModuleCoverage) {
        self.modules.push(module);
        self.recalculate_overall();
    }

    /// Recalculate overall coverage
    fn recalculate_overall(&mut self) {
        if self.modules.is_empty() {
            self.overall_percentage = 0.0;
            return;
        }

        let total_lines: u32 = self.modules.iter().map(|m| m.total_lines).sum();
        let covered_lines: u32 = self.modules.iter().map(|m| m.covered_lines).sum();

        self.overall_percentage = if total_lines > 0 {
            (covered_lines as f64 / total_lines as f64) * 100.0
        } else {
            0.0
        };
    }

    /// Check if coverage meets target
    pub fn meets_target(&self) -> bool {
        self.overall_percentage >= self.target_percentage
    }

    /// Get modules below target
    pub fn modules_below_target(&self) -> Vec<&ModuleCoverage> {
        self.modules
            .iter()
            .filter(|m| m.coverage_percentage < m.target_percentage)
            .collect()
    }

    /// Generate summary
    pub fn to_summary(&self) -> String {
        let mut output = format!("Coverage Report: {}\n", self.project_name);
        output.push_str(&format!("Generated: {}\n\n", self.generated_at.format("%Y-%m-%d %H:%M:%S UTC")));

        let status = if self.meets_target() { "✓" } else { "⚠️" };
        output.push_str(&format!(
            "Overall: {:.1}% (target: {:.0}%) {}\n\n",
            self.overall_percentage, self.target_percentage, status
        ));

        output.push_str("Modules:\n");
        for module in &self.modules {
            let status = if module.coverage_percentage >= module.target_percentage {
                ""
            } else {
                " ⚠️"
            };
            output.push_str(&format!(
                "  {}: {:.1}% (target: {:.0}%){}\n",
                module.name, module.coverage_percentage, module.target_percentage, status
            ));
        }

        output
    }
}

/// Module coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleCoverage {
    pub name: String,
    pub path: String,
    pub total_lines: u32,
    pub covered_lines: u32,
    pub coverage_percentage: f64,
    pub target_percentage: f64,
    pub files: Vec<FileCoverage>,
}

impl ModuleCoverage {
    /// Create new module coverage
    pub fn new(name: &str, path: &str) -> Self {
        Self {
            name: name.to_string(),
            path: path.to_string(),
            total_lines: 0,
            covered_lines: 0,
            coverage_percentage: 0.0,
            target_percentage: 80.0,
            files: vec![],
        }
    }

    /// Add file coverage
    pub fn add_file(&mut self, file: FileCoverage) {
        self.total_lines += file.total_lines;
        self.covered_lines += file.covered_lines;
        self.files.push(file);
        self.recalculate();
    }

    fn recalculate(&mut self) {
        self.coverage_percentage = if self.total_lines > 0 {
            (self.covered_lines as f64 / self.total_lines as f64) * 100.0
        } else {
            0.0
        };
    }
}

/// File coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub path: String,
    pub total_lines: u32,
    pub covered_lines: u32,
    pub coverage_percentage: f64,
    pub uncovered_lines: Vec<u32>,
}

impl FileCoverage {
    /// Create new file coverage
    pub fn new(path: &str, total: u32, covered: u32) -> Self {
        let percentage = if total > 0 {
            (covered as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            path: path.to_string(),
            total_lines: total,
            covered_lines: covered,
            coverage_percentage: percentage,
            uncovered_lines: vec![],
        }
    }
}

/// Coverage trend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageTrend {
    pub date: DateTime<Utc>,
    pub overall_percentage: f64,
    pub commit_sha: Option<String>,
}

/// Test run result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRun {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: TestRunStatus,
    pub total_tests: u32,
    pub passed: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_seconds: Option<f64>,
    pub test_results: Vec<TestResult>,
    pub coverage: Option<CoverageReport>,
}

impl TestRun {
    /// Create a new test run
    pub fn new(id: &str) -> Self {
        Self {
            id: id.to_string(),
            started_at: Utc::now(),
            completed_at: None,
            status: TestRunStatus::Running,
            total_tests: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            duration_seconds: None,
            test_results: vec![],
            coverage: None,
        }
    }

    /// Add a test result
    pub fn add_result(&mut self, result: TestResult) {
        self.total_tests += 1;
        match result.status {
            TestResultStatus::Passed => self.passed += 1,
            TestResultStatus::Failed => self.failed += 1,
            TestResultStatus::Skipped => self.skipped += 1,
        }
        self.test_results.push(result);
    }

    /// Complete the test run
    pub fn complete(&mut self, status: TestRunStatus) {
        self.completed_at = Some(Utc::now());
        self.status = status;
        if let Some(completed) = self.completed_at {
            self.duration_seconds = Some((completed - self.started_at).num_milliseconds() as f64 / 1000.0);
        }
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.status == TestRunStatus::Completed
    }
}

/// Test run status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestRunStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl TestRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub name: String,
    pub status: TestResultStatus,
    pub duration_ms: Option<u64>,
    pub error_message: Option<String>,
    pub stack_trace: Option<String>,
}

/// Test result status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestResultStatus {
    Passed,
    Failed,
    Skipped,
}

impl TestResultStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::Skipped => "skipped",
        }
    }
}

/// Test quality validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestQualityReport {
    pub total_tests: u32,
    pub validated_at: DateTime<Utc>,
    pub issues: Vec<TestQualityIssue>,
    pub mutation_score: Option<f64>,
    pub suggestions: Vec<String>,
}

impl TestQualityReport {
    /// Create a new quality report
    pub fn new() -> Self {
        Self {
            total_tests: 0,
            validated_at: Utc::now(),
            issues: vec![],
            mutation_score: None,
            suggestions: vec![],
        }
    }

    /// Add an issue
    pub fn add_issue(&mut self, issue: TestQualityIssue) {
        self.issues.push(issue);
    }

    /// Add suggestion
    pub fn add_suggestion(&mut self, suggestion: &str) {
        self.suggestions.push(suggestion.to_string());
    }

    /// Check if tests are high quality
    pub fn is_high_quality(&self) -> bool {
        self.issues.is_empty() && self.mutation_score.unwrap_or(0.0) >= 70.0
    }
}

impl Default for TestQualityReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Test quality issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestQualityIssue {
    pub test_name: String,
    pub issue_type: TestQualityIssueType,
    pub description: String,
    pub severity: IssueSeverity,
}

/// Test quality issue type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestQualityIssueType {
    WeakAssertion,
    AlwaysPasses,
    TestsImplementation,
    MissingEdgeCases,
    NoAssertions,
    DuplicateTest,
}

impl TestQualityIssueType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WeakAssertion => "weak_assertion",
            Self::AlwaysPasses => "always_passes",
            Self::TestsImplementation => "tests_implementation",
            Self::MissingEdgeCases => "missing_edge_cases",
            Self::NoAssertions => "no_assertions",
            Self::DuplicateTest => "duplicate_test",
        }
    }
}

/// Issue severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    High,
    Medium,
    Low,
}

impl IssueSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
        }
    }
}

/// Test suggestion for PR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuggestion {
    pub file_path: String,
    pub function_name: String,
    pub suggested_tests: Vec<String>,
    pub reason: String,
    pub priority: IssueSeverity,
}

/// Testable unit identified in code
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestableUnit {
    pub name: String,
    pub unit_type: TestableUnitType,
    pub file_path: String,
    pub line_number: u32,
    pub parameters: Vec<String>,
    pub return_type: Option<String>,
    pub has_tests: bool,
    pub complexity: u32,
}

/// Type of testable unit
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestableUnitType {
    Function,
    Method,
    Class,
    Module,
    Trait,
}

impl TestableUnitType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Class => "class",
            Self::Module => "module",
            Self::Trait => "trait",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_type_from_str() {
        use std::str::FromStr;

        assert_eq!(TestType::from_str("unit").unwrap(), TestType::Unit);
        assert_eq!(TestType::from_str("integration").unwrap(), TestType::Integration);
        assert_eq!(TestType::from_str("e2e").unwrap(), TestType::EndToEnd);
        assert_eq!(TestType::from_str("property").unwrap(), TestType::Property);
        assert!(TestType::from_str("unknown").is_err());
    }

    #[test]
    fn test_framework_detection() {
        assert_eq!(TestFramework::from_extension("rs"), Some(TestFramework::CargoTest));
        assert_eq!(TestFramework::from_extension("ts"), Some(TestFramework::Vitest));
        assert_eq!(TestFramework::from_extension("py"), Some(TestFramework::Pytest));
        assert_eq!(TestFramework::from_extension("go"), None);
    }

    #[test]
    fn test_generated_test_creation() {
        let test = GeneratedTest::new(
            "test_add_numbers",
            TestType::Unit,
            TestFramework::CargoTest,
            "src/math.rs",
        )
        .with_function("add_numbers")
        .with_code("#[test]\nfn test_add_numbers() { assert_eq!(add(2, 3), 5); }");

        assert_eq!(test.name, "test_add_numbers");
        assert_eq!(test.test_type, TestType::Unit);
        assert_eq!(test.target_function, Some("add_numbers".to_string()));
        assert!(test.test_code.contains("assert_eq!"));
    }

    #[test]
    fn test_coverage_report() {
        let mut report = CoverageReport::new("my-project");
        report.target_percentage = 80.0;

        let mut module = ModuleCoverage::new("core", "src/core");
        module.add_file(FileCoverage::new("src/core/lib.rs", 100, 75));
        module.add_file(FileCoverage::new("src/core/utils.rs", 50, 40));
        report.add_module(module);

        assert_eq!(report.modules.len(), 1);
        assert!((report.overall_percentage - 76.67).abs() < 0.1);
        assert!(!report.meets_target());

        let summary = report.to_summary();
        assert!(summary.contains("76."));  // Could be 76.6% or 76.7%
        assert!(summary.contains("my-project"));
    }

    #[test]
    fn test_test_run() {
        let mut run = TestRun::new("run-001");

        run.add_result(TestResult {
            name: "test_pass".to_string(),
            status: TestResultStatus::Passed,
            duration_ms: Some(10),
            error_message: None,
            stack_trace: None,
        });

        run.add_result(TestResult {
            name: "test_fail".to_string(),
            status: TestResultStatus::Failed,
            duration_ms: Some(20),
            error_message: Some("assertion failed".to_string()),
            stack_trace: None,
        });

        assert_eq!(run.total_tests, 2);
        assert_eq!(run.passed, 1);
        assert_eq!(run.failed, 1);
        assert!(!run.all_passed());

        run.complete(TestRunStatus::Failed);
        assert_eq!(run.status, TestRunStatus::Failed);
        assert!(run.completed_at.is_some());
    }

    #[test]
    fn test_quality_report() {
        let mut report = TestQualityReport::new();
        report.total_tests = 10;
        report.mutation_score = Some(75.0);

        report.add_issue(TestQualityIssue {
            test_name: "test_weak".to_string(),
            issue_type: TestQualityIssueType::WeakAssertion,
            description: "Uses assertTrue instead of assertEquals".to_string(),
            severity: IssueSeverity::Medium,
        });

        assert!(!report.is_high_quality());
        assert_eq!(report.issues.len(), 1);
    }

    #[test]
    fn test_module_coverage() {
        let mut module = ModuleCoverage::new("api", "src/api");

        module.add_file(FileCoverage::new("src/api/routes.rs", 200, 180));
        module.add_file(FileCoverage::new("src/api/handlers.rs", 100, 50));

        assert_eq!(module.total_lines, 300);
        assert_eq!(module.covered_lines, 230);
        assert!((module.coverage_percentage - 76.67).abs() < 0.1);
    }
}
