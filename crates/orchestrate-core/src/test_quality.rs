//! Test quality validation and mutation testing
//!
//! This module provides functionality to:
//! - Run mutation testing on generated tests
//! - Identify tests with weak assertions
//! - Detect tests that always pass
//! - Detect tests that test implementation not behavior
//! - Suggest test improvements

use crate::{Database, Error, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::path::Path;

/// Issue found in a test
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestQualityIssue {
    /// Test file path
    pub file_path: String,
    /// Test name
    pub test_name: String,
    /// Issue type
    pub issue_type: TestIssueType,
    /// Description of the issue
    pub description: String,
    /// Suggested improvement
    pub suggestion: Option<String>,
    /// Line number where the issue occurs
    pub line_number: Option<usize>,
}

/// Type of test quality issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestIssueType {
    /// Test has weak or no assertions
    WeakAssertion,
    /// Test always passes regardless of implementation
    AlwaysPasses,
    /// Test checks implementation details instead of behavior
    ImplementationFocused,
    /// Mutation testing detected the test doesn't catch mutations
    MutationSurvived,
    /// Test lacks proper setup or teardown
    ImproperSetup,
}

/// Result of mutation testing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutationTestResult {
    /// Total number of mutations attempted
    pub total_mutations: u32,
    /// Number of mutations that were caught (killed)
    pub mutations_caught: u32,
    /// Number of mutations that survived (not caught)
    pub mutations_survived: u32,
    /// Mutation score (percentage of mutations caught)
    pub mutation_score: f64,
    /// Details of mutations that survived
    pub survived_mutations: Vec<MutationDetail>,
}

impl MutationTestResult {
    /// Create new mutation test result
    pub fn new(caught: u32, survived: u32) -> Self {
        let total = caught + survived;
        let mutation_score = if total > 0 {
            (caught as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        Self {
            total_mutations: total,
            mutations_caught: caught,
            mutations_survived: survived,
            mutation_score,
            survived_mutations: Vec::new(),
        }
    }

    /// Add a survived mutation
    pub fn add_survived_mutation(&mut self, mutation: MutationDetail) {
        self.survived_mutations.push(mutation);
    }
}

/// Details of a specific mutation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutationDetail {
    /// File where mutation occurred
    pub file_path: String,
    /// Line number of mutation
    pub line_number: usize,
    /// Type of mutation applied
    pub mutation_type: MutationType,
    /// Original code
    pub original: String,
    /// Mutated code
    pub mutated: String,
}

/// Type of mutation applied during mutation testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutationType {
    /// Replace arithmetic operator (+ to -, * to /, etc.)
    ArithmeticOperator,
    /// Replace comparison operator (== to !=, < to >, etc.)
    ComparisonOperator,
    /// Replace logical operator (&& to ||, etc.)
    LogicalOperator,
    /// Replace return value
    ReturnValue,
    /// Remove statement
    StatementDeletion,
    /// Replace constant value
    ConstantReplacement,
}

/// Validation result for test quality
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestQualityReport {
    /// Test file analyzed
    pub file_path: String,
    /// Issues found
    pub issues: Vec<TestQualityIssue>,
    /// Mutation testing result (if performed)
    pub mutation_result: Option<MutationTestResult>,
    /// Overall quality score (0-100)
    pub quality_score: f64,
}

impl TestQualityReport {
    /// Create new test quality report
    pub fn new(file_path: String) -> Self {
        Self {
            file_path,
            issues: Vec::new(),
            mutation_result: None,
            quality_score: 100.0,
        }
    }

    /// Add an issue to the report
    pub fn add_issue(&mut self, issue: TestQualityIssue) {
        self.issues.push(issue);
        self.recalculate_quality_score();
    }

    /// Set mutation testing result
    pub fn set_mutation_result(&mut self, result: MutationTestResult) {
        self.mutation_result = Some(result);
        self.recalculate_quality_score();
    }

    /// Recalculate overall quality score
    fn recalculate_quality_score(&mut self) {
        let mut score = 100.0;

        // Deduct points for issues
        let issue_penalty = match self.issues.len() {
            0 => 0.0,
            1..=2 => 10.0,
            3..=5 => 20.0,
            _ => 30.0,
        };
        score -= issue_penalty;

        // Factor in mutation score if available
        if let Some(ref mutation_result) = self.mutation_result {
            // Weight mutation score at 50% of total
            score = (score * 0.5) + (mutation_result.mutation_score * 0.5);
        }

        self.quality_score = score.max(0.0);
    }
}

/// Service for validating test quality
pub struct TestQualityService {
    db: Database,
}

impl TestQualityService {
    /// Create new test quality service
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Validate a test file and identify quality issues
    pub async fn validate_test_file(&self, test_file: &Path) -> Result<TestQualityReport> {
        let file_path = test_file.to_string_lossy().to_string();
        let mut report = TestQualityReport::new(file_path.clone());

        // Read test file content
        let content = tokio::fs::read_to_string(test_file)
            .await
            .map_err(|e| Error::Other(format!("Failed to read test file: {}", e)))?;

        // Run all quality checks
        let mut weak_assertions = self.detect_weak_assertions(&content).await;
        let mut always_passing = self.detect_always_passing_tests(&content).await;
        let mut implementation_focused = self.detect_implementation_tests(&content).await;

        // Set file_path for all issues
        for issue in &mut weak_assertions {
            issue.file_path = file_path.clone();
        }
        for issue in &mut always_passing {
            issue.file_path = file_path.clone();
        }
        for issue in &mut implementation_focused {
            issue.file_path = file_path.clone();
        }

        // Add all issues to report
        for issue in weak_assertions {
            report.add_issue(issue);
        }
        for issue in always_passing {
            report.add_issue(issue);
        }
        for issue in implementation_focused {
            report.add_issue(issue);
        }

        Ok(report)
    }

    /// Run mutation testing on a test file
    pub async fn run_mutation_testing(
        &self,
        _test_file: &Path,
        _source_file: &Path,
    ) -> Result<MutationTestResult> {
        // To be implemented
        Err(Error::Other("Not implemented".to_string()))
    }

    /// Detect weak assertions in test code
    pub async fn detect_weak_assertions(&self, test_content: &str) -> Vec<TestQualityIssue> {
        let mut issues = Vec::new();

        // Pattern to find test functions in Rust
        let test_fn_regex = Regex::new(r"#\[test\]\s*(?:async\s+)?fn\s+(\w+)\s*\([^)]*\)\s*\{([^}]*(?:\{[^}]*\}[^}]*)*)\}").unwrap();

        for cap in test_fn_regex.captures_iter(test_content) {
            let test_name = cap.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            let test_body = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Check for assertions
            let has_assertion = test_body.contains("assert!")
                || test_body.contains("assert_eq!")
                || test_body.contains("assert_ne!")
                || test_body.contains("expect(")
                || test_body.contains(".toBe(")
                || test_body.contains(".toEqual(")
                || test_body.contains("self.assert");

            if !has_assertion {
                // Find line number (approximate)
                let line_number = test_content[..cap.get(0).unwrap().start()]
                    .lines()
                    .count();

                issues.push(TestQualityIssue {
                    file_path: String::new(), // Will be set by caller
                    test_name: test_name.to_string(),
                    issue_type: TestIssueType::WeakAssertion,
                    description: format!("Test '{}' has no assertions", test_name),
                    suggestion: Some("Add assertions to verify expected behavior (assert_eq!, assert!, expect())".to_string()),
                    line_number: Some(line_number),
                });
            }
        }

        issues
    }

    /// Detect tests that always pass
    pub async fn detect_always_passing_tests(&self, test_content: &str) -> Vec<TestQualityIssue> {
        let mut issues = Vec::new();

        // Pattern to find test functions
        let test_fn_regex = Regex::new(r"#\[test\]\s*(?:async\s+)?fn\s+(\w+)\s*\([^)]*\)\s*\{([^}]*(?:\{[^}]*\}[^}]*)*)\}").unwrap();

        for cap in test_fn_regex.captures_iter(test_content) {
            let test_name = cap.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            let test_body = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Check for tautological assertions
            let always_passes = test_body.contains("assert!(true)")
                || test_body.contains("assert_eq!(true, true)")
                || test_body.contains("assert_eq!(1, 1)")
                || test_body.contains("expect(true).toBe(true)")
                || test_body.contains("assertTrue(true)");

            if always_passes {
                let line_number = test_content[..cap.get(0).unwrap().start()]
                    .lines()
                    .count();

                issues.push(TestQualityIssue {
                    file_path: String::new(),
                    test_name: test_name.to_string(),
                    issue_type: TestIssueType::AlwaysPasses,
                    description: format!("Test '{}' has tautological assertion that always passes", test_name),
                    suggestion: Some("Replace with meaningful assertion that verifies actual behavior".to_string()),
                    line_number: Some(line_number),
                });
            }
        }

        issues
    }

    /// Detect tests that focus on implementation instead of behavior
    pub async fn detect_implementation_tests(&self, test_content: &str) -> Vec<TestQualityIssue> {
        let mut issues = Vec::new();

        // Pattern to find test functions
        let test_fn_regex = Regex::new(r"#\[test\]\s*(?:async\s+)?fn\s+(\w+)\s*\([^)]*\)\s*\{([^}]*(?:\{[^}]*\}[^}]*)*)\}").unwrap();

        for cap in test_fn_regex.captures_iter(test_content) {
            let test_name = cap.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            let test_body = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            // Check for implementation-focused patterns
            let implementation_patterns = [
                ("internal_", "Accessing internal field"),
                ("private_", "Accessing private field"),
                ("._", "Accessing private/internal member"),
                (".counter", "Testing internal state counter"),
                (".cache", "Testing internal cache state"),
                (".lock", "Testing internal lock state"),
            ];

            for (pattern, reason) in &implementation_patterns {
                if test_body.contains(pattern) {
                    let line_number = test_content[..cap.get(0).unwrap().start()]
                        .lines()
                        .count();

                    issues.push(TestQualityIssue {
                        file_path: String::new(),
                        test_name: test_name.to_string(),
                        issue_type: TestIssueType::ImplementationFocused,
                        description: format!("Test '{}' may be testing implementation details: {}", test_name, reason),
                        suggestion: Some("Focus on testing public behavior and observable outcomes instead of internal state".to_string()),
                        line_number: Some(line_number),
                    });
                    break; // Only report once per test
                }
            }
        }

        issues
    }

    /// Store test quality report in database
    pub async fn store_quality_report(&self, report: &TestQualityReport) -> Result<i64> {
        let mut tx = self.db.begin().await?;

        // Insert quality report
        let report_id = sqlx::query(
            r#"
            INSERT INTO test_quality_reports (file_path, quality_score)
            VALUES (?, ?)
            "#,
        )
        .bind(&report.file_path)
        .bind(report.quality_score)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        // Insert issues
        for issue in &report.issues {
            let issue_type_str = match issue.issue_type {
                TestIssueType::WeakAssertion => "weak_assertion",
                TestIssueType::AlwaysPasses => "always_passes",
                TestIssueType::ImplementationFocused => "implementation_focused",
                TestIssueType::MutationSurvived => "mutation_survived",
                TestIssueType::ImproperSetup => "improper_setup",
            };

            sqlx::query(
                r#"
                INSERT INTO test_quality_issues (report_id, file_path, test_name, issue_type, description, suggestion, line_number)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(report_id)
            .bind(&issue.file_path)
            .bind(&issue.test_name)
            .bind(issue_type_str)
            .bind(&issue.description)
            .bind(&issue.suggestion)
            .bind(issue.line_number.map(|n| n as i64))
            .execute(&mut *tx)
            .await?;
        }

        // Insert mutation result if present
        if let Some(ref mutation_result) = report.mutation_result {
            let mutation_result_id = sqlx::query(
                r#"
                INSERT INTO mutation_test_results (report_id, total_mutations, mutations_caught, mutations_survived, mutation_score)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(report_id)
            .bind(mutation_result.total_mutations)
            .bind(mutation_result.mutations_caught)
            .bind(mutation_result.mutations_survived)
            .bind(mutation_result.mutation_score)
            .execute(&mut *tx)
            .await?
            .last_insert_rowid();

            // Insert mutation details
            for mutation in &mutation_result.survived_mutations {
                let mutation_type_str = match mutation.mutation_type {
                    MutationType::ArithmeticOperator => "arithmetic_operator",
                    MutationType::ComparisonOperator => "comparison_operator",
                    MutationType::LogicalOperator => "logical_operator",
                    MutationType::ReturnValue => "return_value",
                    MutationType::StatementDeletion => "statement_deletion",
                    MutationType::ConstantReplacement => "constant_replacement",
                };

                sqlx::query(
                    r#"
                    INSERT INTO mutation_details (mutation_result_id, file_path, line_number, mutation_type, original, mutated)
                    VALUES (?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(mutation_result_id)
                .bind(&mutation.file_path)
                .bind(mutation.line_number as i64)
                .bind(mutation_type_str)
                .bind(&mutation.original)
                .bind(&mutation.mutated)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(report_id)
    }

    /// Get quality reports for a file
    pub async fn get_quality_reports(&self, file_path: &str) -> Result<Vec<TestQualityReport>> {
        let rows = sqlx::query(
            r#"
            SELECT id, file_path, quality_score, created_at
            FROM test_quality_reports
            WHERE file_path = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(file_path)
        .fetch_all(self.db.pool())
        .await?;

        let mut reports = Vec::new();

        for row in rows {
            let report_id: i64 = row.get(0);
            let file_path: String = row.get(1);
            let quality_score: f64 = row.get(2);

            let mut report = TestQualityReport {
                file_path,
                issues: Vec::new(),
                mutation_result: None,
                quality_score,
            };

            // Get issues for this report
            let issue_rows = sqlx::query(
                r#"
                SELECT file_path, test_name, issue_type, description, suggestion, line_number
                FROM test_quality_issues
                WHERE report_id = ?
                "#,
            )
            .bind(report_id)
            .fetch_all(self.db.pool())
            .await?;

            for issue_row in issue_rows {
                let file_path: String = issue_row.get(0);
                let test_name: String = issue_row.get(1);
                let issue_type_str: String = issue_row.get(2);
                let description: String = issue_row.get(3);
                let suggestion: Option<String> = issue_row.get(4);
                let line_number: Option<i64> = issue_row.get(5);

                let issue_type = match issue_type_str.as_str() {
                    "weak_assertion" => TestIssueType::WeakAssertion,
                    "always_passes" => TestIssueType::AlwaysPasses,
                    "implementation_focused" => TestIssueType::ImplementationFocused,
                    "mutation_survived" => TestIssueType::MutationSurvived,
                    "improper_setup" => TestIssueType::ImproperSetup,
                    _ => TestIssueType::WeakAssertion,
                };

                report.issues.push(TestQualityIssue {
                    file_path,
                    test_name,
                    issue_type,
                    description,
                    suggestion,
                    line_number: line_number.map(|n| n as usize),
                });
            }

            // Get mutation result if present
            let mutation_row = sqlx::query(
                r#"
                SELECT id, total_mutations, mutations_caught, mutations_survived, mutation_score
                FROM mutation_test_results
                WHERE report_id = ?
                LIMIT 1
                "#,
            )
            .bind(report_id)
            .fetch_optional(self.db.pool())
            .await?;

            if let Some(mutation_row) = mutation_row {
                let mutation_result_id: i64 = mutation_row.get(0);
                let total_mutations: u32 = mutation_row.get::<i64, _>(1) as u32;
                let mutations_caught: u32 = mutation_row.get::<i64, _>(2) as u32;
                let mutations_survived: u32 = mutation_row.get::<i64, _>(3) as u32;
                let mutation_score: f64 = mutation_row.get(4);

                let mut mutation_result = MutationTestResult {
                    total_mutations,
                    mutations_caught,
                    mutations_survived,
                    mutation_score,
                    survived_mutations: Vec::new(),
                };

                // Get mutation details
                let detail_rows = sqlx::query(
                    r#"
                    SELECT file_path, line_number, mutation_type, original, mutated
                    FROM mutation_details
                    WHERE mutation_result_id = ?
                    "#,
                )
                .bind(mutation_result_id)
                .fetch_all(self.db.pool())
                .await?;

                for detail_row in detail_rows {
                    let file_path: String = detail_row.get(0);
                    let line_number: i64 = detail_row.get(1);
                    let mutation_type_str: String = detail_row.get(2);
                    let original: String = detail_row.get(3);
                    let mutated: String = detail_row.get(4);

                    let mutation_type = match mutation_type_str.as_str() {
                        "arithmetic_operator" => MutationType::ArithmeticOperator,
                        "comparison_operator" => MutationType::ComparisonOperator,
                        "logical_operator" => MutationType::LogicalOperator,
                        "return_value" => MutationType::ReturnValue,
                        "statement_deletion" => MutationType::StatementDeletion,
                        "constant_replacement" => MutationType::ConstantReplacement,
                        _ => MutationType::ArithmeticOperator,
                    };

                    mutation_result.survived_mutations.push(MutationDetail {
                        file_path,
                        line_number: line_number as usize,
                        mutation_type,
                        original,
                        mutated,
                    });
                }

                report.mutation_result = Some(mutation_result);
            }

            reports.push(report);
        }

        Ok(reports)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mutation_result_new() {
        let result = MutationTestResult::new(8, 2);
        assert_eq!(result.total_mutations, 10);
        assert_eq!(result.mutations_caught, 8);
        assert_eq!(result.mutations_survived, 2);
        assert_eq!(result.mutation_score, 80.0);
    }

    #[test]
    fn test_mutation_result_zero_mutations() {
        let result = MutationTestResult::new(0, 0);
        assert_eq!(result.total_mutations, 0);
        assert_eq!(result.mutation_score, 0.0);
    }

    #[test]
    fn test_mutation_result_add_survived() {
        let mut result = MutationTestResult::new(8, 2);
        assert_eq!(result.survived_mutations.len(), 0);

        let mutation = MutationDetail {
            file_path: "src/lib.rs".to_string(),
            line_number: 42,
            mutation_type: MutationType::ComparisonOperator,
            original: "x == y".to_string(),
            mutated: "x != y".to_string(),
        };

        result.add_survived_mutation(mutation);
        assert_eq!(result.survived_mutations.len(), 1);
    }

    #[test]
    fn test_quality_report_new() {
        let report = TestQualityReport::new("tests/test_foo.rs".to_string());
        assert_eq!(report.file_path, "tests/test_foo.rs");
        assert_eq!(report.issues.len(), 0);
        assert_eq!(report.quality_score, 100.0);
        assert!(report.mutation_result.is_none());
    }

    #[test]
    fn test_quality_report_add_issue() {
        let mut report = TestQualityReport::new("tests/test_foo.rs".to_string());

        let issue = TestQualityIssue {
            file_path: "tests/test_foo.rs".to_string(),
            test_name: "test_something".to_string(),
            issue_type: TestIssueType::WeakAssertion,
            description: "Test has no assertions".to_string(),
            suggestion: Some("Add assert_eq! or similar".to_string()),
            line_number: Some(10),
        };

        report.add_issue(issue);
        assert_eq!(report.issues.len(), 1);
        assert!(report.quality_score < 100.0);
    }

    #[test]
    fn test_quality_report_scoring_with_issues() {
        let mut report = TestQualityReport::new("tests/test_foo.rs".to_string());

        // No issues = 100
        assert_eq!(report.quality_score, 100.0);

        // 1-2 issues = -10 points
        report.add_issue(TestQualityIssue {
            file_path: "test.rs".to_string(),
            test_name: "test1".to_string(),
            issue_type: TestIssueType::WeakAssertion,
            description: "Weak".to_string(),
            suggestion: None,
            line_number: None,
        });
        assert_eq!(report.quality_score, 90.0);

        // 3-5 issues = -20 points
        report.add_issue(TestQualityIssue {
            file_path: "test.rs".to_string(),
            test_name: "test2".to_string(),
            issue_type: TestIssueType::AlwaysPasses,
            description: "Always passes".to_string(),
            suggestion: None,
            line_number: None,
        });
        report.add_issue(TestQualityIssue {
            file_path: "test.rs".to_string(),
            test_name: "test3".to_string(),
            issue_type: TestIssueType::ImplementationFocused,
            description: "Implementation".to_string(),
            suggestion: None,
            line_number: None,
        });
        assert_eq!(report.quality_score, 80.0);
    }

    #[test]
    fn test_quality_report_scoring_with_mutation() {
        let mut report = TestQualityReport::new("tests/test_foo.rs".to_string());

        // Set mutation result with 80% score
        let mutation_result = MutationTestResult::new(8, 2);
        report.set_mutation_result(mutation_result);

        // Score should be weighted average: (100 * 0.5) + (80 * 0.5) = 90
        assert_eq!(report.quality_score, 90.0);
    }

    #[test]
    fn test_quality_report_combined_scoring() {
        let mut report = TestQualityReport::new("tests/test_foo.rs".to_string());

        // Add 2 issues (penalty -10)
        report.add_issue(TestQualityIssue {
            file_path: "test.rs".to_string(),
            test_name: "test1".to_string(),
            issue_type: TestIssueType::WeakAssertion,
            description: "Weak".to_string(),
            suggestion: None,
            line_number: None,
        });
        report.add_issue(TestQualityIssue {
            file_path: "test.rs".to_string(),
            test_name: "test2".to_string(),
            issue_type: TestIssueType::AlwaysPasses,
            description: "Always passes".to_string(),
            suggestion: None,
            line_number: None,
        });

        // Set mutation result with 60% score
        let mutation_result = MutationTestResult::new(6, 4);
        report.set_mutation_result(mutation_result);

        // Score: ((100 - 10) * 0.5) + (60 * 0.5) = 45 + 30 = 75
        assert_eq!(report.quality_score, 75.0);
    }

    #[tokio::test]
    async fn test_validate_test_file() {
        use tempfile::NamedTempFile;
        use std::io::Write;

        let db = Database::new(":memory:").await.unwrap();
        let service = TestQualityService::new(db);

        // Create a temporary test file with quality issues
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, r#"
            #[test]
            fn test_no_assertions() {{
                let x = 5;
                let y = 10;
            }}

            #[test]
            fn test_always_passes() {{
                assert!(true);
            }}
        "#).unwrap();

        let report = service.validate_test_file(temp_file.path()).await.unwrap();

        // Should find at least 2 issues
        assert!(report.issues.len() >= 2);
        assert!(report.quality_score < 100.0);
    }

    #[tokio::test]
    async fn test_run_mutation_testing_not_implemented() {
        let db = Database::new(":memory:").await.unwrap();
        let service = TestQualityService::new(db);

        let result = service
            .run_mutation_testing(Path::new("test.rs"), Path::new("src.rs"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_detect_weak_assertions() {
        let db = Database::new(":memory:").await.unwrap();
        let service = TestQualityService::new(db);

        let test_content = r#"
            #[test]
            fn test_something() {
                let result = do_something();
                // No assertion!
            }
        "#;

        let issues = service.detect_weak_assertions(test_content).await;
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].test_name, "test_something");
        assert_eq!(issues[0].issue_type, TestIssueType::WeakAssertion);
    }

    #[tokio::test]
    async fn test_detect_always_passing_tests() {
        let db = Database::new(":memory:").await.unwrap();
        let service = TestQualityService::new(db);

        let test_content = r#"
            #[test]
            fn test_always_passes() {
                assert!(true);
            }
        "#;

        let issues = service.detect_always_passing_tests(test_content).await;
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].test_name, "test_always_passes");
        assert_eq!(issues[0].issue_type, TestIssueType::AlwaysPasses);
    }

    #[tokio::test]
    async fn test_detect_implementation_tests() {
        let db = Database::new(":memory:").await.unwrap();
        let service = TestQualityService::new(db);

        let test_content = r#"
            #[test]
            fn test_internal_state() {
                let obj = MyStruct::new();
                // Testing internal implementation detail
                assert_eq!(obj.internal_counter, 0);
            }
        "#;

        let issues = service.detect_implementation_tests(test_content).await;
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].test_name, "test_internal_state");
        assert_eq!(issues[0].issue_type, TestIssueType::ImplementationFocused);
    }

    #[tokio::test]
    async fn test_store_and_retrieve_quality_report() {
        let db = Database::new(":memory:").await.unwrap();
        let service = TestQualityService::new(db);

        // Create a quality report with issues
        let mut report = TestQualityReport::new("tests/test_foo.rs".to_string());

        report.add_issue(TestQualityIssue {
            file_path: "tests/test_foo.rs".to_string(),
            test_name: "test_weak".to_string(),
            issue_type: TestIssueType::WeakAssertion,
            description: "No assertions".to_string(),
            suggestion: Some("Add assertions".to_string()),
            line_number: Some(10),
        });

        report.add_issue(TestQualityIssue {
            file_path: "tests/test_foo.rs".to_string(),
            test_name: "test_always_true".to_string(),
            issue_type: TestIssueType::AlwaysPasses,
            description: "Always passes".to_string(),
            suggestion: Some("Fix assertion".to_string()),
            line_number: Some(20),
        });

        // Add mutation result
        let mut mutation_result = MutationTestResult::new(8, 2);
        mutation_result.add_survived_mutation(MutationDetail {
            file_path: "src/lib.rs".to_string(),
            line_number: 42,
            mutation_type: MutationType::ComparisonOperator,
            original: "x == y".to_string(),
            mutated: "x != y".to_string(),
        });
        report.set_mutation_result(mutation_result);

        // Store the report
        let report_id = service.store_quality_report(&report).await.unwrap();
        assert!(report_id > 0);

        // Retrieve the report
        let reports = service.get_quality_reports("tests/test_foo.rs").await.unwrap();
        assert_eq!(reports.len(), 1);

        let retrieved = &reports[0];
        assert_eq!(retrieved.file_path, report.file_path);
        assert_eq!(retrieved.quality_score, report.quality_score);
        assert_eq!(retrieved.issues.len(), 2);
        assert!(retrieved.mutation_result.is_some());

        let mutation = retrieved.mutation_result.as_ref().unwrap();
        assert_eq!(mutation.total_mutations, 10);
        assert_eq!(mutation.mutations_caught, 8);
        assert_eq!(mutation.mutations_survived, 2);
        assert_eq!(mutation.survived_mutations.len(), 1);
    }

    #[tokio::test]
    async fn test_get_quality_reports_empty() {
        let db = Database::new(":memory:").await.unwrap();
        let service = TestQualityService::new(db);

        let result = service.get_quality_reports("nonexistent.rs").await.unwrap();
        assert_eq!(result.len(), 0);
    }
}
