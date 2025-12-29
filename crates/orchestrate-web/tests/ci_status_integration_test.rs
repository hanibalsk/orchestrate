//! Integration tests for CI status event handling
//!
//! Tests check_run and check_suite webhook processing end-to-end.

use orchestrate_core::{AgentType, Database, WebhookEvent};
use orchestrate_web::event_handlers::handle_ci_status;
use std::sync::Arc;

/// Test check_run.completed with failure spawns issue-fixer
#[tokio::test]
async fn test_check_run_failure_processing() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    let payload = serde_json::json!({
        "action": "completed",
        "check_run": {
            "id": 12345,
            "name": "CI Build",
            "status": "completed",
            "conclusion": "failure",
            "head_sha": "abc123def456",
            "pull_requests": [{"number": 42}],
            "check_suite": {"head_branch": "feature/test"},
            "details_url": "https://github.com/owner/repo/runs/12345",
            "html_url": "https://github.com/owner/repo/runs/12345",
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "test-delivery-123".to_string(),
        "check_run".to_string(),
        payload,
    );

    // Process the event
    handle_ci_status(database.clone(), &event).await.unwrap();

    // Verify agent was created
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    assert_eq!(agent.agent_type, AgentType::IssueFixer);
    assert_eq!(agent.context.pr_number, Some(42));
    assert_eq!(
        agent.context.branch_name,
        Some("feature/test".to_string())
    );

    // Verify CI failure context
    let custom = &agent.context.custom;
    assert_eq!(
        custom.get("ci_check_name").unwrap().as_str().unwrap(),
        "CI Build"
    );
    assert_eq!(custom.get("ci_check_id").unwrap().as_i64().unwrap(), 12345);
    assert_eq!(
        custom.get("ci_conclusion").unwrap().as_str().unwrap(),
        "failure"
    );
    assert_eq!(
        custom.get("ci_head_sha").unwrap().as_str().unwrap(),
        "abc123def456"
    );
    assert!(custom.get("ci_details_url").is_some());
}

/// Test check_run with success conclusion doesn't spawn agent
#[tokio::test]
async fn test_check_run_success_not_processed() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    let payload = serde_json::json!({
        "action": "completed",
        "check_run": {
            "id": 67890,
            "name": "Tests",
            "status": "completed",
            "conclusion": "success",
            "head_sha": "success123",
            "pull_requests": [{"number": 50}],
            "check_suite": {"head_branch": "feature/ok"},
            "details_url": "https://github.com/owner/repo/runs/67890",
            "html_url": "https://github.com/owner/repo/runs/67890",
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "test-delivery-456".to_string(),
        "check_run".to_string(),
        payload,
    );

    handle_ci_status(database.clone(), &event).await.unwrap();

    // No agent should be created for successful check
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 0);
}

/// Test check_suite.completed with timed_out spawns issue-fixer
#[tokio::test]
async fn test_check_suite_timeout_processing() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    let payload = serde_json::json!({
        "action": "completed",
        "check_suite": {
            "id": 98765,
            "status": "completed",
            "conclusion": "timed_out",
            "head_sha": "timeout789",
            "head_branch": "feature/slow",
            "pull_requests": [{"number": 99}],
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "test-delivery-789".to_string(),
        "check_suite".to_string(),
        payload,
    );

    handle_ci_status(database.clone(), &event).await.unwrap();

    // Verify agent was created
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    assert_eq!(agent.agent_type, AgentType::IssueFixer);
    assert_eq!(agent.context.pr_number, Some(99));
    assert!(agent.task.contains("timed_out"));

    let custom = &agent.context.custom;
    assert_eq!(custom.get("ci_suite_id").unwrap().as_i64().unwrap(), 98765);
    assert_eq!(
        custom.get("ci_conclusion").unwrap().as_str().unwrap(),
        "timed_out"
    );
}

/// Test CI failure on PR with existing shepherd links agents
#[tokio::test]
async fn test_ci_failure_links_to_shepherd() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    // First, create a pr-shepherd agent
    let shepherd_context = orchestrate_core::AgentContext {
        pr_number: Some(75),
        branch_name: Some("feature/with-ci".to_string()),
        ..Default::default()
    };
    let shepherd = orchestrate_core::Agent::new(
        AgentType::PrShepherd,
        "Shepherd PR #75".to_string(),
    )
    .with_context(shepherd_context);
    database.insert_agent(&shepherd).await.unwrap();

    // Now trigger CI failure
    let payload = serde_json::json!({
        "action": "completed",
        "check_run": {
            "id": 11111,
            "name": "Integration Tests",
            "status": "completed",
            "conclusion": "failure",
            "head_sha": "link123",
            "pull_requests": [{"number": 75}],
            "check_suite": {"head_branch": "feature/with-ci"},
            "details_url": "https://github.com/owner/repo/runs/11111",
            "html_url": "https://github.com/owner/repo/runs/11111",
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "test-delivery-link".to_string(),
        "check_run".to_string(),
        payload,
    );

    handle_ci_status(database.clone(), &event).await.unwrap();

    // Should have both agents
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 2);

    // Find the issue-fixer
    let fixer = agents
        .iter()
        .find(|a| a.agent_type == AgentType::IssueFixer)
        .expect("Should have issue-fixer");

    // Verify link to shepherd
    let custom = &fixer.context.custom;
    assert_eq!(
        custom.get("shepherd_agent_id").unwrap().as_str().unwrap(),
        shepherd.id.to_string()
    );
}

/// Test duplicate CI failures don't spawn multiple agents
#[tokio::test]
async fn test_ci_failure_duplicate_prevention() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    let payload = serde_json::json!({
        "action": "completed",
        "check_run": {
            "id": 22222,
            "name": "Build",
            "status": "completed",
            "conclusion": "failure",
            "head_sha": "dup456",
            "pull_requests": [{"number": 88}],
            "check_suite": {"head_branch": "feature/dup"},
            "details_url": "https://github.com/owner/repo/runs/22222",
            "html_url": "https://github.com/owner/repo/runs/22222",
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event1 = WebhookEvent::new(
        "test-delivery-dup1".to_string(),
        "check_run".to_string(),
        payload.clone(),
    );

    // Process first event
    handle_ci_status(database.clone(), &event1).await.unwrap();

    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    // Process same event again (duplicate delivery)
    let event2 = WebhookEvent::new(
        "test-delivery-dup2".to_string(),
        "check_run".to_string(),
        payload,
    );

    handle_ci_status(database.clone(), &event2).await.unwrap();

    // Should still have only one agent
    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);
}

/// Test CI failure without PR creates agent without PR context
#[tokio::test]
async fn test_ci_failure_without_pr() {
    let database = Arc::new(Database::in_memory().await.unwrap());

    let payload = serde_json::json!({
        "action": "completed",
        "check_run": {
            "id": 33333,
            "name": "Deploy",
            "status": "completed",
            "conclusion": "failure",
            "head_sha": "main789",
            "pull_requests": [],
            "check_suite": {"head_branch": "main"},
            "details_url": "https://github.com/owner/repo/runs/33333",
            "html_url": "https://github.com/owner/repo/runs/33333",
        },
        "repository": {
            "full_name": "owner/repo"
        }
    })
    .to_string();

    let event = WebhookEvent::new(
        "test-delivery-no-pr".to_string(),
        "check_run".to_string(),
        payload,
    );

    handle_ci_status(database.clone(), &event).await.unwrap();

    let agents = database.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);

    let agent = &agents[0];
    assert_eq!(agent.context.pr_number, None);
    assert_eq!(agent.context.branch_name, Some("main".to_string()));
    assert!(!agent.task.contains("PR #"));
}
