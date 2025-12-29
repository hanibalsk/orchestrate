//! Integration test for issue opened webhook event handling
//!
//! This test verifies the complete flow:
//! 1. Event is queued in database
//! 2. Processor picks up event
//! 3. Handler creates issue-triager agent for new issues

use orchestrate_core::{AgentType, Database, WebhookEvent};
use std::sync::Arc;

#[tokio::test]
async fn test_issue_opened_processing() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create issue opened payload
    let payload = serde_json::json!({
        "action": "opened",
        "issue": {
            "number": 500,
            "title": "Bug: Application fails to start",
            "body": "The application crashes immediately when launched. Error log:\n```\nSegmentation fault\n```",
            "state": "open",
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    // Queue the webhook event directly
    let event = WebhookEvent::new(
        "test-issue-500".to_string(),
        "issues".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Verify event was queued
    let queued_event = database
        .get_webhook_event_by_delivery_id("test-issue-500")
        .await
        .unwrap();
    assert!(queued_event.is_some());
    let queued_event = queued_event.unwrap();
    assert_eq!(queued_event.event_type, "issues");
    assert_eq!(
        queued_event.status,
        orchestrate_core::WebhookEventStatus::Pending
    );

    // Process the event using the handler directly
    orchestrate_web::event_handlers::handle_issue_opened(database.clone(), &queued_event)
        .await
        .unwrap();

    // Verify agent was created
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    assert_eq!(agent.agent_type, AgentType::IssueTriager);
    assert!(agent.task.contains("500"));
    assert!(agent.task.contains("Bug: Application fails"));

    // Verify custom context
    let custom = &agent.context.custom;
    assert_eq!(
        custom.get("repository").unwrap().as_str().unwrap(),
        "test-org/test-repo"
    );
    assert_eq!(
        custom.get("event_delivery_id").unwrap().as_str().unwrap(),
        "test-issue-500"
    );
    assert_eq!(custom.get("issue_number").unwrap().as_i64().unwrap(), 500);
    assert_eq!(
        custom.get("issue_title").unwrap().as_str().unwrap(),
        "Bug: Application fails to start"
    );
    assert!(custom
        .get("issue_body")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("Segmentation fault"));
}

#[tokio::test]
async fn test_issue_closed_not_processed() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create issue closed payload
    let payload = serde_json::json!({
        "action": "closed",
        "issue": {
            "number": 501,
            "title": "Closed issue",
            "body": "This issue was fixed",
            "state": "closed",
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    // Queue the webhook event
    let event = WebhookEvent::new(
        "test-issue-closed".to_string(),
        "issues".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process the event
    orchestrate_web::event_handlers::handle_issue_opened(database.clone(), &event)
        .await
        .unwrap();

    // Verify NO agent was created for closed action
    let agents = database.list_agents().await.unwrap();
    assert_eq!(
        agents.len(),
        0,
        "No agents should be created for closed issues"
    );
}

#[tokio::test]
async fn test_issue_with_labels_processing() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create issue with labels
    let payload = serde_json::json!({
        "action": "opened",
        "issue": {
            "number": 502,
            "title": "Feature: Add dark mode",
            "body": "Users have requested a dark mode option",
            "state": "open",
            "labels": [
                {"name": "enhancement"},
                {"name": "priority:medium"},
                {"name": "good first issue"}
            ]
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "test-issue-with-labels".to_string(),
        "issues".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process the event
    orchestrate_web::event_handlers::handle_issue_opened(database.clone(), &event)
        .await
        .unwrap();

    // Verify agent was created with labels in context
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    let custom = &agent.context.custom;
    let labels = custom.get("issue_labels").unwrap().as_array().unwrap();
    assert_eq!(labels.len(), 3);
    assert_eq!(labels[0].as_str().unwrap(), "enhancement");
    assert_eq!(labels[1].as_str().unwrap(), "priority:medium");
    assert_eq!(labels[2].as_str().unwrap(), "good first issue");
}

#[tokio::test]
async fn test_issue_with_assignees_processing() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create issue with assignees
    let payload = serde_json::json!({
        "action": "opened",
        "issue": {
            "number": 503,
            "title": "Bug: Memory leak in worker threads",
            "body": "Memory usage grows unbounded",
            "state": "open",
            "assignees": [
                {"login": "developer1"},
                {"login": "developer2"}
            ]
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "test-issue-with-assignees".to_string(),
        "issues".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process the event
    orchestrate_web::event_handlers::handle_issue_opened(database.clone(), &event)
        .await
        .unwrap();

    // Verify agent was created with assignees in context
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    let custom = &agent.context.custom;
    let assignees = custom.get("issue_assignees").unwrap().as_array().unwrap();
    assert_eq!(assignees.len(), 2);
    assert_eq!(assignees[0].as_str().unwrap(), "developer1");
    assert_eq!(assignees[1].as_str().unwrap(), "developer2");
}

#[tokio::test]
async fn test_issue_with_empty_body_processing() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create issue with empty body
    let payload = serde_json::json!({
        "action": "opened",
        "issue": {
            "number": 504,
            "title": "Issue with no description",
            "body": "",
            "state": "open",
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "test-issue-empty-body".to_string(),
        "issues".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process the event
    orchestrate_web::event_handlers::handle_issue_opened(database.clone(), &event)
        .await
        .unwrap();

    // Verify agent was created even with empty body
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    let custom = &agent.context.custom;
    assert_eq!(custom.get("issue_body").unwrap().as_str().unwrap(), "");
}
