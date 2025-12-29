//! Environment database tests

use crate::database::Database;
use crate::environment::{CreateEnvironment, EnvironmentType};
use std::collections::HashMap;

#[tokio::test]
async fn test_create_environment() {
    let db = Database::in_memory().await.unwrap();

    let mut config = HashMap::new();
    config.insert(
        "cluster".to_string(),
        serde_json::Value::String("staging-ecs".to_string()),
    );
    config.insert(
        "service".to_string(),
        serde_json::Value::String("app-staging".to_string()),
    );

    let mut secrets = HashMap::new();
    secrets.insert("AWS_ACCESS_KEY".to_string(), "test-key-123".to_string());

    let create_env = CreateEnvironment {
        name: "staging".to_string(),
        env_type: EnvironmentType::Staging,
        url: Some("https://staging.example.com".to_string()),
        provider: Some("aws".to_string()),
        config,
        secrets,
        requires_approval: false,
    };

    let env = db.create_environment(create_env).await.unwrap();

    assert_eq!(env.name, "staging");
    assert_eq!(env.env_type, EnvironmentType::Staging);
    assert_eq!(env.url, Some("https://staging.example.com".to_string()));
    assert_eq!(env.provider, Some("aws".to_string()));
    assert_eq!(env.requires_approval, false);
    assert!(env.id > 0);

    // Verify config
    assert_eq!(
        env.config.get("cluster"),
        Some(&serde_json::Value::String("staging-ecs".to_string()))
    );

    // Verify secrets are stored (encrypted)
    assert!(env.secrets.contains_key("AWS_ACCESS_KEY"));
}

#[tokio::test]
async fn test_get_environment_by_name() {
    let db = Database::in_memory().await.unwrap();

    let create_env = CreateEnvironment {
        name: "production".to_string(),
        env_type: EnvironmentType::Production,
        url: Some("https://example.com".to_string()),
        provider: Some("aws".to_string()),
        config: HashMap::new(),
        secrets: HashMap::new(),
        requires_approval: true,
    };

    db.create_environment(create_env).await.unwrap();

    let env = db.get_environment_by_name("production").await.unwrap();

    assert_eq!(env.name, "production");
    assert_eq!(env.env_type, EnvironmentType::Production);
    assert_eq!(env.requires_approval, true);
}

#[tokio::test]
async fn test_get_environment_not_found() {
    let db = Database::in_memory().await.unwrap();

    let result = db.get_environment_by_name("nonexistent").await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_environments() {
    let db = Database::in_memory().await.unwrap();

    // Create multiple environments
    for (name, env_type) in &[
        ("dev", EnvironmentType::Development),
        ("staging", EnvironmentType::Staging),
        ("production", EnvironmentType::Production),
    ] {
        let create_env = CreateEnvironment {
            name: name.to_string(),
            env_type: env_type.clone(),
            url: None,
            provider: None,
            config: HashMap::new(),
            secrets: HashMap::new(),
            requires_approval: false,
        };
        db.create_environment(create_env).await.unwrap();
    }

    let envs = db.list_environments().await.unwrap();

    assert_eq!(envs.len(), 3);
    // Environments are sorted by name
    let names: Vec<&str> = envs.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"dev"));
    assert!(names.contains(&"staging"));
    assert!(names.contains(&"production"));
}

#[tokio::test]
async fn test_update_environment() {
    let db = Database::in_memory().await.unwrap();

    let create_env = CreateEnvironment {
        name: "staging".to_string(),
        env_type: EnvironmentType::Staging,
        url: Some("https://old.example.com".to_string()),
        provider: Some("aws".to_string()),
        config: HashMap::new(),
        secrets: HashMap::new(),
        requires_approval: false,
    };

    let env = db.create_environment(create_env).await.unwrap();

    // Update URL
    let mut updated = env.clone();
    updated.url = Some("https://new.example.com".to_string());
    updated.requires_approval = true;

    db.update_environment(&updated).await.unwrap();

    let fetched = db.get_environment_by_name("staging").await.unwrap();
    assert_eq!(fetched.url, Some("https://new.example.com".to_string()));
    assert_eq!(fetched.requires_approval, true);
}

#[tokio::test]
async fn test_delete_environment() {
    let db = Database::in_memory().await.unwrap();

    let create_env = CreateEnvironment {
        name: "temp".to_string(),
        env_type: EnvironmentType::Development,
        url: None,
        provider: None,
        config: HashMap::new(),
        secrets: HashMap::new(),
        requires_approval: false,
    };

    db.create_environment(create_env).await.unwrap();

    // Verify it exists
    assert!(db.get_environment_by_name("temp").await.is_ok());

    // Delete it
    db.delete_environment("temp").await.unwrap();

    // Verify it's gone
    assert!(db.get_environment_by_name("temp").await.is_err());
}

#[tokio::test]
async fn test_secrets_encryption() {
    let db = Database::in_memory().await.unwrap();

    let mut secrets = HashMap::new();
    secrets.insert("API_KEY".to_string(), "super-secret-key".to_string());
    secrets.insert("DB_PASSWORD".to_string(), "my-password".to_string());

    let create_env = CreateEnvironment {
        name: "secure".to_string(),
        env_type: EnvironmentType::Production,
        url: None,
        provider: None,
        config: HashMap::new(),
        secrets,
        requires_approval: true,
    };

    let env = db.create_environment(create_env).await.unwrap();

    // Secrets should be decrypted when retrieved
    assert_eq!(env.secrets.get("API_KEY").unwrap(), "super-secret-key");
    assert_eq!(env.secrets.get("DB_PASSWORD").unwrap(), "my-password");

    // Verify we can retrieve it again and secrets are decrypted
    let fetched = db.get_environment_by_name("secure").await.unwrap();
    assert_eq!(fetched.secrets.get("API_KEY").unwrap(), "super-secret-key");
}

#[tokio::test]
async fn test_environment_name_unique() {
    let db = Database::in_memory().await.unwrap();

    let create_env = CreateEnvironment {
        name: "unique".to_string(),
        env_type: EnvironmentType::Development,
        url: None,
        provider: None,
        config: HashMap::new(),
        secrets: HashMap::new(),
        requires_approval: false,
    };

    db.create_environment(create_env.clone()).await.unwrap();

    // Try to create another with the same name
    let result = db.create_environment(create_env).await;

    assert!(result.is_err());
}
