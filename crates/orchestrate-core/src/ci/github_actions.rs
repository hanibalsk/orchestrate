//! GitHub Actions CI Integration
//!
//! Implements the CiClientTrait for GitHub Actions.

use crate::ci::client::CiClientTrait;
use crate::ci_integration::{
    CiArtifact, CiConfig, CiConclusion, CiJob, CiRun, CiRunStatus, CiStep, CiTriggerRequest,
};
use crate::error::{Error, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// GitHub Actions API client
pub struct GitHubActionsClient {
    api_url: String,
    token: String,
    http_client: reqwest::Client,
}

impl GitHubActionsClient {
    /// Create a new GitHub Actions client
    pub fn new(config: CiConfig) -> Result<Self> {
        let api_url = config
            .api_url
            .unwrap_or_else(|| "https://api.github.com".to_string());

        let token = config
            .token
            .ok_or_else(|| Error::Config("GitHub token is required".to_string()))?;

        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("orchestrate-ci")
            .build()
            .map_err(|e| Error::Other(e.to_string()))?;

        Ok(Self {
            api_url,
            token,
            http_client,
        })
    }

    /// Extract owner and repo from a GitHub repository string (e.g., "owner/repo")
    fn parse_repo(&self, repo: &str) -> Result<(String, String)> {
        let parts: Vec<&str> = repo.split('/').collect();
        if parts.len() != 2 {
            return Err(Error::Config(format!(
                "Invalid repository format: {}. Expected 'owner/repo'",
                repo
            )));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Convert GitHub workflow run status to CiRunStatus
    fn map_status(status: &str) -> CiRunStatus {
        match status {
            "queued" | "pending" | "waiting" => CiRunStatus::Queued,
            "in_progress" => CiRunStatus::InProgress,
            "completed" => CiRunStatus::Completed,
            _ => CiRunStatus::Queued,
        }
    }

    /// Convert GitHub workflow run conclusion to CiConclusion
    fn map_conclusion(conclusion: Option<&str>) -> Option<CiConclusion> {
        conclusion.and_then(|c| match c {
            "success" => Some(CiConclusion::Success),
            "failure" => Some(CiConclusion::Failure),
            "cancelled" => Some(CiConclusion::Cancelled),
            "skipped" => Some(CiConclusion::Skipped),
            "timed_out" => Some(CiConclusion::TimedOut),
            "action_required" => Some(CiConclusion::ActionRequired),
            "neutral" => Some(CiConclusion::Neutral),
            _ => None,
        })
    }

    /// Parse workflow run from GitHub API response
    fn parse_workflow_run(&self, run: &GitHubWorkflowRun) -> CiRun {
        let status = Self::map_status(&run.status);
        let conclusion = Self::map_conclusion(run.conclusion.as_deref());

        CiRun {
            id: run.id.to_string(),
            provider: crate::ci_integration::CiProvider::GitHubActions,
            workflow_name: run.name.clone(),
            branch: run.head_branch.clone(),
            commit_sha: Some(run.head_sha.clone()),
            status,
            conclusion,
            jobs: vec![], // Jobs need to be fetched separately
            started_at: run.run_started_at,
            completed_at: run.updated_at,
            duration_seconds: None,
            url: Some(run.html_url.clone()),
            triggered_by: run.actor.as_ref().map(|a| a.login.clone()),
        }
    }
}

#[async_trait]
impl CiClientTrait for GitHubActionsClient {
    async fn trigger_run(&self, request: &CiTriggerRequest) -> Result<CiRun> {
        // For GitHub Actions, we need owner/repo from custom_config
        // In a real implementation, this would dispatch a workflow_dispatch event

        // Return a mock run for now (will implement real API call)
        let run = CiRun::new(
            "mock-run-id",
            crate::ci_integration::CiProvider::GitHubActions,
            &request.workflow_name,
            &request.branch,
        );

        Ok(run)
    }

    async fn get_run_status(&self, run_id: &str) -> Result<CiRun> {
        // Mock implementation - in production would call GitHub API
        // GET /repos/{owner}/{repo}/actions/runs/{run_id}

        let run = CiRun::new(
            run_id,
            crate::ci_integration::CiProvider::GitHubActions,
            "test-workflow",
            "main",
        );

        Ok(run)
    }

    async fn get_run_logs(&self, run_id: &str, job_name: Option<&str>) -> Result<String> {
        // Mock implementation
        Ok(format!(
            "Logs for run {} {}",
            run_id,
            job_name.map(|j| format!("job {}", j)).unwrap_or_default()
        ))
    }

    async fn cancel_run(&self, run_id: &str) -> Result<()> {
        // Mock implementation
        // POST /repos/{owner}/{repo}/actions/runs/{run_id}/cancel
        Ok(())
    }

    async fn retry_run(&self, run_id: &str) -> Result<CiRun> {
        // Mock implementation
        // POST /repos/{owner}/{repo}/actions/runs/{run_id}/rerun
        let run = CiRun::new(
            run_id,
            crate::ci_integration::CiProvider::GitHubActions,
            "test-workflow",
            "main",
        );

        Ok(run)
    }

    async fn list_artifacts(&self, run_id: &str) -> Result<Vec<CiArtifact>> {
        // Mock implementation
        // GET /repos/{owner}/{repo}/actions/runs/{run_id}/artifacts
        Ok(vec![])
    }

    async fn download_artifact(&self, run_id: &str, artifact_name: &str) -> Result<Vec<u8>> {
        // Mock implementation
        Ok(vec![])
    }

    async fn wait_for_completion(&self, run_id: &str, timeout_secs: u64) -> Result<CiRun> {
        // Mock implementation - would poll until complete or timeout
        let run = self.get_run_status(run_id).await?;
        Ok(run)
    }
}

// GitHub API response types
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitHubWorkflowRun {
    id: u64,
    name: String,
    head_branch: String,
    head_sha: String,
    status: String,
    conclusion: Option<String>,
    html_url: String,
    run_started_at: Option<DateTime<Utc>>,
    updated_at: Option<DateTime<Utc>>,
    actor: Option<GitHubActor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitHubActor {
    login: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci_integration::{CiAuthType, CiProvider};
    use std::collections::HashMap;

    fn create_test_config() -> CiConfig {
        CiConfig {
            provider: CiProvider::GitHubActions,
            api_url: Some("https://api.github.com".to_string()),
            auth_type: CiAuthType::Bearer,
            token: Some("test-token".to_string()),
            custom_config: HashMap::new(),
        }
    }

    #[test]
    fn test_client_creation() {
        let config = create_test_config();
        let client = GitHubActionsClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_creation_without_token() {
        let mut config = create_test_config();
        config.token = None;
        let client = GitHubActionsClient::new(config);
        assert!(client.is_err());
    }

    #[test]
    fn test_parse_repo_valid() {
        let config = create_test_config();
        let client = GitHubActionsClient::new(config).unwrap();
        let result = client.parse_repo("owner/repo");
        assert!(result.is_ok());
        let (owner, repo) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn test_parse_repo_invalid() {
        let config = create_test_config();
        let client = GitHubActionsClient::new(config).unwrap();

        let result = client.parse_repo("invalid");
        assert!(result.is_err());

        let result = client.parse_repo("too/many/parts");
        assert!(result.is_err());
    }

    #[test]
    fn test_map_status() {
        assert_eq!(GitHubActionsClient::map_status("queued"), CiRunStatus::Queued);
        assert_eq!(GitHubActionsClient::map_status("in_progress"), CiRunStatus::InProgress);
        assert_eq!(GitHubActionsClient::map_status("completed"), CiRunStatus::Completed);
    }

    #[test]
    fn test_map_conclusion() {
        assert_eq!(
            GitHubActionsClient::map_conclusion(Some("success")),
            Some(CiConclusion::Success)
        );
        assert_eq!(
            GitHubActionsClient::map_conclusion(Some("failure")),
            Some(CiConclusion::Failure)
        );
        assert_eq!(
            GitHubActionsClient::map_conclusion(None),
            None
        );
    }

    #[tokio::test]
    async fn test_trigger_run() {
        let config = create_test_config();
        let client = GitHubActionsClient::new(config).unwrap();

        let request = CiTriggerRequest {
            workflow_name: "test.yml".to_string(),
            branch: "main".to_string(),
            inputs: HashMap::new(),
        };

        let result = client.trigger_run(&request).await;
        assert!(result.is_ok());

        let run = result.unwrap();
        assert_eq!(run.workflow_name, "test.yml");
        assert_eq!(run.branch, "main");
    }

    #[tokio::test]
    async fn test_get_run_status() {
        let config = create_test_config();
        let client = GitHubActionsClient::new(config).unwrap();

        let result = client.get_run_status("123").await;
        assert!(result.is_ok());

        let run = result.unwrap();
        assert_eq!(run.id, "123");
    }

    #[tokio::test]
    async fn test_get_run_logs() {
        let config = create_test_config();
        let client = GitHubActionsClient::new(config).unwrap();

        let result = client.get_run_logs("123", Some("test-job")).await;
        assert!(result.is_ok());
        assert!(result.unwrap().contains("123"));
    }

    #[tokio::test]
    async fn test_cancel_run() {
        let config = create_test_config();
        let client = GitHubActionsClient::new(config).unwrap();

        let result = client.cancel_run("123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_run() {
        let config = create_test_config();
        let client = GitHubActionsClient::new(config).unwrap();

        let result = client.retry_run("123").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_list_artifacts() {
        let config = create_test_config();
        let client = GitHubActionsClient::new(config).unwrap();

        let result = client.list_artifacts("123").await;
        assert!(result.is_ok());
    }
}
