//! Environment configuration and secrets management
//!
//! This module provides environment management for deployments with support for:
//! - Environment types (dev/staging/production)
//! - Environment-specific variables
//! - Encrypted secrets storage
//! - Environment connectivity validation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Environment type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvironmentType {
    Development,
    Staging,
    Production,
}

impl std::fmt::Display for EnvironmentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Development => write!(f, "development"),
            Self::Staging => write!(f, "staging"),
            Self::Production => write!(f, "production"),
        }
    }
}

impl std::str::FromStr for EnvironmentType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "development" | "dev" => Ok(Self::Development),
            "staging" | "stage" => Ok(Self::Staging),
            "production" | "prod" => Ok(Self::Production),
            _ => Err(crate::Error::InvalidEnvironmentType(s.to_string())),
        }
    }
}

/// Environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Environment {
    pub id: i64,
    pub name: String,
    pub env_type: EnvironmentType,
    pub url: Option<String>,
    pub provider: Option<String>,
    pub config: HashMap<String, serde_json::Value>,
    pub secrets: HashMap<String, String>,
    pub requires_approval: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Environment creation parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEnvironment {
    pub name: String,
    pub env_type: EnvironmentType,
    pub url: Option<String>,
    pub provider: Option<String>,
    pub config: HashMap<String, serde_json::Value>,
    pub secrets: HashMap<String, String>,
    pub requires_approval: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_environment_type_display() {
        assert_eq!(EnvironmentType::Development.to_string(), "development");
        assert_eq!(EnvironmentType::Staging.to_string(), "staging");
        assert_eq!(EnvironmentType::Production.to_string(), "production");
    }

    #[test]
    fn test_environment_type_from_str() {
        assert_eq!(
            "development".parse::<EnvironmentType>().unwrap(),
            EnvironmentType::Development
        );
        assert_eq!(
            "dev".parse::<EnvironmentType>().unwrap(),
            EnvironmentType::Development
        );
        assert_eq!(
            "staging".parse::<EnvironmentType>().unwrap(),
            EnvironmentType::Staging
        );
        assert_eq!(
            "stage".parse::<EnvironmentType>().unwrap(),
            EnvironmentType::Staging
        );
        assert_eq!(
            "production".parse::<EnvironmentType>().unwrap(),
            EnvironmentType::Production
        );
        assert_eq!(
            "prod".parse::<EnvironmentType>().unwrap(),
            EnvironmentType::Production
        );

        assert!("invalid".parse::<EnvironmentType>().is_err());
    }

    #[test]
    fn test_environment_type_case_insensitive() {
        assert_eq!(
            "PRODUCTION".parse::<EnvironmentType>().unwrap(),
            EnvironmentType::Production
        );
        assert_eq!(
            "Staging".parse::<EnvironmentType>().unwrap(),
            EnvironmentType::Staging
        );
    }
}
