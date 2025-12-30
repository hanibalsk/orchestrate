//! Change Test Analyzer - Analyze git diffs and suggest tests for changed code
//!
//! This module provides functionality to:
//! - Parse git diffs to identify changed/added functions
//! - Determine if changed functions have corresponding tests
//! - Generate test suggestions for untested code
//! - Format suggestions as PR comments

use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a changed function detected in a git diff
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangedFunction {
    /// Function name
    pub name: String,
    /// File path where the function is located
    pub file_path: PathBuf,
    /// Line number where the function starts
    pub line_number: usize,
    /// Type of change (added, modified, deleted)
    pub change_type: ChangeType,
    /// Function signature
    pub signature: String,
    /// Whether the function is public
    pub is_public: bool,
}

/// Type of change detected in diff
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeType {
    /// Function was newly added
    Added,
    /// Function was modified
    Modified,
    /// Function was deleted
    Deleted,
}

/// Information about test coverage for a function
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestCoverage {
    /// Function being analyzed
    pub function: ChangedFunction,
    /// Whether the function has tests
    pub has_tests: bool,
    /// Test file paths that test this function
    pub test_files: Vec<PathBuf>,
    /// Names of tests that cover this function
    pub test_names: Vec<String>,
}

/// A suggestion to add tests for an untested function
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestSuggestion {
    /// Function that needs testing
    pub function: ChangedFunction,
    /// Suggested test cases
    pub suggested_tests: Vec<String>,
    /// Priority level (high, medium, low)
    pub priority: Priority,
    /// Reason why tests are needed
    pub reason: String,
}

/// Priority level for test suggestions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
    High,
    Medium,
    Low,
}

/// Result of analyzing changes for test coverage
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChangeAnalysisResult {
    /// All functions changed in the diff
    pub changed_functions: Vec<ChangedFunction>,
    /// Test coverage information for each function
    pub coverage: Vec<TestCoverage>,
    /// Suggestions for untested functions
    pub suggestions: Vec<TestSuggestion>,
    /// Overall test coverage percentage for changes
    pub coverage_percentage: f64,
}

/// Service for analyzing code changes and generating test suggestions
pub struct ChangeTestAnalyzer {
    /// Base directory for the repository
    repo_path: PathBuf,
}

impl ChangeTestAnalyzer {
    /// Create a new change test analyzer
    pub fn new(repo_path: PathBuf) -> Self {
        Self { repo_path }
    }

    /// Analyze git diff and generate test suggestions
    ///
    /// # Arguments
    /// * `diff_content` - The git diff output to analyze
    /// * `_base_ref` - The base reference (e.g., "main")
    /// * `_head_ref` - The head reference (e.g., "feature-branch")
    ///
    /// # Returns
    /// A `ChangeAnalysisResult` containing all analysis and suggestions
    pub async fn analyze_diff(
        &self,
        diff_content: &str,
        _base_ref: &str,
        _head_ref: &str,
    ) -> Result<ChangeAnalysisResult> {
        // Parse diff to find changed functions
        let changed_functions = self.parse_diff(diff_content).await?;

        // Check test coverage for each function
        let coverage = self.check_test_coverage(&changed_functions).await?;

        // Generate suggestions for untested functions
        let suggestions = self.generate_suggestions(&coverage).await?;

        // Calculate coverage percentage
        let coverage_percentage = self.calculate_coverage_percentage(&coverage);

        Ok(ChangeAnalysisResult {
            changed_functions,
            coverage,
            suggestions,
            coverage_percentage,
        })
    }

    /// Parse git diff to identify changed functions
    async fn parse_diff(&self, diff_content: &str) -> Result<Vec<ChangedFunction>> {
        let mut changed_functions = Vec::new();
        let mut current_file: Option<PathBuf> = None;
        let mut line_number = 0;

        for line in diff_content.lines() {
            // Track current file
            if line.starts_with("diff --git") {
                current_file = self.extract_file_path(line);
            }

            // Track line numbers in diff
            if line.starts_with("@@") {
                if let Some(num) = self.extract_line_number(line) {
                    line_number = num;
                }
            }

            // Look for function additions/modifications
            if line.starts_with('+') && !line.starts_with("+++") {
                if let Some(function) = self.extract_function_from_line(
                    &line[1..],
                    current_file.as_ref(),
                    line_number,
                ) {
                    changed_functions.push(function);
                }
            }

            if line.starts_with('+') || line.starts_with(' ') {
                line_number += 1;
            }
        }

        Ok(changed_functions)
    }

    /// Extract file path from diff header
    fn extract_file_path(&self, line: &str) -> Option<PathBuf> {
        // Format: diff --git a/path/to/file.rs b/path/to/file.rs
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 4 {
            let path = parts[3].trim_start_matches("b/");
            Some(PathBuf::from(path))
        } else {
            None
        }
    }

    /// Extract line number from diff hunk header
    fn extract_line_number(&self, line: &str) -> Option<usize> {
        // Format: @@ -10,7 +10,8 @@
        if let Some(plus_idx) = line.find('+') {
            let after_plus = &line[plus_idx + 1..];
            if let Some(comma_idx) = after_plus.find(',') {
                after_plus[..comma_idx].parse().ok()
            } else if let Some(space_idx) = after_plus.find(' ') {
                after_plus[..space_idx].parse().ok()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Extract function from a line of code
    fn extract_function_from_line(
        &self,
        line: &str,
        file_path: Option<&PathBuf>,
        line_number: usize,
    ) -> Option<ChangedFunction> {
        let trimmed = line.trim();
        let file_path = file_path?.clone();

        // Detect language and extract function
        if file_path.extension()?.to_str()? == "rs" {
            self.extract_rust_function(trimmed, file_path, line_number)
        } else if matches!(file_path.extension()?.to_str()?, "ts" | "tsx" | "js" | "jsx") {
            self.extract_typescript_function(trimmed, file_path, line_number)
        } else if file_path.extension()?.to_str()? == "py" {
            self.extract_python_function(trimmed, file_path, line_number)
        } else {
            None
        }
    }

    /// Extract Rust function from a line
    fn extract_rust_function(
        &self,
        line: &str,
        file_path: PathBuf,
        line_number: usize,
    ) -> Option<ChangedFunction> {
        let is_public = line.starts_with("pub fn ") || line.starts_with("pub async fn ");
        let is_function = line.contains("fn ") && line.contains('(');

        if !is_function {
            return None;
        }

        // Extract function name
        let fn_idx = line.find("fn ")?;
        let after_fn = &line[fn_idx + 3..];
        let paren_idx = after_fn.find('(')?;
        let name = after_fn[..paren_idx].trim().to_string();

        if name.is_empty() {
            return None;
        }

        Some(ChangedFunction {
            name,
            file_path,
            line_number,
            change_type: ChangeType::Added,
            signature: line.to_string(),
            is_public,
        })
    }

    /// Extract TypeScript/JavaScript function from a line
    fn extract_typescript_function(
        &self,
        line: &str,
        file_path: PathBuf,
        line_number: usize,
    ) -> Option<ChangedFunction> {
        let is_public = line.starts_with("export ");
        let is_function = line.contains("function ") || line.contains("const ") && line.contains("=> ");

        if !is_function {
            return None;
        }

        // Try to extract function name
        let name = if line.contains("function ") {
            let fn_idx = line.find("function ")?;
            let after_fn = &line[fn_idx + 9..];
            let paren_idx = after_fn.find('(')?;
            after_fn[..paren_idx].trim().to_string()
        } else if line.contains("const ") {
            let const_idx = line.find("const ")?;
            let after_const = &line[const_idx + 6..];
            let eq_idx = after_const.find('=')?;
            after_const[..eq_idx].trim().to_string()
        } else {
            return None;
        };

        if name.is_empty() {
            return None;
        }

        Some(ChangedFunction {
            name,
            file_path,
            line_number,
            change_type: ChangeType::Added,
            signature: line.to_string(),
            is_public,
        })
    }

    /// Extract Python function from a line
    fn extract_python_function(
        &self,
        line: &str,
        file_path: PathBuf,
        line_number: usize,
    ) -> Option<ChangedFunction> {
        let is_function = line.starts_with("def ");

        if !is_function {
            return None;
        }

        let def_idx = line.find("def ")?;
        let after_def = &line[def_idx + 4..];
        let paren_idx = after_def.find('(')?;
        let name = after_def[..paren_idx].trim().to_string();

        if name.is_empty() {
            return None;
        }

        // Python: functions not starting with _ are considered public
        let is_public = !name.starts_with('_');

        Some(ChangedFunction {
            name,
            file_path,
            line_number,
            change_type: ChangeType::Added,
            signature: line.to_string(),
            is_public,
        })
    }

    /// Check test coverage for changed functions
    async fn check_test_coverage(
        &self,
        changed_functions: &[ChangedFunction],
    ) -> Result<Vec<TestCoverage>> {
        let mut coverage = Vec::new();

        for function in changed_functions {
            let test_files = self.find_test_files(&function.file_path).await?;
            let test_names = self.find_tests_for_function(&function.name, &test_files).await?;
            let has_tests = !test_names.is_empty();

            coverage.push(TestCoverage {
                function: function.clone(),
                has_tests,
                test_files,
                test_names,
            });
        }

        Ok(coverage)
    }

    /// Find test files for a given source file
    async fn find_test_files(&self, source_file: &Path) -> Result<Vec<PathBuf>> {
        let mut test_files = Vec::new();

        // Determine language-specific test file patterns
        if let Some(ext) = source_file.extension().and_then(|e| e.to_str()) {
            match ext {
                "rs" => {
                    // Rust: check for inline tests and external test files
                    test_files.push(source_file.to_path_buf());

                    // Check tests/ directory
                    if let Some(file_name) = source_file.file_stem().and_then(|s| s.to_str()) {
                        let test_path = self.repo_path.join("tests").join(format!("{}_test.rs", file_name));
                        test_files.push(test_path);
                    }
                }
                "ts" | "tsx" | "js" | "jsx" => {
                    // TypeScript/JavaScript: check for .test.ts, .spec.ts, __tests__/
                    if let Some(file_name) = source_file.file_stem().and_then(|s| s.to_str()) {
                        let dir = source_file.parent().unwrap_or(Path::new(""));
                        test_files.push(dir.join(format!("{}.test.{}", file_name, ext)));
                        test_files.push(dir.join(format!("{}.spec.{}", file_name, ext)));
                        test_files.push(dir.join("__tests__").join(format!("{}.test.{}", file_name, ext)));
                    }
                }
                "py" => {
                    // Python: check for test_*.py and *_test.py
                    if let Some(file_name) = source_file.file_stem().and_then(|s| s.to_str()) {
                        let dir = source_file.parent().unwrap_or(Path::new(""));
                        test_files.push(dir.join(format!("test_{}.py", file_name)));
                        test_files.push(dir.join(format!("{}_test.py", file_name)));
                        test_files.push(self.repo_path.join("tests").join(format!("test_{}.py", file_name)));
                    }
                }
                _ => {}
            }
        }

        Ok(test_files)
    }

    /// Find tests for a specific function
    async fn find_tests_for_function(
        &self,
        function_name: &str,
        test_files: &[PathBuf],
    ) -> Result<Vec<String>> {
        let mut test_names = Vec::new();

        for test_file in test_files {
            if !test_file.exists() {
                continue;
            }

            let content = match tokio::fs::read_to_string(test_file).await {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Look for test functions that reference the function name
            for line in content.lines() {
                let trimmed = line.trim();

                // Rust: #[test] fn test_function_name
                if trimmed.starts_with("fn test_") && trimmed.contains(function_name) {
                    if let Some(name) = self.extract_test_name(trimmed, "fn ") {
                        test_names.push(name);
                    }
                }

                // TypeScript/JavaScript: test('test name', ...)
                if (trimmed.starts_with("test(") || trimmed.starts_with("it("))
                    && trimmed.contains(function_name) {
                    if let Some(name) = self.extract_ts_test_name(trimmed) {
                        test_names.push(name);
                    }
                }

                // Python: def test_function_name
                if trimmed.starts_with("def test_") && trimmed.contains(function_name) {
                    if let Some(name) = self.extract_test_name(trimmed, "def ") {
                        test_names.push(name);
                    }
                }
            }
        }

        Ok(test_names)
    }

    /// Extract test name from a line
    fn extract_test_name(&self, line: &str, prefix: &str) -> Option<String> {
        let fn_idx = line.find(prefix)?;
        let after_fn = &line[fn_idx + prefix.len()..];
        let paren_idx = after_fn.find('(')?;
        Some(after_fn[..paren_idx].trim().to_string())
    }

    /// Extract TypeScript test name
    fn extract_ts_test_name(&self, line: &str) -> Option<String> {
        let start = line.find('(')? + 1;
        let rest = &line[start..];
        let quote_start = rest.find(|c| c == '"' || c == '\'' || c == '`')? + 1;
        let after_quote = &rest[quote_start..];
        let quote_end = after_quote.find(|c| c == '"' || c == '\'' || c == '`')?;
        Some(after_quote[..quote_end].to_string())
    }

    /// Generate test suggestions for untested functions
    async fn generate_suggestions(
        &self,
        coverage: &[TestCoverage],
    ) -> Result<Vec<TestSuggestion>> {
        let mut suggestions = Vec::new();

        for cov in coverage {
            if !cov.has_tests {
                let priority = self.calculate_priority(&cov.function);
                let reason = self.generate_reason(&cov.function);
                let suggested_tests = self.generate_test_cases(&cov.function);

                suggestions.push(TestSuggestion {
                    function: cov.function.clone(),
                    suggested_tests,
                    priority,
                    reason,
                });
            }
        }

        Ok(suggestions)
    }

    /// Calculate priority for a test suggestion
    fn calculate_priority(&self, function: &ChangedFunction) -> Priority {
        if function.is_public {
            Priority::High
        } else if function.name.starts_with("test_") || function.name.contains("helper") {
            Priority::Low
        } else {
            Priority::Medium
        }
    }

    /// Generate reason for needing tests
    fn generate_reason(&self, function: &ChangedFunction) -> String {
        match function.change_type {
            ChangeType::Added => {
                if function.is_public {
                    format!("New public function '{}' added without tests", function.name)
                } else {
                    format!("New function '{}' added without tests", function.name)
                }
            }
            ChangeType::Modified => {
                format!("Function '{}' was modified but no tests were updated", function.name)
            }
            ChangeType::Deleted => {
                format!("Function '{}' was deleted - ensure related tests are removed", function.name)
            }
        }
    }

    /// Generate suggested test cases for a function
    fn generate_test_cases(&self, function: &ChangedFunction) -> Vec<String> {
        let mut tests = Vec::new();

        // Suggest basic test structure based on language
        if let Some(ext) = function.file_path.extension().and_then(|e| e.to_str()) {
            match ext {
                "rs" => {
                    tests.push(format!("test_{}_happy_path", function.name));
                    tests.push(format!("test_{}_edge_cases", function.name));
                    if function.is_public {
                        tests.push(format!("test_{}_error_handling", function.name));
                    }
                }
                "ts" | "tsx" | "js" | "jsx" => {
                    tests.push(format!("should handle valid inputs for {}", function.name));
                    tests.push(format!("should handle edge cases for {}", function.name));
                    if function.is_public {
                        tests.push(format!("should throw error on invalid input for {}", function.name));
                    }
                }
                "py" => {
                    tests.push(format!("test_{}_valid_input", function.name));
                    tests.push(format!("test_{}_edge_cases", function.name));
                    if function.is_public {
                        tests.push(format!("test_{}_invalid_input", function.name));
                    }
                }
                _ => {}
            }
        }

        tests
    }

    /// Calculate coverage percentage
    fn calculate_coverage_percentage(&self, coverage: &[TestCoverage]) -> f64 {
        if coverage.is_empty() {
            return 100.0;
        }

        let tested = coverage.iter().filter(|c| c.has_tests).count();
        (tested as f64 / coverage.len() as f64) * 100.0
    }

    /// Format suggestions as a PR comment
    pub fn format_pr_comment(&self, result: &ChangeAnalysisResult) -> String {
        let mut comment = String::new();

        comment.push_str("## Test Coverage Analysis\n\n");
        comment.push_str(&format!(
            "**Coverage:** {:.1}% of changed functions have tests\n\n",
            result.coverage_percentage
        ));

        if result.suggestions.is_empty() {
            comment.push_str("✅ All changed functions have test coverage!\n");
            return comment;
        }

        comment.push_str("### Missing Tests\n\n");
        comment.push_str(&format!(
            "Found {} function(s) without tests:\n\n",
            result.suggestions.len()
        ));

        // Group by priority
        let high_priority: Vec<_> = result
            .suggestions
            .iter()
            .filter(|s| s.priority == Priority::High)
            .collect();
        let medium_priority: Vec<_> = result
            .suggestions
            .iter()
            .filter(|s| s.priority == Priority::Medium)
            .collect();
        let low_priority: Vec<_> = result
            .suggestions
            .iter()
            .filter(|s| s.priority == Priority::Low)
            .collect();

        if !high_priority.is_empty() {
            comment.push_str("#### 🔴 High Priority\n\n");
            for suggestion in high_priority {
                comment.push_str(&self.format_suggestion(suggestion));
            }
        }

        if !medium_priority.is_empty() {
            comment.push_str("#### 🟡 Medium Priority\n\n");
            for suggestion in medium_priority {
                comment.push_str(&self.format_suggestion(suggestion));
            }
        }

        if !low_priority.is_empty() {
            comment.push_str("#### 🟢 Low Priority\n\n");
            for suggestion in low_priority {
                comment.push_str(&self.format_suggestion(suggestion));
            }
        }

        comment
    }

    /// Format a single suggestion
    fn format_suggestion(&self, suggestion: &TestSuggestion) -> String {
        let mut text = String::new();

        text.push_str(&format!(
            "**`{}`** in `{}`\n",
            suggestion.function.name,
            suggestion.function.file_path.display()
        ));
        text.push_str(&format!("- {}\n", suggestion.reason));
        text.push_str("- Suggested tests:\n");
        for test in &suggestion.suggested_tests {
            text.push_str(&format!("  - `{}`\n", test));
        }
        text.push_str("\n");

        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_file_path() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let line = "diff --git a/src/main.rs b/src/main.rs";
        let path = analyzer.extract_file_path(line);

        assert_eq!(path, Some(PathBuf::from("src/main.rs")));
    }

    #[test]
    fn test_extract_line_number() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let line = "@@ -10,7 +15,8 @@ impl Foo {";
        let num = analyzer.extract_line_number(line);

        assert_eq!(num, Some(15));
    }

    #[test]
    fn test_extract_rust_function() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let line = "pub fn calculate_sum(a: i32, b: i32) -> i32 {";
        let function = analyzer.extract_rust_function(
            line,
            PathBuf::from("src/math.rs"),
            10,
        );

        assert!(function.is_some());
        let func = function.unwrap();
        assert_eq!(func.name, "calculate_sum");
        assert_eq!(func.is_public, true);
        assert_eq!(func.line_number, 10);
    }

    #[test]
    fn test_extract_rust_private_function() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let line = "fn helper_function() {";
        let function = analyzer.extract_rust_function(
            line,
            PathBuf::from("src/utils.rs"),
            20,
        );

        assert!(function.is_some());
        let func = function.unwrap();
        assert_eq!(func.name, "helper_function");
        assert_eq!(func.is_public, false);
    }

    #[test]
    fn test_extract_typescript_function() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let line = "export function processData(input: string): number {";
        let function = analyzer.extract_typescript_function(
            line,
            PathBuf::from("src/processor.ts"),
            5,
        );

        assert!(function.is_some());
        let func = function.unwrap();
        assert_eq!(func.name, "processData");
        assert_eq!(func.is_public, true);
    }

    #[test]
    fn test_extract_typescript_arrow_function() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let line = "const formatValue = (val: number) => {";
        let function = analyzer.extract_typescript_function(
            line,
            PathBuf::from("src/formatter.ts"),
            15,
        );

        assert!(function.is_some());
        let func = function.unwrap();
        assert_eq!(func.name, "formatValue");
    }

    #[test]
    fn test_extract_python_function() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let line = "def calculate_total(items: list) -> float:";
        let function = analyzer.extract_python_function(
            line,
            PathBuf::from("src/calculator.py"),
            8,
        );

        assert!(function.is_some());
        let func = function.unwrap();
        assert_eq!(func.name, "calculate_total");
        assert_eq!(func.is_public, true);
    }

    #[test]
    fn test_extract_python_private_function() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let line = "def _internal_helper():";
        let function = analyzer.extract_python_function(
            line,
            PathBuf::from("src/helpers.py"),
            12,
        );

        assert!(function.is_some());
        let func = function.unwrap();
        assert_eq!(func.name, "_internal_helper");
        assert_eq!(func.is_public, false);
    }

    #[test]
    fn test_calculate_priority_public_function() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let function = ChangedFunction {
            name: "public_api".to_string(),
            file_path: PathBuf::from("src/api.rs"),
            line_number: 10,
            change_type: ChangeType::Added,
            signature: "pub fn public_api() {}".to_string(),
            is_public: true,
        };

        let priority = analyzer.calculate_priority(&function);
        assert_eq!(priority, Priority::High);
    }

    #[test]
    fn test_calculate_priority_private_function() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let function = ChangedFunction {
            name: "internal_func".to_string(),
            file_path: PathBuf::from("src/internal.rs"),
            line_number: 5,
            change_type: ChangeType::Modified,
            signature: "fn internal_func() {}".to_string(),
            is_public: false,
        };

        let priority = analyzer.calculate_priority(&function);
        assert_eq!(priority, Priority::Medium);
    }

    #[test]
    fn test_calculate_coverage_percentage() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let coverage = vec![
            TestCoverage {
                function: ChangedFunction {
                    name: "func1".to_string(),
                    file_path: PathBuf::from("src/a.rs"),
                    line_number: 1,
                    change_type: ChangeType::Added,
                    signature: "fn func1() {}".to_string(),
                    is_public: true,
                },
                has_tests: true,
                test_files: vec![],
                test_names: vec!["test_func1".to_string()],
            },
            TestCoverage {
                function: ChangedFunction {
                    name: "func2".to_string(),
                    file_path: PathBuf::from("src/b.rs"),
                    line_number: 1,
                    change_type: ChangeType::Added,
                    signature: "fn func2() {}".to_string(),
                    is_public: false,
                },
                has_tests: false,
                test_files: vec![],
                test_names: vec![],
            },
        ];

        let percentage = analyzer.calculate_coverage_percentage(&coverage);
        assert_eq!(percentage, 50.0);
    }

    #[test]
    fn test_generate_test_cases_rust() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let function = ChangedFunction {
            name: "process_data".to_string(),
            file_path: PathBuf::from("src/processor.rs"),
            line_number: 10,
            change_type: ChangeType::Added,
            signature: "pub fn process_data(input: &str) -> String {}".to_string(),
            is_public: true,
        };

        let tests = analyzer.generate_test_cases(&function);

        assert!(tests.contains(&"test_process_data_happy_path".to_string()));
        assert!(tests.contains(&"test_process_data_edge_cases".to_string()));
        assert!(tests.contains(&"test_process_data_error_handling".to_string()));
    }

    #[test]
    fn test_extract_ts_test_name() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let line = "test('should process valid input', async () => {";
        let name = analyzer.extract_ts_test_name(line);

        assert_eq!(name, Some("should process valid input".to_string()));
    }

    #[tokio::test]
    async fn test_parse_diff_rust() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let diff = r#"diff --git a/src/calculator.rs b/src/calculator.rs
index 1234567..abcdefg 100644
--- a/src/calculator.rs
+++ b/src/calculator.rs
@@ -10,6 +10,10 @@ impl Calculator {
     }

+    pub fn multiply(a: i32, b: i32) -> i32 {
+        a * b
+    }
+
     fn internal_helper() {
         // helper
     }
"#;

        let functions = analyzer.parse_diff(diff).await.unwrap();

        assert_eq!(functions.len(), 1);
        assert_eq!(functions[0].name, "multiply");
        assert_eq!(functions[0].is_public, true);
    }

    #[test]
    fn test_format_pr_comment_with_suggestions() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let result = ChangeAnalysisResult {
            changed_functions: vec![],
            coverage: vec![],
            suggestions: vec![
                TestSuggestion {
                    function: ChangedFunction {
                        name: "new_feature".to_string(),
                        file_path: PathBuf::from("src/feature.rs"),
                        line_number: 42,
                        change_type: ChangeType::Added,
                        signature: "pub fn new_feature() {}".to_string(),
                        is_public: true,
                    },
                    suggested_tests: vec![
                        "test_new_feature_happy_path".to_string(),
                        "test_new_feature_edge_cases".to_string(),
                    ],
                    priority: Priority::High,
                    reason: "New public function 'new_feature' added without tests".to_string(),
                },
            ],
            coverage_percentage: 0.0,
        };

        let comment = analyzer.format_pr_comment(&result);

        assert!(comment.contains("Test Coverage Analysis"));
        assert!(comment.contains("0.0% of changed functions have tests"));
        assert!(comment.contains("High Priority"));
        assert!(comment.contains("new_feature"));
        assert!(comment.contains("test_new_feature_happy_path"));
    }

    #[test]
    fn test_format_pr_comment_all_covered() {
        let analyzer = ChangeTestAnalyzer::new(PathBuf::from("/repo"));

        let result = ChangeAnalysisResult {
            changed_functions: vec![],
            coverage: vec![],
            suggestions: vec![],
            coverage_percentage: 100.0,
        };

        let comment = analyzer.format_pr_comment(&result);

        assert!(comment.contains("100.0% of changed functions have tests"));
        assert!(comment.contains("All changed functions have test coverage!"));
    }
}
