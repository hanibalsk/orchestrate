//! Deployment strategies for different deployment scenarios
//!
//! This module provides various deployment strategies:
//! - Rolling: Gradual replacement of instances
//! - Blue-Green: Switch between two identical environments
//! - Canary: Route percentage of traffic to new version
//! - Recreate: Stop old, start new (for dev)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Deployment strategy types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyType {
    /// Gradual replacement of instances
    Rolling,
    /// Switch between two identical environments
    BlueGreen,
    /// Route percentage of traffic to new version
    Canary,
    /// Stop old, start new (for dev)
    Recreate,
}

impl std::fmt::Display for StrategyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rolling => write!(f, "rolling"),
            Self::BlueGreen => write!(f, "blue_green"),
            Self::Canary => write!(f, "canary"),
            Self::Recreate => write!(f, "recreate"),
        }
    }
}

impl std::str::FromStr for StrategyType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rolling" => Ok(Self::Rolling),
            "blue_green" | "bluegreen" => Ok(Self::BlueGreen),
            "canary" => Ok(Self::Canary),
            "recreate" => Ok(Self::Recreate),
            _ => Err(crate::Error::Other(format!(
                "Invalid strategy type: {}",
                s
            ))),
        }
    }
}

/// Rolling deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollingConfig {
    /// Number or percentage of instances to update at once
    pub batch_size: BatchSize,
    /// Wait time between batches (in seconds)
    pub batch_wait_seconds: u32,
    /// Maximum number of unhealthy instances allowed
    pub max_unhealthy: u32,
}

/// Blue-Green deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlueGreenConfig {
    /// Current active environment (blue or green)
    pub active_environment: Environment,
    /// Wait time before switching (in seconds)
    pub switch_wait_seconds: u32,
    /// Keep old environment for rollback
    pub keep_old_environment: bool,
    /// Duration to keep old environment (in seconds)
    pub old_environment_ttl_seconds: u32,
}

/// Canary deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanaryConfig {
    /// Traffic percentages for each step
    pub traffic_steps: Vec<u8>,
    /// Wait time between steps (in seconds)
    pub step_wait_seconds: u32,
    /// Metrics to monitor for anomalies
    pub monitored_metrics: Vec<String>,
    /// Error rate threshold for automatic rollback (percentage)
    pub error_threshold_percent: f64,
}

/// Recreate deployment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecreateConfig {
    /// Allow downtime during deployment
    pub allow_downtime: bool,
}

/// Batch size for rolling deployments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BatchSize {
    /// Absolute number of instances
    Count(u32),
    /// Percentage of total instances
    Percent(u8),
}

/// Environment type for blue-green deployments
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Blue,
    Green,
}

impl Environment {
    /// Get the other environment
    pub fn other(&self) -> Self {
        match self {
            Self::Blue => Self::Green,
            Self::Green => Self::Blue,
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Blue => write!(f, "blue"),
            Self::Green => write!(f, "green"),
        }
    }
}

/// Health check configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Health check endpoint
    pub endpoint: String,
    /// Expected status code
    pub expected_status: u16,
    /// Timeout for health check (in seconds)
    pub timeout_seconds: u32,
    /// Number of retries before marking as unhealthy
    pub max_retries: u32,
    /// Wait time between retries (in seconds)
    pub retry_interval_seconds: u32,
    /// Custom headers for health check
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

/// Deployment strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentStrategy {
    /// Strategy type
    pub strategy_type: StrategyType,
    /// Rolling-specific configuration
    pub rolling: Option<RollingConfig>,
    /// Blue-Green-specific configuration
    pub blue_green: Option<BlueGreenConfig>,
    /// Canary-specific configuration
    pub canary: Option<CanaryConfig>,
    /// Recreate-specific configuration
    pub recreate: Option<RecreateConfig>,
    /// Health checks for this strategy
    pub health_checks: Vec<HealthCheck>,
}

impl DeploymentStrategy {
    /// Create a new rolling deployment strategy
    pub fn rolling(batch_size: BatchSize, batch_wait_seconds: u32) -> Self {
        Self {
            strategy_type: StrategyType::Rolling,
            rolling: Some(RollingConfig {
                batch_size,
                batch_wait_seconds,
                max_unhealthy: 0,
            }),
            blue_green: None,
            canary: None,
            recreate: None,
            health_checks: Vec::new(),
        }
    }

    /// Create a new blue-green deployment strategy
    pub fn blue_green(active_environment: Environment) -> Self {
        Self {
            strategy_type: StrategyType::BlueGreen,
            rolling: None,
            blue_green: Some(BlueGreenConfig {
                active_environment,
                switch_wait_seconds: 30,
                keep_old_environment: true,
                old_environment_ttl_seconds: 3600,
            }),
            canary: None,
            recreate: None,
            health_checks: Vec::new(),
        }
    }

    /// Create a new canary deployment strategy
    pub fn canary(traffic_steps: Vec<u8>) -> Self {
        Self {
            strategy_type: StrategyType::Canary,
            rolling: None,
            blue_green: None,
            canary: Some(CanaryConfig {
                traffic_steps,
                step_wait_seconds: 300,
                monitored_metrics: vec!["error_rate".to_string(), "latency".to_string()],
                error_threshold_percent: 5.0,
            }),
            recreate: None,
            health_checks: Vec::new(),
        }
    }

    /// Create a new recreate deployment strategy
    pub fn recreate() -> Self {
        Self {
            strategy_type: StrategyType::Recreate,
            rolling: None,
            blue_green: None,
            canary: None,
            recreate: Some(RecreateConfig {
                allow_downtime: true,
            }),
            health_checks: Vec::new(),
        }
    }

    /// Add a health check to the strategy
    pub fn with_health_check(mut self, health_check: HealthCheck) -> Self {
        self.health_checks.push(health_check);
        self
    }

    /// Validate the strategy configuration
    pub fn validate(&self) -> crate::Result<()> {
        match self.strategy_type {
            StrategyType::Rolling => {
                if self.rolling.is_none() {
                    return Err(crate::Error::Other(
                        "Rolling strategy requires rolling configuration".to_string(),
                    ));
                }
                let config = self.rolling.as_ref().unwrap();
                match &config.batch_size {
                    BatchSize::Count(n) if *n == 0 => {
                        return Err(crate::Error::Other(
                            "Batch size count must be greater than 0".to_string(),
                        ));
                    }
                    BatchSize::Percent(p) if *p == 0 || *p > 100 => {
                        return Err(crate::Error::Other(
                            "Batch size percent must be between 1 and 100".to_string(),
                        ));
                    }
                    _ => {}
                }
            }
            StrategyType::BlueGreen => {
                if self.blue_green.is_none() {
                    return Err(crate::Error::Other(
                        "Blue-Green strategy requires blue_green configuration".to_string(),
                    ));
                }
            }
            StrategyType::Canary => {
                if self.canary.is_none() {
                    return Err(crate::Error::Other(
                        "Canary strategy requires canary configuration".to_string(),
                    ));
                }
                let config = self.canary.as_ref().unwrap();
                if config.traffic_steps.is_empty() {
                    return Err(crate::Error::Other(
                        "Canary strategy requires at least one traffic step".to_string(),
                    ));
                }
                for step in &config.traffic_steps {
                    if *step > 100 {
                        return Err(crate::Error::Other(
                            "Traffic step percentage must be between 0 and 100".to_string(),
                        ));
                    }
                }
                if config.error_threshold_percent < 0.0 || config.error_threshold_percent > 100.0 {
                    return Err(crate::Error::Other(
                        "Error threshold must be between 0 and 100".to_string(),
                    ));
                }
            }
            StrategyType::Recreate => {
                if self.recreate.is_none() {
                    return Err(crate::Error::Other(
                        "Recreate strategy requires recreate configuration".to_string(),
                    ));
                }
            }
        }
        Ok(())
    }
}

impl Default for HealthCheck {
    fn default() -> Self {
        Self {
            endpoint: "/health".to_string(),
            expected_status: 200,
            timeout_seconds: 10,
            max_retries: 3,
            retry_interval_seconds: 5,
            headers: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== StrategyType Tests ====================

    #[test]
    fn test_strategy_type_display() {
        assert_eq!(StrategyType::Rolling.to_string(), "rolling");
        assert_eq!(StrategyType::BlueGreen.to_string(), "blue_green");
        assert_eq!(StrategyType::Canary.to_string(), "canary");
        assert_eq!(StrategyType::Recreate.to_string(), "recreate");
    }

    #[test]
    fn test_strategy_type_from_str() {
        assert_eq!(
            "rolling".parse::<StrategyType>().unwrap(),
            StrategyType::Rolling
        );
        assert_eq!(
            "blue_green".parse::<StrategyType>().unwrap(),
            StrategyType::BlueGreen
        );
        assert_eq!(
            "bluegreen".parse::<StrategyType>().unwrap(),
            StrategyType::BlueGreen
        );
        assert_eq!(
            "canary".parse::<StrategyType>().unwrap(),
            StrategyType::Canary
        );
        assert_eq!(
            "recreate".parse::<StrategyType>().unwrap(),
            StrategyType::Recreate
        );
        assert!("invalid".parse::<StrategyType>().is_err());
    }

    // ==================== Environment Tests ====================

    #[test]
    fn test_environment_other() {
        assert_eq!(Environment::Blue.other(), Environment::Green);
        assert_eq!(Environment::Green.other(), Environment::Blue);
    }

    #[test]
    fn test_environment_display() {
        assert_eq!(Environment::Blue.to_string(), "blue");
        assert_eq!(Environment::Green.to_string(), "green");
    }

    // ==================== Rolling Strategy Tests ====================

    #[test]
    fn test_create_rolling_strategy_with_count() {
        let strategy = DeploymentStrategy::rolling(BatchSize::Count(5), 30);
        assert_eq!(strategy.strategy_type, StrategyType::Rolling);
        assert!(strategy.rolling.is_some());
        assert!(strategy.blue_green.is_none());
        assert!(strategy.canary.is_none());
        assert!(strategy.recreate.is_none());

        let config = strategy.rolling.unwrap();
        assert!(matches!(config.batch_size, BatchSize::Count(5)));
        assert_eq!(config.batch_wait_seconds, 30);
        assert_eq!(config.max_unhealthy, 0);
    }

    #[test]
    fn test_create_rolling_strategy_with_percent() {
        let strategy = DeploymentStrategy::rolling(BatchSize::Percent(25), 60);
        let config = strategy.rolling.unwrap();
        assert!(matches!(config.batch_size, BatchSize::Percent(25)));
        assert_eq!(config.batch_wait_seconds, 60);
    }

    #[test]
    fn test_validate_rolling_strategy_success() {
        let strategy = DeploymentStrategy::rolling(BatchSize::Count(5), 30);
        assert!(strategy.validate().is_ok());

        let strategy = DeploymentStrategy::rolling(BatchSize::Percent(25), 30);
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_validate_rolling_strategy_zero_count() {
        let strategy = DeploymentStrategy::rolling(BatchSize::Count(0), 30);
        assert!(strategy.validate().is_err());
    }

    #[test]
    fn test_validate_rolling_strategy_invalid_percent() {
        let strategy = DeploymentStrategy::rolling(BatchSize::Percent(0), 30);
        assert!(strategy.validate().is_err());

        let strategy = DeploymentStrategy::rolling(BatchSize::Percent(101), 30);
        assert!(strategy.validate().is_err());
    }

    // ==================== Blue-Green Strategy Tests ====================

    #[test]
    fn test_create_blue_green_strategy() {
        let strategy = DeploymentStrategy::blue_green(Environment::Blue);
        assert_eq!(strategy.strategy_type, StrategyType::BlueGreen);
        assert!(strategy.blue_green.is_some());
        assert!(strategy.rolling.is_none());
        assert!(strategy.canary.is_none());
        assert!(strategy.recreate.is_none());

        let config = strategy.blue_green.unwrap();
        assert_eq!(config.active_environment, Environment::Blue);
        assert_eq!(config.switch_wait_seconds, 30);
        assert!(config.keep_old_environment);
        assert_eq!(config.old_environment_ttl_seconds, 3600);
    }

    #[test]
    fn test_validate_blue_green_strategy() {
        let strategy = DeploymentStrategy::blue_green(Environment::Blue);
        assert!(strategy.validate().is_ok());
    }

    // ==================== Canary Strategy Tests ====================

    #[test]
    fn test_create_canary_strategy() {
        let traffic_steps = vec![10, 25, 50, 100];
        let strategy = DeploymentStrategy::canary(traffic_steps.clone());
        assert_eq!(strategy.strategy_type, StrategyType::Canary);
        assert!(strategy.canary.is_some());
        assert!(strategy.rolling.is_none());
        assert!(strategy.blue_green.is_none());
        assert!(strategy.recreate.is_none());

        let config = strategy.canary.unwrap();
        assert_eq!(config.traffic_steps, traffic_steps);
        assert_eq!(config.step_wait_seconds, 300);
        assert_eq!(config.monitored_metrics.len(), 2);
        assert_eq!(config.error_threshold_percent, 5.0);
    }

    #[test]
    fn test_validate_canary_strategy_success() {
        let strategy = DeploymentStrategy::canary(vec![10, 25, 50, 100]);
        assert!(strategy.validate().is_ok());
    }

    #[test]
    fn test_validate_canary_strategy_empty_steps() {
        let strategy = DeploymentStrategy::canary(vec![]);
        assert!(strategy.validate().is_err());
    }

    #[test]
    fn test_validate_canary_strategy_invalid_step() {
        let strategy = DeploymentStrategy::canary(vec![10, 150]);
        assert!(strategy.validate().is_err());
    }

    // ==================== Recreate Strategy Tests ====================

    #[test]
    fn test_create_recreate_strategy() {
        let strategy = DeploymentStrategy::recreate();
        assert_eq!(strategy.strategy_type, StrategyType::Recreate);
        assert!(strategy.recreate.is_some());
        assert!(strategy.rolling.is_none());
        assert!(strategy.blue_green.is_none());
        assert!(strategy.canary.is_none());

        let config = strategy.recreate.unwrap();
        assert!(config.allow_downtime);
    }

    #[test]
    fn test_validate_recreate_strategy() {
        let strategy = DeploymentStrategy::recreate();
        assert!(strategy.validate().is_ok());
    }

    // ==================== Health Check Tests ====================

    #[test]
    fn test_health_check_default() {
        let health_check = HealthCheck::default();
        assert_eq!(health_check.endpoint, "/health");
        assert_eq!(health_check.expected_status, 200);
        assert_eq!(health_check.timeout_seconds, 10);
        assert_eq!(health_check.max_retries, 3);
        assert_eq!(health_check.retry_interval_seconds, 5);
        assert!(health_check.headers.is_empty());
    }

    #[test]
    fn test_strategy_with_health_check() {
        let health_check = HealthCheck {
            endpoint: "/api/health".to_string(),
            expected_status: 200,
            timeout_seconds: 5,
            max_retries: 3,
            retry_interval_seconds: 2,
            headers: HashMap::new(),
        };

        let strategy = DeploymentStrategy::rolling(BatchSize::Count(5), 30)
            .with_health_check(health_check.clone());

        assert_eq!(strategy.health_checks.len(), 1);
        assert_eq!(strategy.health_checks[0].endpoint, "/api/health");
    }

    #[test]
    fn test_strategy_with_multiple_health_checks() {
        let health_check1 = HealthCheck {
            endpoint: "/health".to_string(),
            ..Default::default()
        };

        let health_check2 = HealthCheck {
            endpoint: "/ready".to_string(),
            ..Default::default()
        };

        let strategy = DeploymentStrategy::rolling(BatchSize::Count(5), 30)
            .with_health_check(health_check1)
            .with_health_check(health_check2);

        assert_eq!(strategy.health_checks.len(), 2);
        assert_eq!(strategy.health_checks[0].endpoint, "/health");
        assert_eq!(strategy.health_checks[1].endpoint, "/ready");
    }

    // ==================== Serialization Tests ====================

    #[test]
    fn test_serialize_rolling_strategy() {
        let strategy = DeploymentStrategy::rolling(BatchSize::Count(5), 30);
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("\"strategy_type\":\"rolling\""));
    }

    #[test]
    fn test_deserialize_rolling_strategy() {
        let json = r#"{
            "strategy_type": "rolling",
            "rolling": {
                "batch_size": 5,
                "batch_wait_seconds": 30,
                "max_unhealthy": 0
            },
            "blue_green": null,
            "canary": null,
            "recreate": null,
            "health_checks": []
        }"#;

        let strategy: DeploymentStrategy = serde_json::from_str(json).unwrap();
        assert_eq!(strategy.strategy_type, StrategyType::Rolling);
        assert!(strategy.rolling.is_some());
    }

    #[test]
    fn test_serialize_canary_strategy() {
        let strategy = DeploymentStrategy::canary(vec![10, 25, 50, 100]);
        let json = serde_json::to_string(&strategy).unwrap();
        assert!(json.contains("\"strategy_type\":\"canary\""));
        assert!(json.contains("\"traffic_steps\":[10,25,50,100]"));
    }
}
