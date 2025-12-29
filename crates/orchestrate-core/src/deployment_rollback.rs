//! Deployment rollback service
//!
//! This module provides deployment rollback capabilities:
//! - Rollback to previous version automatically
//! - Rollback to specific version
//! - Fast rollback for blue-green deployments (traffic switch)
//! - Record rollback events
//! - Notify on rollback
//!
//! Supports different rollback strategies based on the deployment provider
//! and strategy used for the original deployment.

use crate::{Database, Deployment, DeploymentExecutor, DeploymentProvider, DeploymentRequest, DeploymentStatus};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Rollback event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackEvent {
    pub id: i64,
    pub deployment_id: i64,
    pub target_version: String,
    pub rollback_type: RollbackType,
    pub status: RollbackStatus,
    pub error_message: Option<String>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub notification_sent: bool,
}

/// Type of rollback being performed
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackType {
    /// Rollback to previous deployment version
    Previous,
    /// Rollback to a specific version
    Specific,
    /// Fast rollback using blue-green traffic switch
    BlueGreenSwitch,
    /// Automatic rollback triggered by validation failure
    Automatic,
}

impl std::fmt::Display for RollbackType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Previous => write!(f, "previous"),
            Self::Specific => write!(f, "specific"),
            Self::BlueGreenSwitch => write!(f, "blue_green_switch"),
            Self::Automatic => write!(f, "automatic"),
        }
    }
}

/// Rollback status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RollbackStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

impl std::fmt::Display for RollbackStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

/// Rollback request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackRequest {
    /// Environment name to rollback
    pub environment: String,
    /// Optional specific version to rollback to
    pub target_version: Option<String>,
    /// Skip validation checks
    pub skip_validation: bool,
    /// Force rollback even if environment is not in failed state
    pub force: bool,
}

/// Rollback notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackNotification {
    pub rollback_id: i64,
    pub environment: String,
    pub from_version: String,
    pub to_version: String,
    pub rollback_type: RollbackType,
    pub status: RollbackStatus,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Deployment rollback service
pub struct DeploymentRollback {
    db: Arc<Database>,
    executor: DeploymentExecutor,
}

impl DeploymentRollback {
    /// Create a new rollback service
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            executor: DeploymentExecutor::new(Arc::clone(&db)),
            db,
        }
    }

    /// Rollback to previous deployment
    pub async fn rollback(
        &self,
        request: RollbackRequest,
    ) -> crate::Result<RollbackEvent> {
        // Get current deployment
        let current_deployment = self.get_current_deployment(&request.environment).await?;

        // Validate rollback is allowed
        if !request.force && !self.should_allow_rollback(&current_deployment) {
            return Err(crate::Error::Other(format!(
                "Deployment is in {} state. Use --force to rollback anyway.",
                current_deployment.status
            )));
        }

        // Determine target version and rollback type
        let is_specific_version = request.target_version.is_some();
        let target_version = if let Some(ref version) = request.target_version {
            version.clone()
        } else {
            self.get_previous_successful_version(&request.environment, &current_deployment.version)
                .await?
                .ok_or_else(|| crate::Error::Other("No previous successful deployment found".to_string()))?
        };

        // Determine rollback type
        let rollback_type = self.determine_rollback_type(&current_deployment, is_specific_version);

        // Create rollback event
        let rollback_event = self
            .db
            .create_deployment_rollback_event(
                current_deployment.id,
                &target_version,
                &rollback_type,
            )
            .await?;

        // Update status to in progress
        let rollback_event = self
            .db
            .update_deployment_rollback_status(rollback_event.id, RollbackStatus::InProgress, None)
            .await?;

        // Execute rollback based on type
        match self.execute_rollback(
            &rollback_event,
            &current_deployment,
            &request,
        ).await {
            Ok(_) => {
                // Update status to completed
                let rollback_event = self
                    .db
                    .update_deployment_rollback_status(rollback_event.id, RollbackStatus::Completed, None)
                    .await?;

                // Send notification
                self.send_rollback_notification(&rollback_event, &current_deployment).await?;

                // Fetch updated rollback event with notification_sent flag
                let rollback_event = self.db.get_deployment_rollback_event(rollback_event.id).await?;

                Ok(rollback_event)
            }
            Err(e) => {
                let _rollback_event = self
                    .db
                    .update_deployment_rollback_status(
                        rollback_event.id,
                        RollbackStatus::Failed,
                        Some(&e.to_string()),
                    )
                    .await?;
                Err(e)
            }
        }
    }

    /// Get rollback event by ID
    pub async fn get_rollback(&self, rollback_id: i64) -> crate::Result<RollbackEvent> {
        self.db.get_deployment_rollback_event(rollback_id).await
    }

    /// List rollback events for an environment
    pub async fn list_rollbacks(
        &self,
        environment: &str,
        limit: Option<i64>,
    ) -> crate::Result<Vec<RollbackEvent>> {
        self.db.list_deployment_rollback_events(environment, limit).await
    }

    /// Check if rollback should be allowed
    fn should_allow_rollback(&self, deployment: &Deployment) -> bool {
        matches!(
            deployment.status,
            DeploymentStatus::Failed | DeploymentStatus::TimedOut
        )
    }

    /// Get current deployment for an environment
    async fn get_current_deployment(&self, environment: &str) -> crate::Result<Deployment> {
        let deployments = self.executor.list_deployments(environment, Some(1)).await?;
        deployments
            .into_iter()
            .next()
            .ok_or_else(|| crate::Error::Other(format!("No deployments found for environment: {}", environment)))
    }

    /// Get previous successful deployment version
    async fn get_previous_successful_version(
        &self,
        environment: &str,
        current_version: &str,
    ) -> crate::Result<Option<String>> {
        let deployments = self.executor.list_deployments(environment, Some(10)).await?;

        Ok(deployments
            .into_iter()
            .filter(|d| d.status == DeploymentStatus::Completed && d.version != current_version)
            .map(|d| d.version)
            .next())
    }

    /// Determine the type of rollback to perform
    fn determine_rollback_type(
        &self,
        current_deployment: &Deployment,
        is_specific_version: bool,
    ) -> RollbackType {
        if is_specific_version {
            return RollbackType::Specific;
        }

        // Check if we can do a fast blue-green switch
        if let Some(strategy) = &current_deployment.strategy {
            if strategy.strategy_type == crate::deployment_strategy::StrategyType::BlueGreen {
                return RollbackType::BlueGreenSwitch;
            }
        }

        RollbackType::Previous
    }

    /// Execute the rollback
    async fn execute_rollback(
        &self,
        rollback_event: &RollbackEvent,
        current_deployment: &Deployment,
        request: &RollbackRequest,
    ) -> crate::Result<()> {
        match rollback_event.rollback_type {
            RollbackType::BlueGreenSwitch => {
                self.execute_blue_green_rollback(current_deployment).await
            }
            _ => {
                self.execute_standard_rollback(
                    &request.environment,
                    &rollback_event.target_version,
                    &current_deployment.provider,
                    request.skip_validation,
                ).await
            }
        }
    }

    /// Execute blue-green rollback (fast traffic switch)
    async fn execute_blue_green_rollback(
        &self,
        _current_deployment: &Deployment,
    ) -> crate::Result<()> {
        // Simulate fast traffic switch
        // In a real implementation, this would update load balancer configuration
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        Ok(())
    }

    /// Execute standard rollback (redeploy previous version)
    async fn execute_standard_rollback(
        &self,
        environment: &str,
        target_version: &str,
        provider: &DeploymentProvider,
        skip_validation: bool,
    ) -> crate::Result<()> {
        let deploy_request = DeploymentRequest {
            environment: environment.to_string(),
            version: target_version.to_string(),
            provider: Some(provider.clone()),
            strategy: None,
            timeout_seconds: None,
            skip_validation,
        };

        let deployment = self.executor.deploy(deploy_request).await?;

        if deployment.status != DeploymentStatus::Completed {
            return Err(crate::Error::Other(format!(
                "Rollback deployment failed with status: {}",
                deployment.status
            )));
        }

        Ok(())
    }

    /// Send rollback notification
    async fn send_rollback_notification(
        &self,
        rollback_event: &RollbackEvent,
        current_deployment: &Deployment,
    ) -> crate::Result<()> {
        let notification = RollbackNotification {
            rollback_id: rollback_event.id,
            environment: current_deployment.environment_name.clone(),
            from_version: current_deployment.version.clone(),
            to_version: rollback_event.target_version.clone(),
            rollback_type: rollback_event.rollback_type.clone(),
            status: rollback_event.status.clone(),
            timestamp: chrono::Utc::now(),
        };

        // Mark notification as sent
        self.db
            .mark_deployment_rollback_notification_sent(rollback_event.id)
            .await?;

        // In a real implementation, this would send notifications via email, Slack, etc.
        tracing::info!("Rollback notification: {:?}", notification);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CreateEnvironment, DeploymentStrategy, EnvironmentType};
    use std::collections::HashMap;

    // ==================== Type Tests ====================

    #[test]
    fn test_rollback_type_display() {
        assert_eq!(RollbackType::Previous.to_string(), "previous");
        assert_eq!(RollbackType::Specific.to_string(), "specific");
        assert_eq!(RollbackType::BlueGreenSwitch.to_string(), "blue_green_switch");
        assert_eq!(RollbackType::Automatic.to_string(), "automatic");
    }

    #[test]
    fn test_rollback_status_display() {
        assert_eq!(RollbackStatus::Pending.to_string(), "pending");
        assert_eq!(RollbackStatus::InProgress.to_string(), "in_progress");
        assert_eq!(RollbackStatus::Completed.to_string(), "completed");
        assert_eq!(RollbackStatus::Failed.to_string(), "failed");
    }

    // ==================== Rollback Logic Tests ====================

    #[tokio::test]
    async fn test_rollback_to_previous_version() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let rollback = DeploymentRollback::new(db.clone());

        // Create environment
        let env = create_test_environment(&db, "staging").await;

        // Create successful deployment
        create_successful_deployment(&db, &env, "1.0.0").await;

        // Create failed deployment
        let _failed = create_failed_deployment(&db, &env, "1.1.0").await;

        // Rollback to previous version
        let request = RollbackRequest {
            environment: env.name.clone(),
            target_version: None,
            skip_validation: true,
            force: false,
        };

        let rollback_event = rollback.rollback(request).await.unwrap();

        assert_eq!(rollback_event.target_version, "1.0.0");
        assert_eq!(rollback_event.rollback_type, RollbackType::Previous);
        assert_eq!(rollback_event.status, RollbackStatus::Completed);
        assert!(rollback_event.notification_sent);
    }

    #[tokio::test]
    async fn test_rollback_to_specific_version() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let rollback = DeploymentRollback::new(db.clone());

        let env = create_test_environment(&db, "staging").await;

        // Create multiple successful deployments
        create_successful_deployment(&db, &env, "1.0.0").await;
        create_successful_deployment(&db, &env, "1.1.0").await;
        let _failed = create_failed_deployment(&db, &env, "1.2.0").await;

        // Rollback to specific version
        let request = RollbackRequest {
            environment: env.name.clone(),
            target_version: Some("1.0.0".to_string()),
            skip_validation: true,
            force: false,
        };

        let rollback_event = rollback.rollback(request).await.unwrap();

        assert_eq!(rollback_event.target_version, "1.0.0");
        assert_eq!(rollback_event.rollback_type, RollbackType::Specific);
        assert_eq!(rollback_event.status, RollbackStatus::Completed);
    }

    #[tokio::test]
    async fn test_rollback_blue_green_fast_switch() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let rollback = DeploymentRollback::new(db.clone());

        let env = create_test_environment(&db, "production").await;

        // Create successful blue-green deployment
        create_successful_deployment(&db, &env, "1.0.0").await;

        // Create failed blue-green deployment
        let _failed = create_failed_blue_green_deployment(&db, &env, "1.1.0").await;

        // Rollback should use fast blue-green switch
        let request = RollbackRequest {
            environment: env.name.clone(),
            target_version: None,
            skip_validation: true,
            force: false,
        };

        let rollback_event = rollback.rollback(request).await.unwrap();

        assert_eq!(rollback_event.rollback_type, RollbackType::BlueGreenSwitch);
        assert_eq!(rollback_event.status, RollbackStatus::Completed);
    }

    #[tokio::test]
    async fn test_rollback_requires_force_for_successful_deployment() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let rollback = DeploymentRollback::new(db.clone());

        let env = create_test_environment(&db, "staging").await;

        // Create successful deployments
        create_successful_deployment(&db, &env, "1.0.0").await;
        create_successful_deployment(&db, &env, "1.1.0").await;

        // Try rollback without force - should fail
        let request = RollbackRequest {
            environment: env.name.clone(),
            target_version: None,
            skip_validation: true,
            force: false,
        };

        let result = rollback.rollback(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Use --force"));
    }

    #[tokio::test]
    async fn test_rollback_with_force_on_successful_deployment() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let rollback = DeploymentRollback::new(db.clone());

        let env = create_test_environment(&db, "staging").await;

        create_successful_deployment(&db, &env, "1.0.0").await;
        create_successful_deployment(&db, &env, "1.1.0").await;

        // Rollback with force should work
        let request = RollbackRequest {
            environment: env.name.clone(),
            target_version: Some("1.0.0".to_string()),
            skip_validation: true,
            force: true,
        };

        let rollback_event = rollback.rollback(request).await.unwrap();
        assert_eq!(rollback_event.status, RollbackStatus::Completed);
    }

    #[tokio::test]
    async fn test_rollback_no_previous_deployment() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let rollback = DeploymentRollback::new(db.clone());

        let env = create_test_environment(&db, "staging").await;

        // Create only one failed deployment
        let _failed = create_failed_deployment(&db, &env, "1.0.0").await;

        let request = RollbackRequest {
            environment: env.name.clone(),
            target_version: None,
            skip_validation: true,
            force: false,
        };

        let result = rollback.rollback(request).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No previous successful deployment"));
    }

    #[tokio::test]
    async fn test_list_rollback_events() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let rollback = DeploymentRollback::new(db.clone());

        let env = create_test_environment(&db, "staging").await;

        // Create deployments and rollbacks
        create_successful_deployment(&db, &env, "1.0.0").await;
        create_successful_deployment(&db, &env, "1.1.0").await;
        let _failed = create_failed_deployment(&db, &env, "1.2.0").await;

        let request = RollbackRequest {
            environment: env.name.clone(),
            target_version: None,
            skip_validation: true,
            force: false,
        };

        rollback.rollback(request).await.unwrap();

        let rollbacks = rollback.list_rollbacks(&env.name, None).await.unwrap();
        assert_eq!(rollbacks.len(), 1);
        assert_eq!(rollbacks[0].target_version, "1.1.0");
    }

    #[tokio::test]
    async fn test_get_rollback_event() {
        let db = Arc::new(Database::in_memory().await.unwrap());
        let rollback = DeploymentRollback::new(db.clone());

        let env = create_test_environment(&db, "staging").await;

        create_successful_deployment(&db, &env, "1.0.0").await;
        let _failed = create_failed_deployment(&db, &env, "1.1.0").await;

        let request = RollbackRequest {
            environment: env.name.clone(),
            target_version: None,
            skip_validation: true,
            force: false,
        };

        let rollback_event = rollback.rollback(request).await.unwrap();
        let fetched = rollback.get_rollback(rollback_event.id).await.unwrap();

        assert_eq!(fetched.id, rollback_event.id);
        assert_eq!(fetched.target_version, "1.0.0");
    }

    // ==================== Helper Functions ====================

    async fn create_test_environment(db: &Database, name: &str) -> crate::Environment {
        let create_env = CreateEnvironment {
            name: name.to_string(),
            env_type: EnvironmentType::Staging,
            url: Some(format!("https://{}.example.com", name)),
            provider: Some("docker".to_string()),
            config: HashMap::new(),
            secrets: HashMap::new(),
            requires_approval: false,
        };

        db.create_environment(create_env).await.unwrap()
    }

    async fn create_successful_deployment(
        db: &Database,
        env: &crate::Environment,
        version: &str,
    ) -> Deployment {
        let deployment = db
            .create_deployment(
                env.id,
                &env.name,
                version,
                &DeploymentProvider::Docker,
                None,
                1800,
                None,
            )
            .await
            .unwrap();

        db.update_deployment_status(deployment.id, DeploymentStatus::Completed, None)
            .await
            .unwrap()
    }

    async fn create_failed_deployment(
        db: &Database,
        env: &crate::Environment,
        version: &str,
    ) -> Deployment {
        let deployment = db
            .create_deployment(
                env.id,
                &env.name,
                version,
                &DeploymentProvider::Docker,
                None,
                1800,
                None,
            )
            .await
            .unwrap();

        db.update_deployment_status(
            deployment.id,
            DeploymentStatus::Failed,
            Some("Deployment failed"),
        )
        .await
        .unwrap()
    }

    async fn create_failed_blue_green_deployment(
        db: &Database,
        env: &crate::Environment,
        version: &str,
    ) -> Deployment {
        use crate::deployment_strategy::Environment as BlueGreenEnv;

        let strategy = DeploymentStrategy::blue_green(BlueGreenEnv::Blue);

        let deployment = db
            .create_deployment(
                env.id,
                &env.name,
                version,
                &DeploymentProvider::Docker,
                Some(&strategy),
                1800,
                None,
            )
            .await
            .unwrap();

        db.update_deployment_status(
            deployment.id,
            DeploymentStatus::Failed,
            Some("Blue-green deployment failed"),
        )
        .await
        .unwrap()
    }
}
