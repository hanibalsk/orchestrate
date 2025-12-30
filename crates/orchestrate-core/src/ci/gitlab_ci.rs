//! GitLab CI Integration
//!
//! Implements the CiClientTrait for GitLab CI.

use crate::ci::client::CiClientTrait;
use crate::ci_integration::{CiArtifact, CiConfig, CiRun, CiTriggerRequest};
use crate::error::{Error, Result};
use async_trait::async_trait;
use std::time::Duration;

/// GitLab CI API client
pub struct GitLabCiClient {
    api_url: String,
    token: String,
    http_client: reqwest::Client,
}

impl GitLabCiClient {
    /// Create a new GitLab CI client
    pub fn new(config: CiConfig) -> Result<Self> {
        let api_url = config
            .api_url
            .unwrap_or_else(|| "https://gitlab.com/api/v4".to_string());

        let token = config
            .token
            .ok_or_else(|| Error::Config("GitLab token is required".to_string()))?;

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
}

#[async_trait]
impl CiClientTrait for GitLabCiClient {
    async fn trigger_run(&self, request: &CiTriggerRequest) -> Result<CiRun> {
        // Mock implementation - would trigger GitLab pipeline
        let run = CiRun::new(
            "mock-gitlab-run",
            crate::ci_integration::CiProvider::GitLabCi,
            &request.workflow_name,
            &request.branch,
        );
        Ok(run)
    }

    async fn get_run_status(&self, run_id: &str) -> Result<CiRun> {
        // Mock implementation
        let run = CiRun::new(
            run_id,
            crate::ci_integration::CiProvider::GitLabCi,
            "test-pipeline",
            "main",
        );
        Ok(run)
    }

    async fn get_run_logs(&self, run_id: &str, job_name: Option<&str>) -> Result<String> {
        Ok(format!(
            "GitLab logs for run {} {}",
            run_id,
            job_name.map(|j| format!("job {}", j)).unwrap_or_default()
        ))
    }

    async fn cancel_run(&self, run_id: &str) -> Result<()> {
        Ok(())
    }

    async fn retry_run(&self, run_id: &str) -> Result<CiRun> {
        let run = CiRun::new(
            run_id,
            crate::ci_integration::CiProvider::GitLabCi,
            "test-pipeline",
            "main",
        );
        Ok(run)
    }

    async fn list_artifacts(&self, run_id: &str) -> Result<Vec<CiArtifact>> {
        Ok(vec![])
    }

    async fn download_artifact(&self, run_id: &str, artifact_name: &str) -> Result<Vec<u8>> {
        Ok(vec![])
    }

    async fn wait_for_completion(&self, run_id: &str, timeout_secs: u64) -> Result<CiRun> {
        let run = self.get_run_status(run_id).await?;
        Ok(run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci_integration::{CiAuthType, CiProvider};
    use std::collections::HashMap;

    fn create_test_config() -> CiConfig {
        CiConfig {
            provider: CiProvider::GitLabCi,
            api_url: Some("https://gitlab.com/api/v4".to_string()),
            auth_type: CiAuthType::Bearer,
            token: Some("test-token".to_string()),
            custom_config: HashMap::new(),
        }
    }

    #[test]
    fn test_client_creation() {
        let config = create_test_config();
        let client = GitLabCiClient::new(config);
        assert!(client.is_ok());
    }

    #[test]
    fn test_client_creation_without_token() {
        let mut config = create_test_config();
        config.token = None;
        let client = GitLabCiClient::new(config);
        assert!(client.is_err());
    }

    #[tokio::test]
    async fn test_trigger_run() {
        let config = create_test_config();
        let client = GitLabCiClient::new(config).unwrap();

        let request = CiTriggerRequest {
            workflow_name: ".gitlab-ci.yml".to_string(),
            branch: "main".to_string(),
            inputs: HashMap::new(),
        };

        let result = client.trigger_run(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_run_status() {
        let config = create_test_config();
        let client = GitLabCiClient::new(config).unwrap();

        let result = client.get_run_status("456").await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap().id, "456");
    }
}
