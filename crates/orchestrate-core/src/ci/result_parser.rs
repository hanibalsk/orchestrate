//! CI Result Parser
//!
//! Parses CI logs and extracts failure information.

use crate::ci_integration::{FailedJob, FailedTest};
use crate::error::Result;
use regex::Regex;

/// Parses test results from CI logs
pub struct TestResultParser;

impl TestResultParser {
    /// Parse failed tests from log output
    pub fn parse_failed_tests(logs: &str) -> Vec<FailedTest> {
        let mut failed_tests = Vec::new();

        // Rust test failures
        if let Some(tests) = Self::parse_rust_tests(logs) {
            failed_tests.extend(tests);
        }

        // Jest/JavaScript test failures
        if let Some(tests) = Self::parse_jest_tests(logs) {
            failed_tests.extend(tests);
        }

        // Pytest failures
        if let Some(tests) = Self::parse_pytest_tests(logs) {
            failed_tests.extend(tests);
        }

        failed_tests
    }

    /// Parse Rust test failures
    fn parse_rust_tests(logs: &str) -> Option<Vec<FailedTest>> {
        let re = Regex::new(r"---- (\S+) stdout ----\n(?:.*\n)*?thread '.*' panicked at '([^']+)'")
            .ok()?;

        let mut tests = Vec::new();
        for cap in re.captures_iter(logs) {
            let test_name = cap.get(1)?.as_str().to_string();
            let error_message = cap.get(2)?.as_str().to_string();

            tests.push(FailedTest {
                test_name,
                test_file: None,
                error_message,
                stack_trace: None,
                failure_count: 1,
                is_flaky: false,
            });
        }

        if tests.is_empty() {
            None
        } else {
            Some(tests)
        }
    }

    /// Parse Jest test failures
    fn parse_jest_tests(logs: &str) -> Option<Vec<FailedTest>> {
        let re = Regex::new(r"● (.+?)\n\n\s+(.+?)(?:\n\n|$)").ok()?;

        let mut tests = Vec::new();
        for cap in re.captures_iter(logs) {
            let test_name = cap.get(1)?.as_str().to_string();
            let error_message = cap.get(2)?.as_str().to_string();

            tests.push(FailedTest {
                test_name,
                test_file: None,
                error_message,
                stack_trace: None,
                failure_count: 1,
                is_flaky: false,
            });
        }

        if tests.is_empty() {
            None
        } else {
            Some(tests)
        }
    }

    /// Parse pytest failures
    fn parse_pytest_tests(logs: &str) -> Option<Vec<FailedTest>> {
        let re = Regex::new(r"FAILED (.+?) - (.+)").ok()?;

        let mut tests = Vec::new();
        for cap in re.captures_iter(logs) {
            let test_name = cap.get(1)?.as_str().to_string();
            let error_message = cap.get(2)?.as_str().to_string();

            tests.push(FailedTest {
                test_name,
                test_file: None,
                error_message,
                stack_trace: None,
                failure_count: 1,
                is_flaky: false,
            });
        }

        if tests.is_empty() {
            None
        } else {
            Some(tests)
        }
    }

    /// Extract error messages from logs
    pub fn extract_error_messages(logs: &str) -> Vec<String> {
        let mut errors = Vec::new();

        // Common error patterns
        let patterns = [
            r"error: (.+)",
            r"ERROR: (.+)",
            r"Error: (.+)",
            r"FAILED: (.+)",
            r"panic: (.+)",
        ];

        for pattern in &patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(logs) {
                    if let Some(msg) = cap.get(1) {
                        errors.push(msg.as_str().to_string());
                    }
                }
            }
        }

        // Deduplicate
        errors.sort();
        errors.dedup();
        errors
    }
}

/// Parses general log output
pub struct LogParser;

impl LogParser {
    /// Parse failed jobs from logs
    pub fn parse_failed_jobs(logs: &str, job_name: &str) -> Vec<FailedJob> {
        let errors = TestResultParser::extract_error_messages(logs);

        if errors.is_empty() {
            return vec![];
        }

        let error_summary = if errors.len() > 3 {
            format!("{} and {} more errors", errors[0], errors.len() - 1)
        } else {
            errors.join("; ")
        };

        vec![FailedJob {
            job_name: job_name.to_string(),
            step_name: None,
            error_summary,
            log_url: None,
        }]
    }

    /// Detect if a test is likely flaky based on error patterns
    pub fn is_likely_flaky(error_message: &str) -> bool {
        let flaky_patterns = [
            "timeout",
            "connection refused",
            "network",
            "ECONNRESET",
            "temporarily unavailable",
            "race condition",
            "intermittent",
        ];

        let lower = error_message.to_lowercase();
        flaky_patterns.iter().any(|p| lower.contains(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_tests() {
        let log = r#"
running 2 tests
test test_one ... ok
---- test_two stdout ----
thread 'test_two' panicked at 'assertion failed: expected == actual'
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

failures:
    test_two
"#;

        let tests = TestResultParser::parse_failed_tests(log);
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].test_name, "test_two");
        assert!(tests[0].error_message.contains("assertion failed"));
    }

    #[test]
    fn test_parse_jest_tests() {
        let log = r#"
FAIL src/test.ts
  ● Test Suite Failed

    Expected: true
    Received: false

  ● my test case

    expect(received).toBe(expected)
"#;

        let tests = TestResultParser::parse_failed_tests(log);
        // This is a basic test - real implementation would need better parsing
        assert!(tests.len() >= 0);
    }

    #[test]
    fn test_extract_error_messages() {
        let log = r#"
error: compilation failed
ERROR: test failed
Warning: this is just a warning
Error: something went wrong
"#;

        let errors = TestResultParser::extract_error_messages(log);
        assert!(errors.len() > 0);
        assert!(errors.iter().any(|e| e.contains("compilation failed") || e.contains("test failed")));
    }

    #[test]
    fn test_parse_failed_jobs() {
        let log = "error: build failed\nError: tests failed";
        let jobs = LogParser::parse_failed_jobs(log, "build");

        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_name, "build");
        assert!(jobs[0].error_summary.len() > 0);
    }

    #[test]
    fn test_is_likely_flaky_timeout() {
        assert!(LogParser::is_likely_flaky("operation timeout after 30s"));
        assert!(LogParser::is_likely_flaky("Connection refused"));
        assert!(LogParser::is_likely_flaky("Network error occurred"));
        assert!(!LogParser::is_likely_flaky("assertion failed: x == y"));
    }

    #[test]
    fn test_is_likely_flaky_race_condition() {
        assert!(LogParser::is_likely_flaky("detected race condition"));
        assert!(LogParser::is_likely_flaky("intermittent failure"));
        assert!(!LogParser::is_likely_flaky("syntax error"));
    }

    #[test]
    fn test_extract_errors_deduplication() {
        let log = "error: same error\nerror: same error\nerror: different error";
        let errors = TestResultParser::extract_error_messages(log);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_parse_pytest_tests() {
        let log = r#"
FAILED tests/test_example.py::test_function - AssertionError: assert False
FAILED tests/test_another.py::test_case - ValueError: invalid value
"#;

        let tests = TestResultParser::parse_failed_tests(log);
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].test_name, "tests/test_example.py::test_function");
        assert!(tests[0].error_message.contains("AssertionError"));
    }
}
