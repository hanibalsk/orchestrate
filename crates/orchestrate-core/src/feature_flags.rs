//! Feature Flags Management
//!
//! Provides local feature flag management with support for:
//! - Creating and managing feature flags
//! - Enabling/disabling flags globally or per environment
//! - Gradual rollout via percentage-based targeting
//! - Integration hooks for external providers (LaunchDarkly, Unleash, etc.)

use crate::{Database, Error, Result};
use serde::{Deserialize, Serialize};
use sqlx::Row;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum FlagStatus {
    Enabled,
    Disabled,
    Conditional,
}

impl std::fmt::Display for FlagStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlagStatus::Enabled => write!(f, "enabled"),
            FlagStatus::Disabled => write!(f, "disabled"),
            FlagStatus::Conditional => write!(f, "conditional"),
        }
    }
}

impl std::str::FromStr for FlagStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "enabled" => Ok(FlagStatus::Enabled),
            "disabled" => Ok(FlagStatus::Disabled),
            "conditional" => Ok(FlagStatus::Conditional),
            _ => Err(Error::Other(format!("Invalid flag status: {}", s))),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlag {
    pub id: Option<i64>,
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub status: FlagStatus,
    pub rollout_percentage: i32, // 0-100
    pub environment: Option<String>, // None means global
    pub metadata: Option<String>, // JSON metadata for external integrations
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFeatureFlag {
    pub key: String,
    pub name: String,
    pub description: Option<String>,
    pub status: FlagStatus,
    pub rollout_percentage: Option<i32>,
    pub environment: Option<String>,
    pub metadata: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateFeatureFlag {
    pub status: Option<FlagStatus>,
    pub rollout_percentage: Option<i32>,
    pub metadata: Option<String>,
}

// Feature flags service functionality is now in the Database struct
// See database.rs for the implementation

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> Database {
        Database::in_memory().await.unwrap()
    }

    #[tokio::test]
    async fn test_create_feature_flag() {
        let db = setup_test_db().await;

        let flag = CreateFeatureFlag {
            key: "test-feature".to_string(),
            name: "Test Feature".to_string(),
            description: Some("A test feature flag".to_string()),
            status: FlagStatus::Enabled,
            rollout_percentage: Some(100),
            environment: None,
            metadata: None,
        };

        let created = db.create_feature_flag(flag).await.unwrap();

        assert_eq!(created.key, "test-feature");
        assert_eq!(created.name, "Test Feature");
        assert_eq!(created.status, FlagStatus::Enabled);
        assert_eq!(created.rollout_percentage, 100);
        assert!(created.id.is_some());
    }

    #[tokio::test]
    async fn test_create_flag_invalid_percentage() {
        let db = setup_test_db().await;

        let flag = CreateFeatureFlag {
            key: "test-feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            status: FlagStatus::Enabled,
            rollout_percentage: Some(150), // Invalid
            environment: None,
            metadata: None,
        };

        let result = db.create_feature_flag(flag).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("must be between 0 and 100"));
    }

    #[tokio::test]
    async fn test_get_feature_flag() {
        let db = setup_test_db().await;

        let flag = CreateFeatureFlag {
            key: "test-feature".to_string(),
            name: "Test Feature".to_string(),
            description: None,
            status: FlagStatus::Disabled,
            rollout_percentage: Some(50),
            environment: None,
            metadata: None,
        };

        db.create_feature_flag(flag).await.unwrap();

        let retrieved = db.get_feature_flag("test-feature", None).await.unwrap();
        assert_eq!(retrieved.key, "test-feature");
        assert_eq!(retrieved.status, FlagStatus::Disabled);
        assert_eq!(retrieved.rollout_percentage, 50);
    }

    #[tokio::test]
    async fn test_get_nonexistent_flag() {
        let db = setup_test_db().await;

        let result = db.get_feature_flag("nonexistent", None).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("not found"));
    }

    #[tokio::test]
    async fn test_list_feature_flags() {
        let db = setup_test_db().await;

        // Create multiple flags
        db
            .create_feature_flag(CreateFeatureFlag {
                key: "feature-1".to_string(),
                name: "Feature 1".to_string(),
                description: None,
                status: FlagStatus::Enabled,
                rollout_percentage: Some(100),
                environment: None,
                metadata: None,
            })
            .await
            .unwrap();

        db
            .create_feature_flag(CreateFeatureFlag {
                key: "feature-2".to_string(),
                name: "Feature 2".to_string(),
                description: None,
                status: FlagStatus::Disabled,
                rollout_percentage: Some(0),
                environment: Some("staging".to_string()),
                metadata: None,
            })
            .await
            .unwrap();

        let flags = db.list_feature_flags(None).await.unwrap();
        assert_eq!(flags.len(), 2);
    }

    #[tokio::test]
    async fn test_list_flags_by_environment() {
        let db = setup_test_db().await;

        // Create global flag
        db
            .create_feature_flag(CreateFeatureFlag {
                key: "global-feature".to_string(),
                name: "Global Feature".to_string(),
                description: None,
                status: FlagStatus::Enabled,
                rollout_percentage: Some(100),
                environment: None,
                metadata: None,
            })
            .await
            .unwrap();

        // Create staging flag
        db
            .create_feature_flag(CreateFeatureFlag {
                key: "staging-feature".to_string(),
                name: "Staging Feature".to_string(),
                description: None,
                status: FlagStatus::Enabled,
                rollout_percentage: Some(100),
                environment: Some("staging".to_string()),
                metadata: None,
            })
            .await
            .unwrap();

        // List staging flags (should include global + staging-specific)
        let flags = db.list_feature_flags(Some("staging")).await.unwrap();
        assert_eq!(flags.len(), 2);

        // List all flags
        let all_flags = db.list_feature_flags(None).await.unwrap();
        assert_eq!(all_flags.len(), 2);
    }

    #[tokio::test]
    async fn test_enable_flag() {
        let db = setup_test_db().await;

        db
            .create_feature_flag(CreateFeatureFlag {
                key: "test-feature".to_string(),
                name: "Test Feature".to_string(),
                description: None,
                status: FlagStatus::Disabled,
                rollout_percentage: Some(0),
                environment: None,
                metadata: None,
            })
            .await
            .unwrap();

        let enabled = db.enable_feature_flag("test-feature", None).await.unwrap();
        assert_eq!(enabled.status, FlagStatus::Enabled);
    }

    #[tokio::test]
    async fn test_disable_flag() {
        let db = setup_test_db().await;

        db
            .create_feature_flag(CreateFeatureFlag {
                key: "test-feature".to_string(),
                name: "Test Feature".to_string(),
                description: None,
                status: FlagStatus::Enabled,
                rollout_percentage: Some(100),
                environment: None,
                metadata: None,
            })
            .await
            .unwrap();

        let disabled = db.disable_feature_flag("test-feature", None).await.unwrap();
        assert_eq!(disabled.status, FlagStatus::Disabled);
    }

    #[tokio::test]
    async fn test_delete_flag() {
        let db = setup_test_db().await;

        db
            .create_feature_flag(CreateFeatureFlag {
                key: "test-feature".to_string(),
                name: "Test Feature".to_string(),
                description: None,
                status: FlagStatus::Enabled,
                rollout_percentage: Some(100),
                environment: None,
                metadata: None,
            })
            .await
            .unwrap();

        db.delete_feature_flag("test-feature", None).await.unwrap();

        let result = db.get_feature_flag("test-feature", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_flag() {
        let db = setup_test_db().await;

        let result = db.delete_feature_flag("nonexistent", None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_set_rollout_percentage() {
        let db = setup_test_db().await;

        db
            .create_feature_flag(CreateFeatureFlag {
                key: "test-feature".to_string(),
                name: "Test Feature".to_string(),
                description: None,
                status: FlagStatus::Enabled,
                rollout_percentage: Some(100),
                environment: None,
                metadata: None,
            })
            .await
            .unwrap();

        let updated = db
            .set_feature_flag_rollout("test-feature", None, 50)
            .await
            .unwrap();

        assert_eq!(updated.status, FlagStatus::Conditional);
        assert_eq!(updated.rollout_percentage, 50);
    }

    #[tokio::test]
    async fn test_environment_specific_flags() {
        let db = setup_test_db().await;

        // Create global flag
        db
            .create_feature_flag(CreateFeatureFlag {
                key: "feature".to_string(),
                name: "Feature".to_string(),
                description: None,
                status: FlagStatus::Disabled,
                rollout_percentage: Some(0),
                environment: None,
                metadata: None,
            })
            .await
            .unwrap();

        // Create production override
        db
            .create_feature_flag(CreateFeatureFlag {
                key: "feature".to_string(),
                name: "Feature".to_string(),
                description: None,
                status: FlagStatus::Enabled,
                rollout_percentage: Some(100),
                environment: Some("production".to_string()),
                metadata: None,
            })
            .await
            .unwrap();

        // Global should be disabled
        let global = db.get_feature_flag("feature", None).await.unwrap();
        assert_eq!(global.status, FlagStatus::Disabled);

        // Production should be enabled
        let prod = db
            .get_feature_flag("feature", Some("production"))
            .await
            .unwrap();
        assert_eq!(prod.status, FlagStatus::Enabled);
    }

    #[tokio::test]
    async fn test_update_flag_metadata() {
        let db = setup_test_db().await;

        db
            .create_feature_flag(CreateFeatureFlag {
                key: "test-feature".to_string(),
                name: "Test Feature".to_string(),
                description: None,
                status: FlagStatus::Enabled,
                rollout_percentage: Some(100),
                environment: None,
                metadata: None,
            })
            .await
            .unwrap();

        let metadata = r#"{"launchdarkly_key": "ld-test-feature"}"#;
        let updated = db
            .update_feature_flag(
                "test-feature",
                None,
                UpdateFeatureFlag {
                    status: None,
                    rollout_percentage: None,
                    metadata: Some(metadata.to_string()),
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.metadata.as_deref(), Some(metadata));
    }
}
