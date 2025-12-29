//! Event handlers for different GitHub webhook event types
//!
//! This module processes specific webhook events and spawns appropriate agents.

use orchestrate_core::{
    create_pr_worktree, Agent, AgentContext, AgentType, Database, Result, WebhookEvent,
};
use orchestrate_github::GitHubClient;
use serde_json::Value;
use std::env;
use std::sync::Arc;
use tracing::{debug, error, info, warn};

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

    // Create worktree for the PR branch
    let worktree_dir = env::var("WORKTREE_DIR").unwrap_or_else(|_| ".worktrees".to_string());

    let mut worktree = match create_pr_worktree(pr_number as i32, &branch_name, &worktree_dir) {
        Ok(wt) => wt,
        Err(e) => {
            warn!(
                pr_number = pr_number,
                branch = %branch_name,
                error = %e,
                "Failed to create worktree, continuing without it"
            );
            // Continue without worktree - agent can still be created
            // This allows the system to work even if git operations fail
            orchestrate_core::Worktree::new(
                format!("pr-{}", pr_number),
                format!("{}/.worktrees/pr-{}", env::current_dir().unwrap().display(), pr_number),
                branch_name.clone(),
                "main".to_string(),
            )
        }
    };

    // Create agent context
    let context = AgentContext {
        pr_number: Some(pr_number as i32),
        branch_name: Some(branch_name.clone()),
        working_directory: Some(worktree.path.clone()),
        custom: serde_json::json!({
            "repository": repo_full_name,
            "event_delivery_id": event.delivery_id,
        }),
        ..Default::default()
    };

    // Create pr-shepherd agent
    let mut agent = Agent::new(
        AgentType::PrShepherd,
        format!("Shepherd PR #{} on branch {}", pr_number, branch_name),
    )
    .with_context(context);

    // Associate worktree with agent
    worktree = worktree.with_agent(agent.id);
    agent = agent.with_worktree(worktree.id.clone());

    // Save agent to database FIRST (worktree has FK to agent)
    database.insert_agent(&agent).await?;

    // Save worktree to database AFTER agent
    database.insert_worktree(&worktree).await?;

    info!(
        agent_id = %agent.id,
        pr_number = pr_number,
        worktree_id = %worktree.id,
        "pr-shepherd agent and worktree created"
    );

    // Post comment on PR indicating orchestrate is watching
    // This is done asynchronously and errors are logged but not fatal
    if let Err(e) = try_post_pr_comment(pr_number as i32).await {
        error!(
            pr_number = pr_number,
            error = %e,
            "Failed to post PR comment, continuing anyway"
        );
    }

    // TODO: Actually spawn the agent (call orchestrate CLI or spawn process)

    Ok(())
}

/// Try to post a comment on the PR
///
/// This is a best-effort operation. Failures are logged but not fatal.
async fn try_post_pr_comment(pr_number: i32) -> Result<()> {
    let client = GitHubClient::new()
        .map_err(|e| orchestrate_core::Error::Other(format!("Failed to create GitHub client: {}", e)))?;

    let comment_body = format!(
        "🤖 **Orchestrate is now watching this PR**\n\n\
        I'll automatically:\n\
        - Monitor for review comments and fix issues\n\
        - Watch CI checks and resolve failures\n\
        - Keep the PR up to date\n\n\
        PR shepherd agent has been assigned to PR #{}.",
        pr_number
    );

    client.post_comment(pr_number, &comment_body)
        .map_err(|e| orchestrate_core::Error::Other(format!("Failed to post comment: {}", e)))?;

    info!(pr_number = pr_number, "Posted orchestrate watching comment");

    Ok(())
}

/// Handle a pull_request_review.submitted event
///
/// Spawns an issue-fixer agent when changes are requested.
///
/// Returns Ok(()) if event was handled successfully, Err if processing should be retried.
pub async fn handle_pr_review_submitted(
    database: Arc<Database>,
    event: &WebhookEvent,
) -> Result<()> {
    info!(
        delivery_id = %event.delivery_id,
        "Processing pull_request_review.submitted event"
    );

    // Parse payload
    let payload: Value = serde_json::from_str(&event.payload)?;

    // Extract action - must be "submitted"
    let action = payload
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing action field".to_string()))?;

    if action != "submitted" {
        debug!(action = %action, "Skipping non-submitted action");
        return Ok(());
    }

    // Extract review data
    let review = payload
        .get("review")
        .ok_or_else(|| orchestrate_core::Error::Other("Missing review field".to_string()))?;

    let review_state = review
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing review state".to_string()))?;

    // Only handle "changes_requested" reviews
    if review_state != "changes_requested" {
        debug!(
            review_state = %review_state,
            "Skipping review with state other than changes_requested"
        );
        return Ok(());
    }

    let review_body = review
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

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

    info!(
        pr_number = pr_number,
        branch = %branch_name,
        repository = %repo_full_name,
        "Spawning issue-fixer agent for review changes requested"
    );

    // Look for existing pr-shepherd agent for this PR
    let shepherd_agent_id = database
        .list_agents()
        .await?
        .into_iter()
        .find(|a| {
            a.agent_type == AgentType::PrShepherd
                && a.context.pr_number == Some(pr_number as i32)
        })
        .map(|a| a.id);

    // Build custom context with review information
    let mut custom = serde_json::json!({
        "repository": repo_full_name,
        "event_delivery_id": event.delivery_id,
        "review_body": review_body,
    });

    // Link to shepherd if found
    if let Some(shepherd_id) = shepherd_agent_id {
        custom["shepherd_agent_id"] = serde_json::json!(shepherd_id.to_string());
        info!(
            pr_number = pr_number,
            shepherd_id = %shepherd_id,
            "Linking issue-fixer to existing pr-shepherd"
        );
    }

    // Create agent context
    let context = AgentContext {
        pr_number: Some(pr_number as i32),
        branch_name: Some(branch_name.clone()),
        working_directory: None, // Will use existing worktree if shepherd exists
        custom,
        ..Default::default()
    };

    // Create issue-fixer agent
    let agent = Agent::new(
        AgentType::IssueFixer,
        format!(
            "Fix review comments for PR #{} on branch {}",
            pr_number, branch_name
        ),
    )
    .with_context(context);

    // Save agent to database
    database.insert_agent(&agent).await?;

    info!(
        agent_id = %agent.id,
        pr_number = pr_number,
        "issue-fixer agent created for review changes"
    );

    // TODO: Actually spawn the agent (call orchestrate CLI or spawn process)

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

    #[tokio::test]
    async fn test_handle_pr_opened_creates_worktree() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_pr_opened_payload(123, "feature/worktree-test", false);
        let event = WebhookEvent::new(
            "delivery-worktree".to_string(),
            "pull_request".to_string(),
            payload,
        );

        handle_pr_opened(database.clone(), &event).await.unwrap();

        // Verify agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);
        let agent = &agents[0];

        // Verify worktree was created (would be done by git worktree add in implementation)
        // For now, just verify that agent has worktree_id set
        assert!(agent.worktree_id.is_some(), "Agent should have worktree_id");

        let worktree_id = agent.worktree_id.as_ref().unwrap();

        // Verify worktree record exists in database
        let worktree_path = database.get_worktree_path(worktree_id).await.unwrap();
        assert!(worktree_path.is_some(), "Worktree should exist in database");
    }

    #[tokio::test]
    async fn test_handle_pr_opened_posts_comment() {
        // Note: This test verifies the function completes successfully
        // Actual GitHub API calls would need to be mocked or tested in integration tests
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_pr_opened_payload(456, "feature/comment-test", false);
        let event = WebhookEvent::new(
            "delivery-comment".to_string(),
            "pull_request".to_string(),
            payload,
        );

        let result = handle_pr_opened(database.clone(), &event).await;
        assert!(result.is_ok(), "Handler should succeed even if comment posting fails gracefully");

        // Verify agent was still created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);
    }

    // PR Review Event Handler Tests

    fn create_pr_review_payload(
        pr_number: i64,
        branch: &str,
        review_state: &str,
        review_body: &str,
    ) -> String {
        serde_json::json!({
            "action": "submitted",
            "review": {
                "state": review_state,
                "body": review_body,
            },
            "pull_request": {
                "number": pr_number,
                "head": {
                    "ref": branch,
                },
            },
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string()
    }

    #[tokio::test]
    async fn test_handle_pr_review_changes_requested_creates_agent() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let review_body = "Please fix the null pointer issue in line 42";
        let payload = create_pr_review_payload(
            55,
            "feature/fix-bug",
            "changes_requested",
            review_body,
        );
        let event = WebhookEvent::new(
            "delivery-review-123".to_string(),
            "pull_request_review".to_string(),
            payload,
        );

        let result = handle_pr_review_submitted(database.clone(), &event).await;
        assert!(result.is_ok());

        // Verify issue-fixer agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.agent_type, AgentType::IssueFixer);
        assert_eq!(agent.state, AgentState::Created);
        assert_eq!(agent.context.pr_number, Some(55));
        assert_eq!(agent.context.branch_name, Some("feature/fix-bug".to_string()));
        assert!(agent.task.contains("55"));
    }

    #[tokio::test]
    async fn test_handle_pr_review_skips_non_changes_requested() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Test "approved" review
        let payload = create_pr_review_payload(60, "feature/approved", "approved", "LGTM!");
        let event = WebhookEvent::new(
            "delivery-review-approved".to_string(),
            "pull_request_review".to_string(),
            payload,
        );

        let result = handle_pr_review_submitted(database.clone(), &event).await;
        assert!(result.is_ok());

        // No agent should be created for approved review
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);

        // Test "commented" review
        let payload = create_pr_review_payload(61, "feature/commented", "commented", "Nice work");
        let event = WebhookEvent::new(
            "delivery-review-commented".to_string(),
            "pull_request_review".to_string(),
            payload,
        );

        let result = handle_pr_review_submitted(database.clone(), &event).await;
        assert!(result.is_ok());

        // Still no agent
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_pr_review_skips_non_submitted_action() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = serde_json::json!({
            "action": "edited",
            "review": {
                "state": "changes_requested",
                "body": "Fix this",
            },
            "pull_request": {
                "number": 70,
                "head": {
                    "ref": "feature/edited",
                },
            },
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-review-edited".to_string(),
            "pull_request_review".to_string(),
            payload,
        );

        let result = handle_pr_review_submitted(database.clone(), &event).await;
        assert!(result.is_ok());

        // No agent for non-submitted action
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_pr_review_parses_review_comments() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let review_body = "Multiple issues:\n\
            1. Fix memory leak in function foo()\n\
            2. Update documentation for API changes\n\
            3. Add error handling for edge cases";

        let payload = create_pr_review_payload(
            75,
            "feature/multiple-fixes",
            "changes_requested",
            review_body,
        );
        let event = WebhookEvent::new(
            "delivery-review-comments".to_string(),
            "pull_request_review".to_string(),
            payload,
        );

        handle_pr_review_submitted(database.clone(), &event)
            .await
            .unwrap();

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        // Verify review body is in the agent context
        let custom = &agent.context.custom;
        assert!(custom.get("review_body").is_some());
        assert_eq!(custom.get("review_body").unwrap().as_str().unwrap(), review_body);
    }

    #[tokio::test]
    async fn test_handle_pr_review_links_to_existing_shepherd() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // First create a pr-shepherd agent for PR #80
        let shepherd_context = AgentContext {
            pr_number: Some(80),
            branch_name: Some("feature/with-shepherd".to_string()),
            ..Default::default()
        };
        let shepherd = Agent::new(
            AgentType::PrShepherd,
            "Shepherd PR #80".to_string(),
        )
        .with_context(shepherd_context);
        database.insert_agent(&shepherd).await.unwrap();

        // Now submit a review for the same PR
        let payload = create_pr_review_payload(
            80,
            "feature/with-shepherd",
            "changes_requested",
            "Please fix the tests",
        );
        let event = WebhookEvent::new(
            "delivery-review-link".to_string(),
            "pull_request_review".to_string(),
            payload,
        );

        handle_pr_review_submitted(database.clone(), &event)
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
    async fn test_handle_pr_review_missing_fields() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Missing review field
        let payload = serde_json::json!({
            "action": "submitted",
            "pull_request": {
                "number": 90,
            }
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-review-missing".to_string(),
            "pull_request_review".to_string(),
            payload,
        );

        let result = handle_pr_review_submitted(database.clone(), &event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_pr_review_extracts_repository_info() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_pr_review_payload(
            100,
            "feature/repo-info",
            "changes_requested",
            "Fix the bug",
        );
        let event = WebhookEvent::new(
            "delivery-review-repo".to_string(),
            "pull_request_review".to_string(),
            payload,
        );

        handle_pr_review_submitted(database.clone(), &event)
            .await
            .unwrap();

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        let custom = &agent.context.custom;
        assert_eq!(custom.get("repository").unwrap().as_str().unwrap(), "owner/repo");
        assert_eq!(
            custom.get("event_delivery_id").unwrap().as_str().unwrap(),
            "delivery-review-repo"
        );
    }
}
