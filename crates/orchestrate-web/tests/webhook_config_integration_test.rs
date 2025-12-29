//! Integration tests for webhook configuration
//!
//! Tests that webhook configuration properly filters and routes events.

use orchestrate_core::{AgentType, Database, WebhookConfig, WebhookEvent};
use orchestrate_web::webhook_processor::{WebhookProcessor, WebhookProcessorConfig};
use std::sync::Arc;

#[tokio::test]
async fn test_config_filters_by_branch() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Configuration that only handles PRs to main or develop
    let yaml = r#"
webhooks:
  events:
    pull_request.opened:
      agent: pr_shepherd
      filter:
        base_branch: [main, develop]
"#;
    let config = WebhookConfig::from_yaml_str(yaml).unwrap();

    // Create PR event to main branch (should be handled)
    let payload_main = serde_json::json!({
        "action": "opened",
        "pull_request": {
            "number": 1,
            "base": {
                "ref": "main"
            },
            "head": {
                "ref": "feature/test",
                "repo": {
                    "fork": false
                }
            }
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event_main = WebhookEvent::new(
        "delivery-main".to_string(),
        "pull_request".to_string(),
        payload_main,
    );
    database.insert_webhook_event(&event_main).await.unwrap();

    // Create PR event to feature branch (should be skipped)
    let payload_feature = serde_json::json!({
        "action": "opened",
        "pull_request": {
            "number": 2,
            "base": {
                "ref": "staging"
            },
            "head": {
                "ref": "feature/other",
                "repo": {
                    "fork": false
                }
            }
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event_feature = WebhookEvent::new(
        "delivery-feature".to_string(),
        "pull_request".to_string(),
        payload_feature,
    );
    database
        .insert_webhook_event(&event_feature)
        .await
        .unwrap();

    // Process with configuration
    let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default())
        .with_config(config);
    processor.process_batch().await.unwrap();

    // Only one agent should be created (for main branch)
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].agent_type, AgentType::PrShepherd);
    assert_eq!(agents[0].context.pr_number, Some(1));
}

#[tokio::test]
async fn test_config_filters_by_fork() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Configuration that skips forks
    let yaml = r#"
webhooks:
  events:
    pull_request.opened:
      agent: pr_shepherd
      filter:
        skip_forks: true
"#;
    let config = WebhookConfig::from_yaml_str(yaml).unwrap();

    // Create PR from fork (should be skipped)
    let payload_fork = serde_json::json!({
        "action": "opened",
        "pull_request": {
            "number": 10,
            "base": {
                "ref": "main"
            },
            "head": {
                "ref": "feature/fork",
                "repo": {
                    "fork": true
                }
            }
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event_fork = WebhookEvent::new(
        "delivery-fork".to_string(),
        "pull_request".to_string(),
        payload_fork,
    );
    database.insert_webhook_event(&event_fork).await.unwrap();

    // Create PR from same repo (should be handled)
    let payload_same = serde_json::json!({
        "action": "opened",
        "pull_request": {
            "number": 11,
            "base": {
                "ref": "main"
            },
            "head": {
                "ref": "feature/same",
                "repo": {
                    "fork": false
                }
            }
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event_same = WebhookEvent::new(
        "delivery-same".to_string(),
        "pull_request".to_string(),
        payload_same,
    );
    database.insert_webhook_event(&event_same).await.unwrap();

    // Process with configuration
    let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default())
        .with_config(config);
    processor.process_batch().await.unwrap();

    // Only one agent should be created (non-fork)
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].context.pr_number, Some(11));
}

#[tokio::test]
async fn test_config_filters_by_conclusion() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Configuration that only handles failures and timeouts
    let yaml = r#"
webhooks:
  events:
    check_run.completed:
      agent: issue_fixer
      filter:
        conclusion: [failure, timed_out]
"#;
    let config = WebhookConfig::from_yaml_str(yaml).unwrap();

    // Create check run with failure (should be handled)
    let payload_failure = serde_json::json!({
        "action": "completed",
        "check_run": {
            "id": 123,
            "name": "test",
            "conclusion": "failure",
            "head_sha": "abc123",
            "pull_requests": [{"number": 20}],
            "check_suite": {},
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event_failure = WebhookEvent::new(
        "delivery-failure".to_string(),
        "check_run".to_string(),
        payload_failure,
    );
    database
        .insert_webhook_event(&event_failure)
        .await
        .unwrap();

    // Create check run with success (should be skipped)
    let payload_success = serde_json::json!({
        "action": "completed",
        "check_run": {
            "id": 124,
            "name": "lint",
            "conclusion": "success",
            "head_sha": "def456",
            "pull_requests": [{"number": 21}],
            "check_suite": {},
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event_success = WebhookEvent::new(
        "delivery-success".to_string(),
        "check_run".to_string(),
        payload_success,
    );
    database
        .insert_webhook_event(&event_success)
        .await
        .unwrap();

    // Process with configuration
    let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default())
        .with_config(config);
    processor.process_batch().await.unwrap();

    // Only one agent should be created (for failure)
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].agent_type, AgentType::IssueFixer);
}

#[tokio::test]
async fn test_config_skips_unconfigured_events() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Configuration that only handles PR opened events
    let yaml = r#"
webhooks:
  events:
    pull_request.opened:
      agent: pr_shepherd
"#;
    let config = WebhookConfig::from_yaml_str(yaml).unwrap();

    // Create push event (not configured)
    let payload_push = serde_json::json!({
        "ref": "refs/heads/main",
        "before": "abc123",
        "after": "def456",
        "repository": {
            "full_name": "owner/repo"
        },
        "commits": []
    })
    .to_string();

    let event_push = WebhookEvent::new(
        "delivery-push".to_string(),
        "push".to_string(),
        payload_push,
    );
    database.insert_webhook_event(&event_push).await.unwrap();

    // Process with configuration
    let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default())
        .with_config(config);
    processor.process_batch().await.unwrap();

    // No agents should be created
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 0);
}

#[tokio::test]
async fn test_config_allows_pr_review_events() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Configuration that explicitly enables PR review handling
    let yaml = r#"
webhooks:
  events:
    pull_request_review.submitted:
      agent: issue_fixer
      filter:
        # No specific filters
"#;
    let config = WebhookConfig::from_yaml_str(yaml).unwrap();

    // Create review event with changes requested
    let payload = serde_json::json!({
        "action": "submitted",
        "review": {
            "state": "changes_requested",
            "body": "Please fix",
        },
        "pull_request": {
            "number": 30,
            "head": {
                "ref": "feature/test",
            },
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "delivery-review".to_string(),
        "pull_request_review".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process with configuration
    let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default())
        .with_config(config);
    processor.process_batch().await.unwrap();

    // Agent should be created (agent type from handler, not config)
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);
    // Note: Currently, event handlers determine agent type, not config
    // The config's agent field is informational/documentation
    assert_eq!(agents[0].agent_type, AgentType::IssueFixer);
}

#[tokio::test]
async fn test_no_config_uses_defaults() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create PR event
    let payload = serde_json::json!({
        "action": "opened",
        "pull_request": {
            "number": 40,
            "base": {
                "ref": "main"
            },
            "head": {
                "ref": "feature/no-config",
                "repo": {
                    "fork": false
                }
            }
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "delivery-no-config".to_string(),
        "pull_request".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process WITHOUT configuration (should use defaults)
    let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default());
    processor.process_batch().await.unwrap();

    // Agent should still be created using default handler
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].agent_type, AgentType::PrShepherd);
}
