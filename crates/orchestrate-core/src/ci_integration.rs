//! CI/CD Platform Integration Module
//!
//! Types and utilities for integrating with CI/CD platforms like
//! GitHub Actions, GitLab CI, and CircleCI.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// CI Provider type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiProvider {
    GitHubActions,
    GitLabCi,
    CircleCi,
    JenkinsCI,
    Custom,
}

impl CiProvider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GitHubActions => "github_actions",
            Self::GitLabCi => "gitlab_ci",
            Self::CircleCi => "circleci",
            Self::JenkinsCI => "jenkins",
            Self::Custom => "custom",
        }
    }
}

impl std::fmt::Display for CiProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for CiProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "github_actions" | "github-actions" | "githubactions" => Ok(Self::GitHubActions),
            "gitlab_ci" | "gitlab-ci" | "gitlabci" => Ok(Self::GitLabCi),
            "circleci" | "circle_ci" | "circle-ci" => Ok(Self::CircleCi),
            "jenkins" | "jenkins_ci" | "jenkinsci" => Ok(Self::JenkinsCI),
            "custom" => Ok(Self::Custom),
            _ => Err(format!("Unknown CI provider: {}", s)),
        }
    }
}

/// CI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiConfig {
    pub provider: CiProvider,
    pub api_url: Option<String>,
    pub auth_type: CiAuthType,
    pub token: Option<String>,
    pub custom_config: HashMap<String, String>,
}

/// CI authentication type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiAuthType {
    Bearer,
    Basic,
    ApiKey,
    None,
}

impl Default for CiConfig {
    fn default() -> Self {
        Self {
            provider: CiProvider::GitHubActions,
            api_url: None,
            auth_type: CiAuthType::Bearer,
            token: None,
            custom_config: HashMap::new(),
        }
    }
}

/// CI workflow/pipeline run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiRun {
    pub id: String,
    pub provider: CiProvider,
    pub workflow_name: String,
    pub branch: String,
    pub commit_sha: Option<String>,
    pub status: CiRunStatus,
    pub conclusion: Option<CiConclusion>,
    pub jobs: Vec<CiJob>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<i64>,
    pub url: Option<String>,
    pub triggered_by: Option<String>,
}

/// CI run status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiRunStatus {
    Queued,
    InProgress,
    Completed,
    Cancelled,
    Skipped,
}

impl CiRunStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Cancelled | Self::Skipped)
    }
}

/// CI run conclusion (final result)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CiConclusion {
    Success,
    Failure,
    Cancelled,
    Skipped,
    TimedOut,
    ActionRequired,
    Neutral,
}

impl CiConclusion {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failure => "failure",
            Self::Cancelled => "cancelled",
            Self::Skipped => "skipped",
            Self::TimedOut => "timed_out",
            Self::ActionRequired => "action_required",
            Self::Neutral => "neutral",
        }
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Self::Success | Self::Skipped | Self::Neutral)
    }
}

/// CI job within a run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiJob {
    pub id: String,
    pub name: String,
    pub status: CiRunStatus,
    pub conclusion: Option<CiConclusion>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub steps: Vec<CiStep>,
}

/// CI step within a job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiStep {
    pub name: String,
    pub status: CiRunStatus,
    pub conclusion: Option<CiConclusion>,
    pub number: u32,
}

impl CiRun {
    /// Create a new CI run
    pub fn new(id: &str, provider: CiProvider, workflow_name: &str, branch: &str) -> Self {
        Self {
            id: id.to_string(),
            provider,
            workflow_name: workflow_name.to_string(),
            branch: branch.to_string(),
            commit_sha: None,
            status: CiRunStatus::Queued,
            conclusion: None,
            jobs: vec![],
            started_at: None,
            completed_at: None,
            duration_seconds: None,
            url: None,
            triggered_by: None,
        }
    }

    /// Check if run is still in progress
    pub fn is_running(&self) -> bool {
        !self.status.is_terminal()
    }

    /// Check if run completed successfully
    pub fn is_success(&self) -> bool {
        self.conclusion.map(|c| c.is_success()).unwrap_or(false)
    }

    /// Get failed jobs
    pub fn failed_jobs(&self) -> Vec<&CiJob> {
        self.jobs
            .iter()
            .filter(|j| j.conclusion == Some(CiConclusion::Failure))
            .collect()
    }
}

/// CI failure analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiFailureAnalysis {
    pub run_id: String,
    pub failed_jobs: Vec<FailedJob>,
    pub failed_tests: Vec<FailedTest>,
    pub error_messages: Vec<String>,
    pub is_flaky: bool,
    pub flaky_confidence: f64,
    pub recommendations: Vec<String>,
    pub analyzed_at: DateTime<Utc>,
}

/// Information about a failed job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedJob {
    pub job_name: String,
    pub step_name: Option<String>,
    pub error_summary: String,
    pub log_url: Option<String>,
}

/// Information about a failed test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedTest {
    pub test_name: String,
    pub test_file: Option<String>,
    pub error_message: String,
    pub stack_trace: Option<String>,
    pub failure_count: u32,
    pub is_flaky: bool,
}

impl CiFailureAnalysis {
    /// Create a new failure analysis
    pub fn new(run_id: &str) -> Self {
        Self {
            run_id: run_id.to_string(),
            failed_jobs: vec![],
            failed_tests: vec![],
            error_messages: vec![],
            is_flaky: false,
            flaky_confidence: 0.0,
            recommendations: vec![],
            analyzed_at: Utc::now(),
        }
    }

    /// Add a recommendation
    pub fn add_recommendation(&mut self, recommendation: &str) {
        self.recommendations.push(recommendation.to_string());
    }

    /// Determine if auto-fix should be attempted
    pub fn should_auto_fix(&self) -> bool {
        // Don't auto-fix if it's likely flaky
        if self.is_flaky && self.flaky_confidence > 0.7 {
            return false;
        }

        // Auto-fix if we have specific test failures
        !self.failed_tests.is_empty()
    }

    /// Generate summary
    pub fn to_summary(&self) -> String {
        let mut output = format!("CI Failure Analysis for run: {}\n", self.run_id);
        output.push_str(&format!("Analyzed at: {}\n\n", self.analyzed_at.format("%Y-%m-%d %H:%M:%S UTC")));

        if !self.failed_jobs.is_empty() {
            output.push_str(&format!("Failed Jobs ({}):\n", self.failed_jobs.len()));
            for job in &self.failed_jobs {
                output.push_str(&format!("  - {}: {}\n", job.job_name, job.error_summary));
            }
            output.push('\n');
        }

        if !self.failed_tests.is_empty() {
            output.push_str(&format!("Failed Tests ({}):\n", self.failed_tests.len()));
            for test in &self.failed_tests {
                let flaky_marker = if test.is_flaky { " [FLAKY]" } else { "" };
                output.push_str(&format!("  - {}{}: {}\n", test.test_name, flaky_marker, test.error_message));
            }
            output.push('\n');
        }

        if self.is_flaky {
            output.push_str(&format!("Flaky Test Detected (confidence: {:.0}%)\n", self.flaky_confidence * 100.0));
        }

        if !self.recommendations.is_empty() {
            output.push_str("\nRecommendations:\n");
            for rec in &self.recommendations {
                output.push_str(&format!("  - {}\n", rec));
            }
        }

        output
    }
}

/// CI artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiArtifact {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub download_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request to trigger a CI run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiTriggerRequest {
    pub workflow_name: String,
    pub branch: String,
    pub inputs: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn test_ci_provider_from_str() {
        assert_eq!(
            CiProvider::from_str("github_actions").unwrap(),
            CiProvider::GitHubActions
        );
        assert_eq!(
            CiProvider::from_str("gitlab-ci").unwrap(),
            CiProvider::GitLabCi
        );
        assert_eq!(
            CiProvider::from_str("circleci").unwrap(),
            CiProvider::CircleCi
        );
        assert!(CiProvider::from_str("unknown").is_err());
    }

    #[test]
    fn test_ci_run_status() {
        assert!(CiRunStatus::Completed.is_terminal());
        assert!(CiRunStatus::Cancelled.is_terminal());
        assert!(!CiRunStatus::InProgress.is_terminal());
        assert!(!CiRunStatus::Queued.is_terminal());
    }

    #[test]
    fn test_ci_conclusion_success() {
        assert!(CiConclusion::Success.is_success());
        assert!(CiConclusion::Skipped.is_success());
        assert!(CiConclusion::Neutral.is_success());
        assert!(!CiConclusion::Failure.is_success());
        assert!(!CiConclusion::TimedOut.is_success());
    }

    #[test]
    fn test_ci_run_creation() {
        let run = CiRun::new("123", CiProvider::GitHubActions, "test", "main");

        assert_eq!(run.id, "123");
        assert_eq!(run.provider, CiProvider::GitHubActions);
        assert_eq!(run.workflow_name, "test");
        assert_eq!(run.branch, "main");
        assert!(run.is_running());
        assert!(!run.is_success());
    }

    #[test]
    fn test_ci_failure_analysis() {
        let mut analysis = CiFailureAnalysis::new("run-123");
        analysis.failed_tests.push(FailedTest {
            test_name: "test_something".to_string(),
            test_file: Some("tests/test_mod.rs".to_string()),
            error_message: "assertion failed".to_string(),
            stack_trace: None,
            failure_count: 1,
            is_flaky: false,
        });
        analysis.add_recommendation("Fix the test assertion");

        assert!(analysis.should_auto_fix());
        let summary = analysis.to_summary();
        assert!(summary.contains("test_something"));
        assert!(summary.contains("assertion failed"));
    }

    #[test]
    fn test_flaky_detection_prevents_auto_fix() {
        let mut analysis = CiFailureAnalysis::new("run-456");
        analysis.is_flaky = true;
        analysis.flaky_confidence = 0.9;
        analysis.failed_tests.push(FailedTest {
            test_name: "flaky_test".to_string(),
            test_file: None,
            error_message: "intermittent failure".to_string(),
            stack_trace: None,
            failure_count: 5,
            is_flaky: true,
        });

        // Should not auto-fix flaky tests with high confidence
        assert!(!analysis.should_auto_fix());
    }
}
