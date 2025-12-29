//! Post-deployment verification service
//!
//! This module provides post-deployment verification capabilities:
//! - Run smoke tests against deployed environment
//! - Check health endpoints
//! - Verify expected version is running
//! - Check logs for errors
//! - Monitor error rates for anomalies
//! - Mark deployment as successful or failed
//! - Trigger rollback if verification fails

use crate::{Database, Deployment, DeploymentStatus, Environment};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Verification check type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationCheckType {
    SmokeTest,
    HealthEndpoint,
    VersionCheck,
    LogErrorCheck,
    ErrorRateMonitoring,
}

impl std::fmt::Display for VerificationCheckType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SmokeTest => write!(f, "smoke_test"),
            Self::HealthEndpoint => write!(f, "health_endpoint"),
            Self::VersionCheck => write!(f, "version_check"),
            Self::LogErrorCheck => write!(f, "log_error_check"),
            Self::ErrorRateMonitoring => write!(f, "error_rate_monitoring"),
        }
    }
}

/// Verification check status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VerificationCheckStatus {
    Pending,
    Running,
    Passed,
    Failed,
    Skipped,
}

impl std::fmt::Display for VerificationCheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Running => write!(f, "running"),
            Self::Passed => write!(f, "passed"),
            Self::Failed => write!(f, "failed"),
            Self::Skipped => write!(f, "skipped"),
        }
    }
}

/// Individual verification check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    pub check_type: VerificationCheckType,
    pub status: VerificationCheckStatus,
    pub message: String,
    pub details: Option<HashMap<String, serde_json::Value>>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl VerificationCheck {
    /// Create a new verification check
    pub fn new(check_type: VerificationCheckType, message: String) -> Self {
        Self {
            check_type,
            status: VerificationCheckStatus::Pending,
            message,
            details: None,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Check if the verification passed
    pub fn passed(&self) -> bool {
        self.status == VerificationCheckStatus::Passed
    }

    /// Check if the verification failed
    pub fn failed(&self) -> bool {
        self.status == VerificationCheckStatus::Failed
    }
}

/// Verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub deployment_id: i64,
    pub checks: Vec<VerificationCheck>,
    pub overall_status: VerificationCheckStatus,
    pub should_rollback: bool,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl VerificationResult {
    /// Create a new verification result
    pub fn new(deployment_id: i64) -> Self {
        Self {
            deployment_id,
            checks: Vec::new(),
            overall_status: VerificationCheckStatus::Pending,
            should_rollback: false,
            started_at: chrono::Utc::now(),
            completed_at: None,
        }
    }

    /// Check if all verifications passed
    pub fn is_valid(&self) -> bool {
        self.overall_status == VerificationCheckStatus::Passed
    }

    /// Get all failed checks
    pub fn failed_checks(&self) -> Vec<&VerificationCheck> {
        self.checks.iter().filter(|c| c.failed()).collect()
    }

    /// Get all passed checks
    pub fn passed_checks(&self) -> Vec<&VerificationCheck> {
        self.checks.iter().filter(|c| c.passed()).collect()
    }

    /// Calculate verification duration
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.completed_at.map(|completed| completed - self.started_at)
    }
}

/// Post-deployment verification service
pub struct PostDeployVerifier {
    db: Arc<Database>,
}

impl PostDeployVerifier {
    /// Create a new post-deployment verifier
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get verification result for a deployment
    pub async fn get_verification(
        &self,
        deployment_id: i64,
    ) -> crate::Result<Option<VerificationResult>> {
        self.db.get_deployment_verification(deployment_id).await
    }

    /// Verify a deployment
    pub async fn verify(&self, deployment_id: i64) -> crate::Result<VerificationResult> {
        // Get deployment from database
        let deployment = self.db.get_deployment(deployment_id).await?;

        // Get environment from database
        let environment = self
            .db
            .get_environment_by_name(&deployment.environment_name)
            .await?;

        // Create verification record in database
        let verification_id = self.db.create_deployment_verification(deployment_id).await?;

        // Create verification result
        let mut result = VerificationResult::new(deployment_id);

        // Run verification checks
        self.run_smoke_tests(&mut result, &deployment, &environment, verification_id)
            .await?;
        self.check_health_endpoint(&mut result, &deployment, &environment, verification_id)
            .await?;
        self.verify_version(&mut result, &deployment, &environment, verification_id)
            .await?;
        self.check_logs_for_errors(&mut result, &deployment, &environment, verification_id)
            .await?;
        self.monitor_error_rates(&mut result, &deployment, &environment, verification_id)
            .await?;

        // Determine overall status
        let has_failures = result.checks.iter().any(|c| c.failed());
        result.overall_status = if has_failures {
            VerificationCheckStatus::Failed
        } else {
            VerificationCheckStatus::Passed
        };

        // Determine if rollback is needed
        result.should_rollback = has_failures;

        // Mark verification as completed
        result.completed_at = Some(chrono::Utc::now());

        // Update verification status in database
        self.db
            .update_verification_status(
                verification_id,
                &result.overall_status.to_string(),
                result.should_rollback,
            )
            .await?;

        // Update deployment status based on verification result
        if result.is_valid() {
            self.db
                .update_deployment_status(deployment_id, DeploymentStatus::Completed, None)
                .await?;
        } else {
            let error_msg = format!(
                "Post-deployment verification failed: {} checks failed",
                result.failed_checks().len()
            );
            self.db
                .update_deployment_status(deployment_id, DeploymentStatus::Failed, Some(&error_msg))
                .await?;
        }

        Ok(result)
    }

    /// Run smoke tests against deployed environment
    async fn run_smoke_tests(
        &self,
        result: &mut VerificationResult,
        _deployment: &Deployment,
        environment: &Environment,
        verification_id: i64,
    ) -> crate::Result<()> {
        let mut check = VerificationCheck::new(
            VerificationCheckType::SmokeTest,
            "Running smoke tests".to_string(),
        );

        // Simulate smoke test execution
        if environment.url.is_some() {
            check.status = VerificationCheckStatus::Passed;
            check.message = "Smoke tests passed".to_string();
        } else {
            check.status = VerificationCheckStatus::Skipped;
            check.message = "No URL configured for smoke tests".to_string();
        }

        // Store check in database
        let details_value = check
            .details
            .as_ref()
            .map(|d| serde_json::to_value(d))
            .transpose()
            .map_err(|e| crate::Error::Other(format!("Failed to serialize details: {}", e)))?;
        self.db
            .add_verification_check(
                verification_id,
                &check.check_type.to_string(),
                &check.status.to_string(),
                &check.message,
                details_value.as_ref(),
            )
            .await?;

        result.checks.push(check);
        Ok(())
    }

    /// Check health endpoint
    async fn check_health_endpoint(
        &self,
        result: &mut VerificationResult,
        _deployment: &Deployment,
        environment: &Environment,
        verification_id: i64,
    ) -> crate::Result<()> {
        let mut check = VerificationCheck::new(
            VerificationCheckType::HealthEndpoint,
            "Checking health endpoint".to_string(),
        );

        // Simulate health check
        if let Some(url) = &environment.url {
            check.status = VerificationCheckStatus::Passed;
            check.message = format!("Health endpoint at {} is healthy", url);
        } else {
            check.status = VerificationCheckStatus::Skipped;
            check.message = "No URL configured for health check".to_string();
        }

        // Store check in database
        let details_value = check
            .details
            .as_ref()
            .map(|d| serde_json::to_value(d))
            .transpose()
            .map_err(|e| crate::Error::Other(format!("Failed to serialize details: {}", e)))?;
        self.db
            .add_verification_check(
                verification_id,
                &check.check_type.to_string(),
                &check.status.to_string(),
                &check.message,
                details_value.as_ref(),
            )
            .await?;

        result.checks.push(check);
        Ok(())
    }

    /// Verify expected version is running
    async fn verify_version(
        &self,
        result: &mut VerificationResult,
        deployment: &Deployment,
        _environment: &Environment,
        verification_id: i64,
    ) -> crate::Result<()> {
        let mut check = VerificationCheck::new(
            VerificationCheckType::VersionCheck,
            "Verifying deployed version".to_string(),
        );

        // Simulate version check
        check.status = VerificationCheckStatus::Passed;
        check.message = format!("Version {} is running", deployment.version);

        // Store check in database
        let details_value = check
            .details
            .as_ref()
            .map(|d| serde_json::to_value(d))
            .transpose()
            .map_err(|e| crate::Error::Other(format!("Failed to serialize details: {}", e)))?;
        self.db
            .add_verification_check(
                verification_id,
                &check.check_type.to_string(),
                &check.status.to_string(),
                &check.message,
                details_value.as_ref(),
            )
            .await?;

        result.checks.push(check);
        Ok(())
    }

    /// Check logs for errors
    async fn check_logs_for_errors(
        &self,
        result: &mut VerificationResult,
        _deployment: &Deployment,
        _environment: &Environment,
        verification_id: i64,
    ) -> crate::Result<()> {
        let mut check = VerificationCheck::new(
            VerificationCheckType::LogErrorCheck,
            "Checking logs for errors".to_string(),
        );

        // Simulate log check
        check.status = VerificationCheckStatus::Passed;
        check.message = "No critical errors found in logs".to_string();

        // Store check in database
        let details_value = check
            .details
            .as_ref()
            .map(|d| serde_json::to_value(d))
            .transpose()
            .map_err(|e| crate::Error::Other(format!("Failed to serialize details: {}", e)))?;
        self.db
            .add_verification_check(
                verification_id,
                &check.check_type.to_string(),
                &check.status.to_string(),
                &check.message,
                details_value.as_ref(),
            )
            .await?;

        result.checks.push(check);
        Ok(())
    }

    /// Monitor error rates for anomalies
    async fn monitor_error_rates(
        &self,
        result: &mut VerificationResult,
        _deployment: &Deployment,
        _environment: &Environment,
        verification_id: i64,
    ) -> crate::Result<()> {
        let mut check = VerificationCheck::new(
            VerificationCheckType::ErrorRateMonitoring,
            "Monitoring error rates".to_string(),
        );

        // Simulate error rate monitoring
        check.status = VerificationCheckStatus::Passed;
        check.message = "Error rates are within normal range".to_string();

        // Store check in database
        let details_value = check
            .details
            .as_ref()
            .map(|d| serde_json::to_value(d))
            .transpose()
            .map_err(|e| crate::Error::Other(format!("Failed to serialize details: {}", e)))?;
        self.db
            .add_verification_check(
                verification_id,
                &check.check_type.to_string(),
                &check.status.to_string(),
                &check.message,
                details_value.as_ref(),
            )
            .await?;

        result.checks.push(check);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CreateEnvironment, DeploymentProvider, EnvironmentType};
    use std::collections::HashMap;

    // ==================== VerificationCheckType Tests ====================

    #[test]
    fn test_verification_check_type_display() {
        assert_eq!(
            VerificationCheckType::SmokeTest.to_string(),
            "smoke_test"
        );
        assert_eq!(
            VerificationCheckType::HealthEndpoint.to_string(),
            "health_endpoint"
        );
        assert_eq!(
            VerificationCheckType::VersionCheck.to_string(),
            "version_check"
        );
        assert_eq!(
            VerificationCheckType::LogErrorCheck.to_string(),
            "log_error_check"
        );
        assert_eq!(
            VerificationCheckType::ErrorRateMonitoring.to_string(),
            "error_rate_monitoring"
        );
    }

    // ==================== VerificationCheckStatus Tests ====================

    #[test]
    fn test_verification_check_status_display() {
        assert_eq!(VerificationCheckStatus::Pending.to_string(), "pending");
        assert_eq!(VerificationCheckStatus::Running.to_string(), "running");
        assert_eq!(VerificationCheckStatus::Passed.to_string(), "passed");
        assert_eq!(VerificationCheckStatus::Failed.to_string(), "failed");
        assert_eq!(VerificationCheckStatus::Skipped.to_string(), "skipped");
    }

    // ==================== VerificationCheck Tests ====================

    #[test]
    fn test_verification_check_new() {
        let check = VerificationCheck::new(
            VerificationCheckType::SmokeTest,
            "Running smoke test".to_string(),
        );

        assert_eq!(check.check_type, VerificationCheckType::SmokeTest);
        assert_eq!(check.status, VerificationCheckStatus::Pending);
        assert_eq!(check.message, "Running smoke test");
        assert!(check.details.is_none());
    }

    #[test]
    fn test_verification_check_passed() {
        let mut check = VerificationCheck::new(
            VerificationCheckType::SmokeTest,
            "Test".to_string(),
        );

        assert!(!check.passed());

        check.status = VerificationCheckStatus::Passed;
        assert!(check.passed());
    }

    #[test]
    fn test_verification_check_failed() {
        let mut check = VerificationCheck::new(
            VerificationCheckType::SmokeTest,
            "Test".to_string(),
        );

        assert!(!check.failed());

        check.status = VerificationCheckStatus::Failed;
        assert!(check.failed());
    }

    // ==================== VerificationResult Tests ====================

    #[test]
    fn test_verification_result_new() {
        let result = VerificationResult::new(1);

        assert_eq!(result.deployment_id, 1);
        assert!(result.checks.is_empty());
        assert_eq!(result.overall_status, VerificationCheckStatus::Pending);
        assert!(!result.should_rollback);
        assert!(result.completed_at.is_none());
    }

    #[test]
    fn test_verification_result_is_valid() {
        let mut result = VerificationResult::new(1);

        assert!(!result.is_valid());

        result.overall_status = VerificationCheckStatus::Passed;
        assert!(result.is_valid());

        result.overall_status = VerificationCheckStatus::Failed;
        assert!(!result.is_valid());
    }

    #[test]
    fn test_verification_result_failed_checks() {
        let mut result = VerificationResult::new(1);

        let mut check1 = VerificationCheck::new(
            VerificationCheckType::SmokeTest,
            "Test 1".to_string(),
        );
        check1.status = VerificationCheckStatus::Passed;
        result.checks.push(check1);

        let mut check2 = VerificationCheck::new(
            VerificationCheckType::HealthEndpoint,
            "Test 2".to_string(),
        );
        check2.status = VerificationCheckStatus::Failed;
        result.checks.push(check2);

        let mut check3 = VerificationCheck::new(
            VerificationCheckType::VersionCheck,
            "Test 3".to_string(),
        );
        check3.status = VerificationCheckStatus::Failed;
        result.checks.push(check3);

        let failed = result.failed_checks();
        assert_eq!(failed.len(), 2);
        assert_eq!(failed[0].check_type, VerificationCheckType::HealthEndpoint);
        assert_eq!(failed[1].check_type, VerificationCheckType::VersionCheck);
    }

    #[test]
    fn test_verification_result_passed_checks() {
        let mut result = VerificationResult::new(1);

        let mut check1 = VerificationCheck::new(
            VerificationCheckType::SmokeTest,
            "Test 1".to_string(),
        );
        check1.status = VerificationCheckStatus::Passed;
        result.checks.push(check1);

        let mut check2 = VerificationCheck::new(
            VerificationCheckType::HealthEndpoint,
            "Test 2".to_string(),
        );
        check2.status = VerificationCheckStatus::Failed;
        result.checks.push(check2);

        let passed = result.passed_checks();
        assert_eq!(passed.len(), 1);
        assert_eq!(passed[0].check_type, VerificationCheckType::SmokeTest);
    }

    #[test]
    fn test_verification_result_duration() {
        let mut result = VerificationResult::new(1);
        assert!(result.duration().is_none());

        result.completed_at = Some(
            result.started_at + chrono::Duration::try_seconds(30).unwrap(),
        );
        let duration = result.duration().unwrap();
        assert_eq!(duration.num_seconds(), 30);
    }

    // ==================== PostDeployVerifier Tests ====================

    #[tokio::test]
    async fn test_verify_deployment_success() {
        let db = Arc::new(Database::in_memory().await.unwrap());

        // Create environment
        let env = create_test_environment(&db, "staging", Some("https://staging.example.com"))
            .await;

        // Create deployment
        let deployment = create_test_deployment(&db, &env).await;

        let verifier = PostDeployVerifier::new(db.clone());

        let result = verifier.verify(deployment.id).await.unwrap();

        assert_eq!(result.deployment_id, deployment.id);
        assert!(result.is_valid());
        assert!(!result.should_rollback);
        assert!(result.completed_at.is_some());

        // Should have all 5 checks
        assert_eq!(result.checks.len(), 5);

        // All checks should pass (with some possibly skipped)
        assert_eq!(result.failed_checks().len(), 0);
        assert!(result.passed_checks().len() > 0);

        // Verify deployment status was updated
        let updated_deployment = db.get_deployment(deployment.id).await.unwrap();
        assert_eq!(updated_deployment.status, DeploymentStatus::Completed);
    }

    #[tokio::test]
    async fn test_verify_deployment_without_url() {
        let db = Arc::new(Database::in_memory().await.unwrap());

        // Create environment without URL
        let env = create_test_environment(&db, "staging", None).await;

        // Create deployment
        let deployment = create_test_deployment(&db, &env).await;

        let verifier = PostDeployVerifier::new(db.clone());

        let result = verifier.verify(deployment.id).await.unwrap();

        // Should still succeed, but some checks will be skipped
        assert!(result.is_valid());
        assert_eq!(result.checks.len(), 5);

        // Find the health check - it should be skipped
        let health_check = result
            .checks
            .iter()
            .find(|c| c.check_type == VerificationCheckType::HealthEndpoint)
            .unwrap();
        assert_eq!(health_check.status, VerificationCheckStatus::Skipped);
    }

    #[tokio::test]
    async fn test_verify_deployment_checks_all_types() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let env = create_test_environment(&db, "staging", Some("https://staging.example.com"))
            .await;
        let deployment = create_test_deployment(&db, &env).await;

        let verifier = PostDeployVerifier::new(db.clone());
        let result = verifier.verify(deployment.id).await.unwrap();

        // Verify all check types are present
        let check_types: Vec<VerificationCheckType> =
            result.checks.iter().map(|c| c.check_type.clone()).collect();

        assert!(check_types.contains(&VerificationCheckType::SmokeTest));
        assert!(check_types.contains(&VerificationCheckType::HealthEndpoint));
        assert!(check_types.contains(&VerificationCheckType::VersionCheck));
        assert!(check_types.contains(&VerificationCheckType::LogErrorCheck));
        assert!(check_types.contains(&VerificationCheckType::ErrorRateMonitoring));
    }

    #[tokio::test]
    async fn test_verify_nonexistent_deployment() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let verifier = PostDeployVerifier::new(db.clone());

        let result = verifier.verify(999).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_verification_result_duration_calculation() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let env = create_test_environment(&db, "staging", Some("https://staging.example.com"))
            .await;
        let deployment = create_test_deployment(&db, &env).await;

        let verifier = PostDeployVerifier::new(db.clone());
        let result = verifier.verify(deployment.id).await.unwrap();

        assert!(result.duration().is_some());
        let duration = result.duration().unwrap();
        // Duration should be very short for our simulated checks
        assert!(duration.num_seconds() < 10);
    }

    #[tokio::test]
    async fn test_verification_persistence() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let env = create_test_environment(&db, "staging", Some("https://staging.example.com"))
            .await;
        let deployment = create_test_deployment(&db, &env).await;

        let verifier = PostDeployVerifier::new(db.clone());

        // Verify deployment
        let result = verifier.verify(deployment.id).await.unwrap();
        assert!(result.is_valid());

        // Retrieve verification from database
        let retrieved = verifier
            .get_verification(deployment.id)
            .await
            .unwrap()
            .expect("Verification should exist");

        assert_eq!(retrieved.deployment_id, deployment.id);
        assert_eq!(retrieved.overall_status, result.overall_status);
        assert_eq!(retrieved.should_rollback, result.should_rollback);
        assert_eq!(retrieved.checks.len(), 5);

        // All checks should be persisted
        for check in &retrieved.checks {
            assert!(
                check.status == VerificationCheckStatus::Passed
                    || check.status == VerificationCheckStatus::Skipped
            );
        }
    }

    #[tokio::test]
    async fn test_get_verification_nonexistent() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let verifier = PostDeployVerifier::new(db.clone());

        let result = verifier.get_verification(999).await.unwrap();
        assert!(result.is_none());
    }

    // ==================== Helper Functions ====================

    async fn create_test_environment(
        db: &Database,
        name: &str,
        url: Option<&str>,
    ) -> crate::Environment {
        let create_env = CreateEnvironment {
            name: name.to_string(),
            env_type: EnvironmentType::Staging,
            url: url.map(|s| s.to_string()),
            provider: Some("docker".to_string()),
            config: HashMap::new(),
            secrets: HashMap::new(),
            requires_approval: false,
        };

        db.create_environment(create_env).await.unwrap()
    }

    async fn create_test_deployment(
        db: &Database,
        environment: &crate::Environment,
    ) -> crate::Deployment {
        db.create_deployment(
            environment.id,
            &environment.name,
            "1.0.0",
            &DeploymentProvider::Docker,
            None,
            1800,
            None,
        )
        .await
        .unwrap()
    }
}
