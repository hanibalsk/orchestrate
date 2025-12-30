//! CI Client Abstraction
//!
//! Provides a common trait for all CI provider implementations.

use crate::ci_integration::{CiArtifact, CiConfig, CiRun, CiTriggerRequest};
use crate::error::Result;
use async_trait::async_trait;

/// Trait that all CI providers must implement
#[async_trait]
pub trait CiClientTrait: Send + Sync {
    /// Trigger a new workflow/pipeline run
    async fn trigger_run(&self, request: &CiTriggerRequest) -> Result<CiRun>;

    /// Get the status of a specific run
    async fn get_run_status(&self, run_id: &str) -> Result<CiRun>;

    /// Get the logs for a specific run
    async fn get_run_logs(&self, run_id: &str, job_name: Option<&str>) -> Result<String>;

    /// Cancel a running workflow/pipeline
    async fn cancel_run(&self, run_id: &str) -> Result<()>;

    /// Retry a failed run
    async fn retry_run(&self, run_id: &str) -> Result<CiRun>;

    /// List artifacts for a run
    async fn list_artifacts(&self, run_id: &str) -> Result<Vec<CiArtifact>>;

    /// Download an artifact
    async fn download_artifact(&self, run_id: &str, artifact_name: &str) -> Result<Vec<u8>>;

    /// Wait for a run to complete (with timeout)
    async fn wait_for_completion(&self, run_id: &str, timeout_secs: u64) -> Result<CiRun>;
}

/// CI Client factory
pub struct CiClient;

impl CiClient {
    /// Create a new CI client based on the configuration
    pub fn new(config: CiConfig) -> Result<Box<dyn CiClientTrait>> {
        use crate::ci_integration::CiProvider;

        match config.provider {
            CiProvider::GitHubActions => {
                Ok(Box::new(super::GitHubActionsClient::new(config)?))
            }
            CiProvider::GitLabCi => {
                Ok(Box::new(super::GitLabCiClient::new(config)?))
            }
            CiProvider::CircleCi => {
                Ok(Box::new(super::CircleCiClient::new(config)?))
            }
            CiProvider::JenkinsCI => {
                Err(crate::error::Error::Config("Jenkins CI not yet implemented".to_string()))
            }
            CiProvider::Custom => {
                Err(crate::error::Error::Config("Custom CI provider not yet implemented".to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ci_integration::{CiAuthType, CiProvider};
    use std::collections::HashMap;

    #[test]
    fn test_client_factory_github_actions() {
        let config = CiConfig {
            provider: CiProvider::GitHubActions,
            api_url: Some("https://api.github.com".to_string()),
            auth_type: CiAuthType::Bearer,
            token: Some("test-token".to_string()),
            custom_config: HashMap::new(),
        };

        let result = CiClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_factory_gitlab() {
        let config = CiConfig {
            provider: CiProvider::GitLabCi,
            api_url: Some("https://gitlab.com/api/v4".to_string()),
            auth_type: CiAuthType::Bearer,
            token: Some("test-token".to_string()),
            custom_config: HashMap::new(),
        };

        let result = CiClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_factory_circleci() {
        let config = CiConfig {
            provider: CiProvider::CircleCi,
            api_url: Some("https://circleci.com/api/v2".to_string()),
            auth_type: CiAuthType::Bearer,
            token: Some("test-token".to_string()),
            custom_config: HashMap::new(),
        };

        let result = CiClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_client_factory_unsupported() {
        let config = CiConfig {
            provider: CiProvider::JenkinsCI,
            api_url: Some("http://localhost:8080".to_string()),
            auth_type: CiAuthType::Bearer,
            token: Some("test-token".to_string()),
            custom_config: HashMap::new(),
        };

        let result = CiClient::new(config);
        assert!(result.is_err());
    }
}
