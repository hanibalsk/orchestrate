//! Deployment Automation Module
//!
//! This module provides deployment orchestration capabilities including:
//! - Multi-environment deployments (dev, staging, production)
//! - Deployment strategies (rolling, blue-green, canary)
//! - Pre/post deployment validation
//! - Rollback capabilities
//! - Release management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

/// Deployment provider types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentProvider {
    Docker,
    AwsEcs,
    AwsLambda,
    Kubernetes,
    Vercel,
    Netlify,
    Railway,
    Custom(String),
}

impl FromStr for DeploymentProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "docker" => Ok(Self::Docker),
            "aws_ecs" | "ecs" => Ok(Self::AwsEcs),
            "aws_lambda" | "lambda" => Ok(Self::AwsLambda),
            "kubernetes" | "k8s" => Ok(Self::Kubernetes),
            "vercel" => Ok(Self::Vercel),
            "netlify" => Ok(Self::Netlify),
            "railway" => Ok(Self::Railway),
            other => Ok(Self::Custom(other.to_string())),
        }
    }
}

impl std::fmt::Display for DeploymentProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Docker => write!(f, "docker"),
            Self::AwsEcs => write!(f, "aws_ecs"),
            Self::AwsLambda => write!(f, "aws_lambda"),
            Self::Kubernetes => write!(f, "kubernetes"),
            Self::Vercel => write!(f, "vercel"),
            Self::Netlify => write!(f, "netlify"),
            Self::Railway => write!(f, "railway"),
            Self::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Deployment strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStrategy {
    /// Gradual replacement of instances
    Rolling,
    /// Switch between two identical environments
    BlueGreen,
    /// Route percentage of traffic to new version
    Canary,
    /// Stop old, start new (for dev)
    Recreate,
}

impl FromStr for DeploymentStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rolling" => Ok(Self::Rolling),
            "blue_green" | "blue-green" | "bluegreen" => Ok(Self::BlueGreen),
            "canary" => Ok(Self::Canary),
            "recreate" => Ok(Self::Recreate),
            _ => Err(format!("Unknown deployment strategy: {}", s)),
        }
    }
}

impl std::fmt::Display for DeploymentStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rolling => write!(f, "rolling"),
            Self::BlueGreen => write!(f, "blue_green"),
            Self::Canary => write!(f, "canary"),
            Self::Recreate => write!(f, "recreate"),
        }
    }
}

/// Environment type classification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EnvironmentType {
    Development,
    Staging,
    Production,
    Preview,
}

impl FromStr for EnvironmentType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "staging" | "stage" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            "preview" => Ok(Self::Preview),
            _ => Err(format!("Unknown environment type: {}", s)),
        }
    }
}

/// Deployment environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub env_type: EnvironmentType,
    pub url: Option<String>,
    pub provider: DeploymentProvider,
    pub config: HashMap<String, String>,
    pub secrets: HashMap<String, String>,
    pub requires_approval: bool,
    pub default_strategy: DeploymentStrategy,
    pub health_check_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Environment {
    pub fn new(
        name: impl Into<String>,
        env_type: EnvironmentType,
        provider: DeploymentProvider,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            env_type,
            url: None,
            provider,
            config: HashMap::new(),
            secrets: HashMap::new(),
            requires_approval: false,
            default_strategy: DeploymentStrategy::Rolling,
            health_check_url: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn with_approval_required(mut self, required: bool) -> Self {
        self.requires_approval = required;
        self
    }

    pub fn with_strategy(mut self, strategy: DeploymentStrategy) -> Self {
        self.default_strategy = strategy;
        self
    }

    pub fn is_production(&self) -> bool {
        matches!(self.env_type, EnvironmentType::Production)
    }
}

/// Deployment status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentStatus {
    Pending,
    Validating,
    InProgress,
    Verifying,
    Succeeded,
    Failed,
    RolledBack,
    Cancelled,
}

impl std::fmt::Display for DeploymentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Validating => write!(f, "validating"),
            Self::InProgress => write!(f, "in_progress"),
            Self::Verifying => write!(f, "verifying"),
            Self::Succeeded => write!(f, "succeeded"),
            Self::Failed => write!(f, "failed"),
            Self::RolledBack => write!(f, "rolled_back"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// A deployment record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deployment {
    pub id: String,
    pub environment_id: String,
    pub environment_name: String,
    pub version: String,
    pub previous_version: Option<String>,
    pub strategy: DeploymentStrategy,
    pub status: DeploymentStatus,
    pub initiated_by: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_seconds: Option<f64>,
    pub artifact_url: Option<String>,
    pub commit_sha: Option<String>,
    pub error_message: Option<String>,
    pub logs: Vec<DeploymentLogEntry>,
    pub metrics: DeploymentMetrics,
}

impl Deployment {
    pub fn new(
        environment_id: impl Into<String>,
        environment_name: impl Into<String>,
        version: impl Into<String>,
        strategy: DeploymentStrategy,
        initiated_by: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            environment_id: environment_id.into(),
            environment_name: environment_name.into(),
            version: version.into(),
            previous_version: None,
            strategy,
            status: DeploymentStatus::Pending,
            initiated_by: initiated_by.into(),
            started_at: Utc::now(),
            completed_at: None,
            duration_seconds: None,
            artifact_url: None,
            commit_sha: None,
            error_message: None,
            logs: Vec::new(),
            metrics: DeploymentMetrics::default(),
        }
    }

    pub fn start(&mut self) {
        self.status = DeploymentStatus::InProgress;
        self.add_log(DeploymentLogLevel::Info, "Deployment started");
    }

    pub fn complete(&mut self, success: bool, error: Option<String>) {
        let now = Utc::now();
        self.completed_at = Some(now);
        self.duration_seconds = Some((now - self.started_at).num_milliseconds() as f64 / 1000.0);

        if success {
            self.status = DeploymentStatus::Succeeded;
            self.add_log(DeploymentLogLevel::Info, "Deployment completed successfully");
        } else {
            self.status = DeploymentStatus::Failed;
            self.error_message = error.clone();
            self.add_log(
                DeploymentLogLevel::Error,
                &format!("Deployment failed: {}", error.unwrap_or_default()),
            );
        }
    }

    pub fn rollback(&mut self) {
        self.status = DeploymentStatus::RolledBack;
        self.add_log(DeploymentLogLevel::Warning, "Deployment rolled back");
    }

    pub fn add_log(&mut self, level: DeploymentLogLevel, message: &str) {
        self.logs.push(DeploymentLogEntry {
            timestamp: Utc::now(),
            level,
            message: message.to_string(),
        });
    }

    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            DeploymentStatus::Succeeded
                | DeploymentStatus::Failed
                | DeploymentStatus::RolledBack
                | DeploymentStatus::Cancelled
        )
    }
}

/// Deployment log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: DeploymentLogLevel,
    pub message: String,
}

/// Deployment log level
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentLogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Deployment metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeploymentMetrics {
    pub instances_deployed: u32,
    pub instances_healthy: u32,
    pub instances_failed: u32,
    pub traffic_percentage: f64,
    pub error_rate_before: Option<f64>,
    pub error_rate_after: Option<f64>,
    pub response_time_before_ms: Option<f64>,
    pub response_time_after_ms: Option<f64>,
}

/// Pre-deployment validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreDeploymentValidation {
    pub environment_id: String,
    pub version: String,
    pub validated_at: DateTime<Utc>,
    pub passed: bool,
    pub checks: Vec<ValidationCheck>,
    pub blocking_issues: Vec<String>,
    pub warnings: Vec<String>,
}

impl PreDeploymentValidation {
    pub fn new(environment_id: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            environment_id: environment_id.into(),
            version: version.into(),
            validated_at: Utc::now(),
            passed: false,
            checks: Vec::new(),
            blocking_issues: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn add_check(&mut self, check: ValidationCheck) {
        if !check.passed && check.is_blocking {
            self.blocking_issues.push(check.message.clone());
        } else if !check.passed {
            self.warnings.push(check.message.clone());
        }
        self.checks.push(check);
    }

    pub fn finalize(&mut self) {
        self.passed = self.blocking_issues.is_empty();
    }

    pub fn summary(&self) -> String {
        let passed = self.checks.iter().filter(|c| c.passed).count();
        let total = self.checks.len();
        format!(
            "Validation {}: {}/{} checks passed, {} blocking issues",
            if self.passed { "PASSED" } else { "FAILED" },
            passed,
            total,
            self.blocking_issues.len()
        )
    }
}

/// Individual validation check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCheck {
    pub name: String,
    pub check_type: ValidationCheckType,
    pub passed: bool,
    pub message: String,
    pub is_blocking: bool,
    pub duration_ms: Option<u64>,
}

/// Types of validation checks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationCheckType {
    TestsPassing,
    SecurityScan,
    ArtifactExists,
    ArtifactSigned,
    EnvironmentReachable,
    NoConflictingDeployment,
    DeploymentWindow,
    RequiredApproval,
    Custom(String),
}

/// Post-deployment verification result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDeploymentVerification {
    pub deployment_id: String,
    pub verified_at: DateTime<Utc>,
    pub passed: bool,
    pub checks: Vec<VerificationCheck>,
    pub should_rollback: bool,
    pub rollback_reason: Option<String>,
}

impl PostDeploymentVerification {
    pub fn new(deployment_id: impl Into<String>) -> Self {
        Self {
            deployment_id: deployment_id.into(),
            verified_at: Utc::now(),
            passed: false,
            checks: Vec::new(),
            should_rollback: false,
            rollback_reason: None,
        }
    }

    pub fn add_check(&mut self, check: VerificationCheck) {
        if !check.passed && check.triggers_rollback {
            self.should_rollback = true;
            if self.rollback_reason.is_none() {
                self.rollback_reason = Some(check.message.clone());
            }
        }
        self.checks.push(check);
    }

    pub fn finalize(&mut self) {
        self.passed = !self.should_rollback;
    }
}

/// Individual verification check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationCheck {
    pub name: String,
    pub check_type: VerificationCheckType,
    pub passed: bool,
    pub message: String,
    pub triggers_rollback: bool,
    pub metric_value: Option<f64>,
}

/// Types of verification checks
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerificationCheckType {
    SmokeTest,
    HealthCheck,
    VersionVerification,
    LogErrors,
    ErrorRate,
    ResponseTime,
    Custom(String),
}

/// Release information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Release {
    pub id: String,
    pub version: String,
    pub previous_version: Option<String>,
    pub release_type: ReleaseType,
    pub status: ReleaseStatus,
    pub branch: Option<String>,
    pub commit_sha: Option<String>,
    pub tag: Option<String>,
    pub changelog: Option<String>,
    pub release_notes: Option<String>,
    pub assets: Vec<ReleaseAsset>,
    pub created_at: DateTime<Utc>,
    pub published_at: Option<DateTime<Utc>>,
    pub created_by: String,
}

impl Release {
    pub fn new(version: impl Into<String>, release_type: ReleaseType, created_by: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            version: version.into(),
            previous_version: None,
            release_type,
            status: ReleaseStatus::Draft,
            branch: None,
            commit_sha: None,
            tag: None,
            changelog: None,
            release_notes: None,
            assets: Vec::new(),
            created_at: Utc::now(),
            published_at: None,
            created_by: created_by.into(),
        }
    }

    pub fn publish(&mut self) {
        self.status = ReleaseStatus::Published;
        self.published_at = Some(Utc::now());
    }
}

/// Release type (semver)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseType {
    Major,
    Minor,
    Patch,
    PreRelease,
}

impl FromStr for ReleaseType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "major" => Ok(Self::Major),
            "minor" => Ok(Self::Minor),
            "patch" => Ok(Self::Patch),
            "prerelease" | "pre-release" | "pre" => Ok(Self::PreRelease),
            _ => Err(format!("Unknown release type: {}", s)),
        }
    }
}

/// Release status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStatus {
    Draft,
    Prepared,
    Published,
    Cancelled,
}

/// Release asset
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    pub name: String,
    pub url: String,
    pub size_bytes: u64,
    pub content_type: String,
}

/// Canary deployment stage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryStage {
    pub stage_number: u32,
    pub traffic_percentage: f64,
    pub duration_minutes: u32,
    pub status: CanaryStageStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub metrics: CanaryMetrics,
}

/// Canary stage status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CanaryStageStatus {
    Pending,
    InProgress,
    Passed,
    Failed,
    Skipped,
}

/// Canary deployment metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CanaryMetrics {
    pub error_rate: Option<f64>,
    pub response_time_p50_ms: Option<f64>,
    pub response_time_p99_ms: Option<f64>,
    pub success_rate: Option<f64>,
    pub baseline_error_rate: Option<f64>,
    pub is_anomaly: bool,
}

/// Feature flag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub environment_overrides: HashMap<String, bool>,
    pub rollout_percentage: Option<f64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl FeatureFlag {
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            enabled: false,
            environment_overrides: HashMap::new(),
            rollout_percentage: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn is_enabled_for_environment(&self, env: &str) -> bool {
        self.environment_overrides.get(env).copied().unwrap_or(self.enabled)
    }
}

/// Deployment diff showing what will change
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentDiff {
    pub environment: String,
    pub current_version: Option<String>,
    pub target_version: String,
    pub changes: Vec<ChangeItem>,
    pub files_changed: u32,
    pub additions: u32,
    pub deletions: u32,
}

/// Individual change item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeItem {
    pub change_type: DeploymentChangeType,
    pub path: String,
    pub description: String,
}

/// Type of deployment change
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeploymentChangeType {
    Added,
    Modified,
    Deleted,
    ConfigChange,
    DependencyUpdate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deployment_provider_from_str() {
        assert_eq!(
            DeploymentProvider::from_str("docker").unwrap(),
            DeploymentProvider::Docker
        );
        assert_eq!(
            DeploymentProvider::from_str("k8s").unwrap(),
            DeploymentProvider::Kubernetes
        );
        assert_eq!(
            DeploymentProvider::from_str("ecs").unwrap(),
            DeploymentProvider::AwsEcs
        );
        if let DeploymentProvider::Custom(name) = DeploymentProvider::from_str("custom-provider").unwrap() {
            assert_eq!(name, "custom-provider");
        } else {
            panic!("Expected Custom variant");
        }
    }

    #[test]
    fn test_deployment_strategy_from_str() {
        assert_eq!(
            DeploymentStrategy::from_str("rolling").unwrap(),
            DeploymentStrategy::Rolling
        );
        assert_eq!(
            DeploymentStrategy::from_str("blue-green").unwrap(),
            DeploymentStrategy::BlueGreen
        );
        assert_eq!(
            DeploymentStrategy::from_str("canary").unwrap(),
            DeploymentStrategy::Canary
        );
    }

    #[test]
    fn test_environment_creation() {
        let env = Environment::new("staging", EnvironmentType::Staging, DeploymentProvider::AwsEcs)
            .with_url("https://staging.example.com")
            .with_approval_required(false);

        assert_eq!(env.name, "staging");
        assert_eq!(env.env_type, EnvironmentType::Staging);
        assert_eq!(env.url, Some("https://staging.example.com".to_string()));
        assert!(!env.requires_approval);
        assert!(!env.is_production());
    }

    #[test]
    fn test_production_environment() {
        let env = Environment::new("prod", EnvironmentType::Production, DeploymentProvider::Kubernetes)
            .with_approval_required(true);

        assert!(env.is_production());
        assert!(env.requires_approval);
    }

    #[test]
    fn test_deployment_lifecycle() {
        let mut deployment = Deployment::new(
            "env-123",
            "staging",
            "1.0.0",
            DeploymentStrategy::Rolling,
            "user@example.com",
        );

        assert_eq!(deployment.status, DeploymentStatus::Pending);
        assert!(!deployment.is_complete());

        deployment.start();
        assert_eq!(deployment.status, DeploymentStatus::InProgress);
        assert!(!deployment.is_complete());

        deployment.complete(true, None);
        assert_eq!(deployment.status, DeploymentStatus::Succeeded);
        assert!(deployment.is_complete());
        assert!(deployment.completed_at.is_some());
        assert!(deployment.duration_seconds.is_some());
    }

    #[test]
    fn test_deployment_failure_and_rollback() {
        let mut deployment = Deployment::new(
            "env-123",
            "production",
            "2.0.0",
            DeploymentStrategy::BlueGreen,
            "deployer",
        );

        deployment.start();
        deployment.complete(false, Some("Health check failed".to_string()));

        assert_eq!(deployment.status, DeploymentStatus::Failed);
        assert_eq!(
            deployment.error_message,
            Some("Health check failed".to_string())
        );

        deployment.rollback();
        assert_eq!(deployment.status, DeploymentStatus::RolledBack);
    }

    #[test]
    fn test_pre_deployment_validation() {
        let mut validation = PreDeploymentValidation::new("env-123", "1.0.0");

        validation.add_check(ValidationCheck {
            name: "Tests Passing".to_string(),
            check_type: ValidationCheckType::TestsPassing,
            passed: true,
            message: "All tests pass".to_string(),
            is_blocking: true,
            duration_ms: Some(1500),
        });

        validation.add_check(ValidationCheck {
            name: "Security Scan".to_string(),
            check_type: ValidationCheckType::SecurityScan,
            passed: false,
            message: "Medium severity vulnerability found".to_string(),
            is_blocking: false,
            duration_ms: Some(5000),
        });

        validation.finalize();

        assert!(validation.passed);
        assert!(validation.blocking_issues.is_empty());
        assert_eq!(validation.warnings.len(), 1);
    }

    #[test]
    fn test_validation_with_blocking_issue() {
        let mut validation = PreDeploymentValidation::new("env-123", "1.0.0");

        validation.add_check(ValidationCheck {
            name: "Artifact Exists".to_string(),
            check_type: ValidationCheckType::ArtifactExists,
            passed: false,
            message: "Artifact not found".to_string(),
            is_blocking: true,
            duration_ms: None,
        });

        validation.finalize();

        assert!(!validation.passed);
        assert_eq!(validation.blocking_issues.len(), 1);
        assert!(validation.summary().contains("FAILED"));
    }

    #[test]
    fn test_post_deployment_verification() {
        let mut verification = PostDeploymentVerification::new("deploy-123");

        verification.add_check(VerificationCheck {
            name: "Health Check".to_string(),
            check_type: VerificationCheckType::HealthCheck,
            passed: true,
            message: "All endpoints healthy".to_string(),
            triggers_rollback: true,
            metric_value: None,
        });

        verification.add_check(VerificationCheck {
            name: "Error Rate".to_string(),
            check_type: VerificationCheckType::ErrorRate,
            passed: false,
            message: "Error rate 5% exceeds threshold 1%".to_string(),
            triggers_rollback: true,
            metric_value: Some(5.0),
        });

        verification.finalize();

        assert!(!verification.passed);
        assert!(verification.should_rollback);
        assert_eq!(
            verification.rollback_reason,
            Some("Error rate 5% exceeds threshold 1%".to_string())
        );
    }

    #[test]
    fn test_release_creation() {
        let mut release = Release::new("1.2.0", ReleaseType::Minor, "developer");

        assert_eq!(release.version, "1.2.0");
        assert_eq!(release.release_type, ReleaseType::Minor);
        assert_eq!(release.status, ReleaseStatus::Draft);
        assert!(release.published_at.is_none());

        release.publish();

        assert_eq!(release.status, ReleaseStatus::Published);
        assert!(release.published_at.is_some());
    }

    #[test]
    fn test_feature_flag() {
        let mut flag = FeatureFlag::new("dark-mode");
        flag.enabled = true;
        flag.environment_overrides.insert("staging".to_string(), true);
        flag.environment_overrides.insert("production".to_string(), false);

        assert!(flag.is_enabled_for_environment("staging"));
        assert!(!flag.is_enabled_for_environment("production"));
        assert!(flag.is_enabled_for_environment("development")); // Falls back to default
    }

    #[test]
    fn test_canary_metrics() {
        let mut metrics = CanaryMetrics::default();
        metrics.error_rate = Some(0.5);
        metrics.baseline_error_rate = Some(0.3);
        metrics.is_anomaly = true;

        assert!(metrics.is_anomaly);
        assert!(metrics.error_rate.unwrap() > metrics.baseline_error_rate.unwrap());
    }
}
