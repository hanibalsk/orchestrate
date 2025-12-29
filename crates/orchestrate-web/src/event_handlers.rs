//! Event handlers for different GitHub webhook event types
//!
//! This module processes specific webhook events and spawns appropriate agents.

use orchestrate_core::{Agent, AgentContext, AgentType, Database, Result, WebhookEvent};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Handle a pull_request.opened event
///
/// Spawns a pr-shepherd agent for the PR.
///
/// Returns Ok(()) if event was handled successfully, Err if processing should be retried.
pub async fn handle_pr_opened(
    database: Arc<Database>,
    event: &WebhookEvent,
) -> Result<()> {
    info!(
        delivery_id = %event.delivery_id,
        "Processing pull_request.opened event"
    );

    // Parse payload
    let payload: Value = serde_json::from_str(&event.payload)?;

    // Extract action - must be "opened"
    let action = payload
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing action field".to_string()))?;

    if action != "opened" {
        debug!(action = %action, "Skipping non-opened action");
        return Ok(());
    }

    // Extract PR data
    let pr = payload
        .get("pull_request")
        .ok_or_else(|| orchestrate_core::Error::Other("Missing pull_request field".to_string()))?;

    let pr_number = pr
        .get("number")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing PR number".to_string()))?;

    let branch_name = pr
        .get("head")
        .and_then(|h| h.get("ref"))
        .and_then(|r| r.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing branch name".to_string()))?
        .to_string();

    let repo_full_name = payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing repository name".to_string()))?
        .to_string();

    // Check if PR is from fork (security measure)
    let is_fork = pr
        .get("head")
        .and_then(|h| h.get("repo"))
        .and_then(|r| r.get("fork"))
        .and_then(|f| f.as_bool())
        .unwrap_or(false);

    if is_fork {
        warn!(
            pr_number = pr_number,
            "Skipping PR from fork for security reasons"
        );
        return Ok(());
    }

    info!(
        pr_number = pr_number,
        branch = %branch_name,
        repository = %repo_full_name,
        "Spawning pr-shepherd agent for PR"
    );

    // Create agent context
    let context = AgentContext {
        pr_number: Some(pr_number as i32),
        branch_name: Some(branch_name.clone()),
        custom: serde_json::json!({
            "repository": repo_full_name,
            "event_delivery_id": event.delivery_id,
        }),
        ..Default::default()
    };

    // Create pr-shepherd agent
    let agent = Agent::new(
        AgentType::PrShepherd,
        format!("Shepherd PR #{} on branch {}", pr_number, branch_name),
    )
    .with_context(context);

    // Save agent to database
    database.insert_agent(&agent).await?;

    info!(
        agent_id = %agent.id,
        pr_number = pr_number,
        "pr-shepherd agent created"
    );

    // TODO: Actually spawn the agent (call orchestrate CLI or spawn process)
    // TODO: Create worktree for the PR branch
    // TODO: Update PR with comment indicating orchestrate is watching

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestrate_core::AgentState;

    fn create_pr_opened_payload(pr_number: i64, branch: &str, is_fork: bool) -> String {
        serde_json::json!({
            "action": "opened",
            "number": 123,
            "pull_request": {
                "number": pr_number,
                "head": {
                    "ref": branch,
                    "repo": {
                        "fork": is_fork
                    }
                }
            },
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string()
    }

    #[tokio::test]
    async fn test_handle_pr_opened_creates_agent() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_pr_opened_payload(42, "feature/test", false);
        let event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            payload,
        );

        let result = handle_pr_opened(database.clone(), &event).await;
        assert!(result.is_ok());

        // Verify agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.agent_type, AgentType::PrShepherd);
        assert_eq!(agent.state, AgentState::Created);
        assert_eq!(agent.context.pr_number, Some(42));
        assert_eq!(agent.context.branch_name, Some("feature/test".to_string()));
        assert!(agent.task.contains("42"));
        assert!(agent.task.contains("feature/test"));
    }

    #[tokio::test]
    async fn test_handle_pr_opened_skips_fork() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_pr_opened_payload(42, "feature/test", true);
        let event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            payload,
        );

        let result = handle_pr_opened(database.clone(), &event).await;
        assert!(result.is_ok());

        // Verify no agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_pr_opened_skips_non_opened_action() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = serde_json::json!({
            "action": "synchronize",
            "pull_request": {
                "number": 42,
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

        let event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            payload,
        );

        let result = handle_pr_opened(database.clone(), &event).await;
        assert!(result.is_ok());

        // Verify no agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_pr_opened_missing_fields() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Missing pull_request field
        let payload = serde_json::json!({
            "action": "opened"
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            payload,
        );

        let result = handle_pr_opened(database.clone(), &event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_pr_opened_extracts_repository_info() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_pr_opened_payload(99, "fix/bug", false);
        let event = WebhookEvent::new(
            "delivery-456".to_string(),
            "pull_request".to_string(),
            payload,
        );

        handle_pr_opened(database.clone(), &event).await.unwrap();

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        let custom = &agent.context.custom;
        assert_eq!(custom.get("repository").unwrap().as_str().unwrap(), "owner/repo");
        assert_eq!(custom.get("event_delivery_id").unwrap().as_str().unwrap(), "delivery-456");
    }
}
