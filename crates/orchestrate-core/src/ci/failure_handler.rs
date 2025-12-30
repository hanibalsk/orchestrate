//! CI Failure Handler
//!
//! Handles CI failures and determines appropriate responses.

use crate::ci::result_parser::{LogParser, TestResultParser};
use crate::ci_integration::{CiFailureAnalysis, CiRun, FailedJob, FailedTest};
use crate::database::Database;
use crate::error::Result;

/// Response to a CI failure
#[derive(Debug, Clone)]
pub enum FailureResponse {
    /// Spawn an issue-fixer agent
    SpawnIssueFixer {
        run_id: String,
        context: String,
    },
    /// Retry the run (likely flaky)
    RetryRun {
        run_id: String,
        reason: String,
    },
    /// Manual intervention required
    ManualReview {
        run_id: String,
        reason: String,
    },
    /// No action needed
    NoAction,
}

/// Handles CI failures and determines appropriate responses
pub struct FailureHandler {
    db: Database,
}

impl FailureHandler {
    /// Create a new failure handler
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Analyze a failed CI run and determine response
    pub async fn analyze_failure(
        &self,
        run: &CiRun,
        logs: &str,
    ) -> Result<(CiFailureAnalysis, FailureResponse)> {
        let mut analysis = CiFailureAnalysis::new(&run.id);

        // Parse failed tests
        analysis.failed_tests = TestResultParser::parse_failed_tests(logs);

        // Parse failed jobs
        for job in &run.jobs {
            if job.conclusion == Some(crate::ci_integration::CiConclusion::Failure) {
                let failed_jobs = LogParser::parse_failed_jobs(logs, &job.name);
                analysis.failed_jobs.extend(failed_jobs);
            }
        }

        // Extract error messages
        analysis.error_messages = TestResultParser::extract_error_messages(logs);

        // Detect flaky tests
        self.detect_flaky_tests(&mut analysis).await?;

        // Generate recommendations
        self.generate_recommendations(&mut analysis);

        // Determine response
        let response = self.determine_response(&analysis);

        Ok((analysis, response))
    }

    /// Detect flaky tests based on historical data
    async fn detect_flaky_tests(&self, analysis: &mut CiFailureAnalysis) -> Result<()> {
        let mut flaky_count = 0;
        let total_tests = analysis.failed_tests.len();

        for test in &mut analysis.failed_tests {
            // Check if error message suggests flakiness
            if LogParser::is_likely_flaky(&test.error_message) {
                test.is_flaky = true;
                flaky_count += 1;
            }

            // TODO: Query database for historical failure rates
            // If a test fails frequently but inconsistently, mark as flaky
        }

        if total_tests > 0 {
            let flaky_ratio = flaky_count as f64 / total_tests as f64;
            if flaky_ratio > 0.5 {
                analysis.is_flaky = true;
                analysis.flaky_confidence = flaky_ratio;
            }
        }

        Ok(())
    }

    /// Generate recommendations based on analysis
    fn generate_recommendations(&self, analysis: &mut CiFailureAnalysis) {
        if analysis.is_flaky && analysis.flaky_confidence > 0.7 {
            analysis.add_recommendation("High likelihood of flaky tests - consider retrying");
            analysis.add_recommendation("Review tests for non-deterministic behavior");
        }

        if !analysis.failed_tests.is_empty() {
            analysis.add_recommendation(&format!(
                "Fix {} failing test(s)",
                analysis.failed_tests.len()
            ));
        }

        if !analysis.failed_jobs.is_empty() {
            let recommendations: Vec<String> = analysis.failed_jobs.iter()
                .map(|job| format!("Review job '{}': {}", job.job_name, job.error_summary))
                .collect();

            for rec in recommendations {
                analysis.add_recommendation(&rec);
            }
        }

        if analysis.should_auto_fix() {
            analysis.add_recommendation("Auto-fix available - spawn issue-fixer agent");
        }
    }

    /// Determine the appropriate response to a failure
    fn determine_response(&self, analysis: &CiFailureAnalysis) -> FailureResponse {
        // If highly likely flaky, retry
        if analysis.is_flaky && analysis.flaky_confidence > 0.8 {
            return FailureResponse::RetryRun {
                run_id: analysis.run_id.clone(),
                reason: format!(
                    "Likely flaky (confidence: {:.0}%)",
                    analysis.flaky_confidence * 100.0
                ),
            };
        }

        // If should auto-fix, spawn issue-fixer
        if analysis.should_auto_fix() {
            let context = analysis.to_summary();
            return FailureResponse::SpawnIssueFixer {
                run_id: analysis.run_id.clone(),
                context,
            };
        }

        // If no clear path forward, require manual review
        if !analysis.failed_tests.is_empty() || !analysis.failed_jobs.is_empty() {
            return FailureResponse::ManualReview {
                run_id: analysis.run_id.clone(),
                reason: "Complex failure requiring manual review".to_string(),
            };
        }

        FailureResponse::NoAction
    }

    /// Store failure analysis in database
    pub async fn store_analysis(&self, _analysis: &CiFailureAnalysis) -> Result<i64> {
        // TODO: Implement database storage
        // For now, return a mock ID
        Ok(1)
    }

    /// Track flaky test pattern
    pub async fn track_flaky_test(&self, _test_name: &str, _run_id: &str) -> Result<()> {
        // TODO: Implement database tracking
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci_integration::{CiConclusion, CiJob, CiProvider, CiRunStatus};
    use tempfile::NamedTempFile;

    async fn create_test_db() -> Database {
        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();
        Database::new(db_path).await.unwrap()
    }

    #[tokio::test]
    async fn test_analyze_failure_with_tests() {
        let db = create_test_db().await;
        let handler = FailureHandler::new(db);

        let mut run = CiRun::new("test-run", CiProvider::GitHubActions, "test", "main");
        run.conclusion = Some(CiConclusion::Failure);

        let logs = r#"
---- test_example stdout ----
thread 'test_example' panicked at 'assertion failed'
"#;

        let (analysis, response) = handler.analyze_failure(&run, logs).await.unwrap();

        assert_eq!(analysis.failed_tests.len(), 1);
        assert_eq!(analysis.failed_tests[0].test_name, "test_example");
    }

    #[tokio::test]
    async fn test_analyze_failure_flaky_detection() {
        let db = create_test_db().await;
        let handler = FailureHandler::new(db);

        let run = CiRun::new("test-run", CiProvider::GitHubActions, "test", "main");

        let logs = r#"
---- test_network_call stdout ----
thread 'test_network_call' panicked at 'connection timeout after 30s'

---- test_api_call stdout ----
thread 'test_api_call' panicked at 'Network connection refused'
"#;

        let (analysis, _response) = handler.analyze_failure(&run, logs).await.unwrap();

        assert!(analysis.is_flaky);
        assert!(analysis.flaky_confidence > 0.0);
    }

    #[tokio::test]
    async fn test_determine_response_auto_fix() {
        let db = create_test_db().await;
        let handler = FailureHandler::new(db);

        let mut analysis = CiFailureAnalysis::new("test-run");
        analysis.failed_tests.push(FailedTest {
            test_name: "test_something".to_string(),
            test_file: None,
            error_message: "assertion failed".to_string(),
            stack_trace: None,
            failure_count: 1,
            is_flaky: false,
        });

        let response = handler.determine_response(&analysis);

        match response {
            FailureResponse::SpawnIssueFixer { run_id, .. } => {
                assert_eq!(run_id, "test-run");
            }
            _ => panic!("Expected SpawnIssueFixer response"),
        }
    }

    #[tokio::test]
    async fn test_determine_response_retry_flaky() {
        let db = create_test_db().await;
        let handler = FailureHandler::new(db);

        let mut analysis = CiFailureAnalysis::new("test-run");
        analysis.is_flaky = true;
        analysis.flaky_confidence = 0.9;

        let response = handler.determine_response(&analysis);

        match response {
            FailureResponse::RetryRun { run_id, reason } => {
                assert_eq!(run_id, "test-run");
                assert!(reason.contains("flaky"));
            }
            _ => panic!("Expected RetryRun response"),
        }
    }

    #[tokio::test]
    async fn test_generate_recommendations() {
        let db = create_test_db().await;
        let handler = FailureHandler::new(db);

        let mut analysis = CiFailureAnalysis::new("test-run");
        analysis.failed_tests.push(FailedTest {
            test_name: "test1".to_string(),
            test_file: None,
            error_message: "failed".to_string(),
            stack_trace: None,
            failure_count: 1,
            is_flaky: false,
        });

        handler.generate_recommendations(&mut analysis);

        assert!(!analysis.recommendations.is_empty());
        assert!(analysis.recommendations.iter().any(|r| r.contains("Fix")));
    }

    #[tokio::test]
    async fn test_detect_flaky_tests() {
        let db = create_test_db().await;
        let handler = FailureHandler::new(db);

        let mut analysis = CiFailureAnalysis::new("test-run");
        analysis.failed_tests.push(FailedTest {
            test_name: "flaky_test".to_string(),
            test_file: None,
            error_message: "connection timeout".to_string(),
            stack_trace: None,
            failure_count: 1,
            is_flaky: false,
        });

        handler.detect_flaky_tests(&mut analysis).await.unwrap();

        assert!(analysis.failed_tests[0].is_flaky);
    }
}
