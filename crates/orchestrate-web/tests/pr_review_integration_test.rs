//! Integration test for PR review webhook event handling
//!
//! This test verifies the complete flow:
//! 1. Event is queued in database
//! 2. Processor picks up event
//! 3. Handler creates issue-fixer agent when changes are requested

use orchestrate_core::{Agent, AgentContext, AgentType, Database, WebhookEvent};
use std::sync::Arc;

#[tokio::test]
async fn test_pr_review_changes_requested_processing() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create PR review submitted payload
    let payload = serde_json::json!({
        "action": "submitted",
        "review": {
            "state": "changes_requested",
            "body": "Please fix:\n1. Add error handling\n2. Update tests",
        },
        "pull_request": {
            "number": 200,
            "head": {
                "ref": "feature/needs-fixes",
            }
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    // Queue the webhook event directly
    let event = WebhookEvent::new(
        "test-review-200".to_string(),
        "pull_request_review".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Verify event was queued
    let queued_event = database
        .get_webhook_event_by_delivery_id("test-review-200")
        .await
        .unwrap();
    assert!(queued_event.is_some());
    let queued_event = queued_event.unwrap();
    assert_eq!(queued_event.event_type, "pull_request_review");
    assert_eq!(
        queued_event.status,
        orchestrate_core::WebhookEventStatus::Pending
    );

    // Process the event using the handler directly
    orchestrate_web::event_handlers::handle_pr_review_submitted(database.clone(), &queued_event)
        .await
        .unwrap();

    // Verify agent was created
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    assert_eq!(agent.agent_type, AgentType::IssueFixer);
    assert_eq!(agent.context.pr_number, Some(200));
    assert_eq!(
        agent.context.branch_name,
        Some("feature/needs-fixes".to_string())
    );
    assert!(agent.task.contains("200"));
    assert!(agent.task.contains("feature/needs-fixes"));

    // Verify custom context
    let custom = &agent.context.custom;
    assert_eq!(
        custom.get("repository").unwrap().as_str().unwrap(),
        "test-org/test-repo"
    );
    assert_eq!(
        custom.get("event_delivery_id").unwrap().as_str().unwrap(),
        "test-review-200"
    );
    assert_eq!(
        custom.get("review_body").unwrap().as_str().unwrap(),
        "Please fix:\n1. Add error handling\n2. Update tests"
    );
}

#[tokio::test]
async fn test_pr_review_approved_not_processed() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create approved review payload
    let payload = serde_json::json!({
        "action": "submitted",
        "review": {
            "state": "approved",
            "body": "Looks good!",
        },
        "pull_request": {
            "number": 201,
            "head": {
                "ref": "feature/approved",
            }
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    // Queue the webhook event
    let event = WebhookEvent::new(
        "test-review-approved".to_string(),
        "pull_request_review".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process the event
    orchestrate_web::event_handlers::handle_pr_review_submitted(database.clone(), &event)
        .await
        .unwrap();

    // Verify NO agent was created for approved review
    let agents = database.list_agents().await.unwrap();
    assert_eq!(
        agents.len(),
        0,
        "No agents should be created for approved reviews"
    );
}

#[tokio::test]
async fn test_pr_review_links_to_shepherd() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // First create a pr-shepherd agent for PR #202
    let shepherd_context = AgentContext {
        pr_number: Some(202),
        branch_name: Some("feature/with-shepherd".to_string()),
        ..Default::default()
    };
    let shepherd = Agent::new(AgentType::PrShepherd, "Shepherd PR #202".to_string())
        .with_context(shepherd_context);
    database.insert_agent(&shepherd).await.unwrap();

    // Now create a review for the same PR
    let payload = serde_json::json!({
        "action": "submitted",
        "review": {
            "state": "changes_requested",
            "body": "Please update the documentation",
        },
        "pull_request": {
            "number": 202,
            "head": {
                "ref": "feature/with-shepherd",
            }
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "test-review-with-shepherd".to_string(),
        "pull_request_review".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process the event
    orchestrate_web::event_handlers::handle_pr_review_submitted(database.clone(), &event)
        .await
        .unwrap();

    // Should have both shepherd and issue-fixer
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 2);

    // Find the issue-fixer agent
    let fixer = agents
        .iter()
        .find(|a| a.agent_type == AgentType::IssueFixer)
        .expect("Should have issue-fixer agent");

    // Verify it has the shepherd_agent_id in context
    let custom = &fixer.context.custom;
    assert!(custom.get("shepherd_agent_id").is_some());
    assert_eq!(
        custom.get("shepherd_agent_id").unwrap().as_str().unwrap(),
        shepherd.id.to_string()
    );
}

#[tokio::test]
async fn test_pr_review_comment_only_not_processed() {
    // Setup database
    let database = Arc::new(Database::in_memory().await.unwrap());

    // Create comment-only review payload (not requesting changes)
    let payload = serde_json::json!({
        "action": "submitted",
        "review": {
            "state": "commented",
            "body": "Have you considered using a different approach?",
        },
        "pull_request": {
            "number": 203,
            "head": {
                "ref": "feature/commented",
            }
        },
        "repository": {
            "full_name": "test-org/test-repo"
        }
    })
    .to_string();

    // Queue the webhook event
    let event = WebhookEvent::new(
        "test-review-commented".to_string(),
        "pull_request_review".to_string(),
        payload,
    );
    database.insert_webhook_event(&event).await.unwrap();

    // Process the event
    orchestrate_web::event_handlers::handle_pr_review_submitted(database.clone(), &event)
        .await
        .unwrap();

    // Verify NO agent was created for comment-only review
    let agents = database.list_agents().await.unwrap();
    assert_eq!(
        agents.len(),
        0,
        "No agents should be created for comment-only reviews"
    );
}
