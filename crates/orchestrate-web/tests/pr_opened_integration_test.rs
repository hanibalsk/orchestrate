//! Integration test for PR opened webhook event handling
//!
//! This test verifies the complete flow:
//! 1. Event is queued in database
//! 2. Processor picks up event
//! 3. Handler creates pr-shepherd agent

use orchestrate_core::{AgentType, Database, WebhookEvent};
use std::sync::Arc;

#[tokio::test]
async fn test_pr_opened_event_processing() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create PR opened payload
    let payload = serde_json::json!({
        "action": "opened",
        "number": 123,
        "pull_request": {
            "number": 123,
            "title": "Add new feature",
            "head": {
                "ref": "feature/new-feature",
                "repo": {
                    "fork": false
                }
            }
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    // Queue the webhook event directly
    let event = WebhookEvent::new(
        "test-pr-opened-123".to_string(),
        "pull_request".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Verify event was queued
    let queued_event = database
        .get_webhook_event_by_delivery_id("test-pr-opened-123")
        .await
        .unwrap();
    assert!(queued_event.is_some());
    let queued_event = queued_event.unwrap();
    assert_eq!(queued_event.event_type, "pull_request");
    assert_eq!(
        queued_event.status,
        orchestrate_core::WebhookEventStatus::Pending
    );

    // Process the event using the handler directly
    orchestrate_web::event_handlers::handle_pr_opened(database.clone(), &queued_event)
        .await
        .unwrap();

    // Verify agent was created
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    assert_eq!(agent.agent_type, AgentType::PrShepherd);
    assert_eq!(agent.context.pr_number, Some(123));
    assert_eq!(
        agent.context.branch_name,
        Some("feature/new-feature".to_string())
    );
    assert!(agent.task.contains("123"));
    assert!(agent.task.contains("feature/new-feature"));

    // Verify custom context
    let custom = &agent.context.custom;
    assert_eq!(
        custom.get("repository").unwrap().as_str().unwrap(),
        "test-org/test-repo"
    );
    assert_eq!(
        custom.get("event_delivery_id").unwrap().as_str().unwrap(),
        "test-pr-opened-123"
    );
}

#[tokio::test]
async fn test_pr_from_fork_security() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create PR opened payload from fork (security risk)
    let payload = serde_json::json!({
        "action": "opened",
        "number": 456,
        "pull_request": {
            "number": 456,
            "title": "Malicious PR",
            "head": {
                "ref": "feature/hack",
                "repo": {
                    "fork": true  // This is from a fork!
                }
            }
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    // Queue the webhook event
    let event = WebhookEvent::new(
        "test-fork-pr".to_string(),
        "pull_request".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process the event
    orchestrate_web::event_handlers::handle_pr_opened(database.clone(), &event)
        .await
        .unwrap();

    // Verify NO agent was created (security measure)
    let agents = database.list_agents().await.unwrap();
    assert_eq!(
        agents.len(),
        0,
        "No agents should be created for fork PRs"
    );
}
