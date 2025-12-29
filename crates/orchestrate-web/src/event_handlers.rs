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

/// Handle a check_run.completed or check_suite.completed event
///
/// Spawns an issue-fixer agent when CI fails.
///
/// Returns Ok(()) if event was handled successfully, Err if processing should be retried.
pub async fn handle_ci_status(
    database: Arc<Database>,
    event: &WebhookEvent,
) -> Result<()> {
    info!(
        delivery_id = %event.delivery_id,
        event_type = %event.event_type,
        "Processing CI status event"
    );

    // Parse payload
    let payload: Value = serde_json::from_str(&event.payload)?;

    // Handle both check_run and check_suite events
    match event.event_type.as_str() {
        "check_run" => handle_check_run_completed(database, event, payload).await,
        "check_suite" => handle_check_suite_completed(database, event, payload).await,
        _ => {
            warn!(event_type = %event.event_type, "Unexpected event type in handle_ci_status");
            Ok(())
        }
    }
}

/// Handle check_run.completed event
async fn handle_check_run_completed(
    database: Arc<Database>,
    event: &WebhookEvent,
    payload: Value,
) -> Result<()> {
    // Extract action - must be "completed"
    let action = payload
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing action field".to_string()))?;

    if action != "completed" {
        debug!(action = %action, "Skipping non-completed action");
        return Ok(());
    }

    // Extract check_run data
    let check_run = payload
        .get("check_run")
        .ok_or_else(|| orchestrate_core::Error::Other("Missing check_run field".to_string()))?;

    let conclusion = check_run
        .get("conclusion")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing conclusion field".to_string()))?;

    // Only handle failure or timed_out conclusions
    if conclusion != "failure" && conclusion != "timed_out" {
        debug!(
            conclusion = %conclusion,
            "Skipping check_run with conclusion other than failure/timed_out"
        );
        return Ok(());
    }

    let check_name = check_run
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Check")
        .to_string();

    let check_id = check_run
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing check_run id".to_string()))?;

    let details_url = check_run
        .get("details_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let html_url = check_run
        .get("html_url")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract PR information if available
    let pr_numbers: Vec<i32> = check_run
        .get("pull_requests")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|pr| pr.get("number").and_then(|n| n.as_i64()).map(|n| n as i32))
                .collect()
        })
        .unwrap_or_default();

    // Extract branch/commit information
    let head_sha = check_run
        .get("head_sha")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing head_sha field".to_string()))?
        .to_string();

    let head_branch = check_run
        .get("check_suite")
        .and_then(|cs| cs.get("head_branch"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let repo_full_name = payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing repository name".to_string()))?
        .to_string();

    // Check for duplicate fixers for the same check failure
    if let Some(duplicate) = find_duplicate_ci_fixer(
        &database,
        check_id,
        &head_sha,
        pr_numbers.first().copied(),
    )
    .await?
    {
        info!(
            check_id = check_id,
            check_name = %check_name,
            existing_agent_id = %duplicate,
            "Skipping duplicate CI fixer for same check failure"
        );
        return Ok(());
    }

    info!(
        check_name = %check_name,
        check_id = check_id,
        conclusion = %conclusion,
        pr_numbers = ?pr_numbers,
        head_sha = %head_sha,
        repository = %repo_full_name,
        "Spawning issue-fixer agent for CI failure"
    );

    // Build custom context with CI failure information
    let mut custom = serde_json::json!({
        "repository": repo_full_name,
        "event_delivery_id": event.delivery_id,
        "ci_check_name": check_name,
        "ci_check_id": check_id,
        "ci_conclusion": conclusion,
        "ci_head_sha": head_sha,
    });

    if let Some(url) = details_url {
        custom["ci_details_url"] = serde_json::json!(url);
    }
    if let Some(url) = html_url {
        custom["ci_html_url"] = serde_json::json!(url);
    }
    if let Some(branch) = &head_branch {
        custom["ci_head_branch"] = serde_json::json!(branch);
    }

    // Look for existing pr-shepherd agent if this is for a PR
    let (pr_number, branch_name, _shepherd_agent_id) = if let Some(pr_num) = pr_numbers.first() {
        let shepherd_id = database
            .list_agents()
            .await?
            .into_iter()
            .find(|a| {
                a.agent_type == AgentType::PrShepherd && a.context.pr_number == Some(*pr_num)
            })
            .map(|a| a.id);

        if let Some(shepherd_id) = shepherd_id {
            custom["shepherd_agent_id"] = serde_json::json!(shepherd_id.to_string());
            info!(
                pr_number = pr_num,
                shepherd_id = %shepherd_id,
                "Linking CI fixer to existing pr-shepherd"
            );
        }

        (Some(*pr_num), head_branch.clone(), shepherd_id)
    } else {
        (None, head_branch.clone(), None)
    };

    // Create agent context
    let context = AgentContext {
        pr_number,
        branch_name,
        working_directory: None, // Will use existing worktree if shepherd exists
        custom,
        ..Default::default()
    };

    // Create issue-fixer agent
    let task = if let Some(pr_num) = pr_number {
        format!(
            "Fix CI failure '{}' for PR #{} ({})",
            check_name, pr_num, conclusion
        )
    } else {
        format!("Fix CI failure '{}' ({})", check_name, conclusion)
    };

    let agent = Agent::new(AgentType::IssueFixer, task).with_context(context);

    // Save agent to database
    database.insert_agent(&agent).await?;

    info!(
        agent_id = %agent.id,
        check_name = %check_name,
        pr_number = ?pr_number,
        "issue-fixer agent created for CI failure"
    );

    Ok(())
}

/// Handle check_suite.completed event
async fn handle_check_suite_completed(
    database: Arc<Database>,
    event: &WebhookEvent,
    payload: Value,
) -> Result<()> {
    // Extract action - must be "completed"
    let action = payload
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing action field".to_string()))?;

    if action != "completed" {
        debug!(action = %action, "Skipping non-completed action");
        return Ok(());
    }

    // Extract check_suite data
    let check_suite = payload
        .get("check_suite")
        .ok_or_else(|| orchestrate_core::Error::Other("Missing check_suite field".to_string()))?;

    let conclusion = check_suite
        .get("conclusion")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing conclusion field".to_string()))?;

    // Only handle failure or timed_out conclusions
    if conclusion != "failure" && conclusion != "timed_out" {
        debug!(
            conclusion = %conclusion,
            "Skipping check_suite with conclusion other than failure/timed_out"
        );
        return Ok(());
    }

    let suite_id = check_suite
        .get("id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing check_suite id".to_string()))?;

    let head_sha = check_suite
        .get("head_sha")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing head_sha field".to_string()))?
        .to_string();

    let head_branch = check_suite
        .get("head_branch")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Extract PR information if available
    let pr_numbers: Vec<i32> = check_suite
        .get("pull_requests")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|pr| pr.get("number").and_then(|n| n.as_i64()).map(|n| n as i32))
                .collect()
        })
        .unwrap_or_default();

    let repo_full_name = payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing repository name".to_string()))?
        .to_string();

    // Check for duplicate fixers for the same suite failure
    if let Some(duplicate) = find_duplicate_ci_fixer(
        &database,
        suite_id,
        &head_sha,
        pr_numbers.first().copied(),
    )
    .await?
    {
        info!(
            suite_id = suite_id,
            existing_agent_id = %duplicate,
            "Skipping duplicate CI fixer for same check suite failure"
        );
        return Ok(());
    }

    info!(
        suite_id = suite_id,
        conclusion = %conclusion,
        pr_numbers = ?pr_numbers,
        head_sha = %head_sha,
        repository = %repo_full_name,
        "Spawning issue-fixer agent for check suite failure"
    );

    // Build custom context with CI failure information
    let mut custom = serde_json::json!({
        "repository": repo_full_name,
        "event_delivery_id": event.delivery_id,
        "ci_suite_id": suite_id,
        "ci_conclusion": conclusion,
        "ci_head_sha": head_sha,
    });

    if let Some(branch) = &head_branch {
        custom["ci_head_branch"] = serde_json::json!(branch);
    }

    // Look for existing pr-shepherd agent if this is for a PR
    let (pr_number, branch_name, _shepherd_agent_id) = if let Some(pr_num) = pr_numbers.first() {
        let shepherd_id = database
            .list_agents()
            .await?
            .into_iter()
            .find(|a| {
                a.agent_type == AgentType::PrShepherd && a.context.pr_number == Some(*pr_num)
            })
            .map(|a| a.id);

        if let Some(shepherd_id) = shepherd_id {
            custom["shepherd_agent_id"] = serde_json::json!(shepherd_id.to_string());
            info!(
                pr_number = pr_num,
                shepherd_id = %shepherd_id,
                "Linking CI fixer to existing pr-shepherd"
            );
        }

        (Some(*pr_num), head_branch.clone(), shepherd_id)
    } else {
        (None, head_branch.clone(), None)
    };

    // Create agent context
    let context = AgentContext {
        pr_number,
        branch_name,
        working_directory: None, // Will use existing worktree if shepherd exists
        custom,
        ..Default::default()
    };

    // Create issue-fixer agent
    let task = if let Some(pr_num) = pr_number {
        format!("Fix CI suite failure for PR #{} ({})", pr_num, conclusion)
    } else {
        format!("Fix CI suite failure ({})", conclusion)
    };

    let agent = Agent::new(AgentType::IssueFixer, task).with_context(context);

    // Save agent to database
    database.insert_agent(&agent).await?;

    info!(
        agent_id = %agent.id,
        suite_id = suite_id,
        pr_number = ?pr_number,
        "issue-fixer agent created for check suite failure"
    );

    Ok(())
}

/// Find duplicate CI fixer agents for the same failure
///
/// Checks if we already have an issue-fixer agent for this specific CI failure.
/// Returns the agent ID if found.
async fn find_duplicate_ci_fixer(
    database: &Arc<Database>,
    check_id: i64,
    head_sha: &str,
    pr_number: Option<i32>,
) -> Result<Option<uuid::Uuid>> {
    let agents = database.list_agents().await?;

    for agent in agents {
        if agent.agent_type != AgentType::IssueFixer {
            continue;
        }

        // Check if this fixer is for the same check/suite
        let has_same_check = agent
            .context
            .custom
            .get("ci_check_id")
            .and_then(|v| v.as_i64())
            .map(|id| id == check_id)
            .unwrap_or(false);

        let has_same_suite = agent
            .context
            .custom
            .get("ci_suite_id")
            .and_then(|v| v.as_i64())
            .map(|id| id == check_id)
            .unwrap_or(false);

        if !has_same_check && !has_same_suite {
            continue;
        }

        // Same check/suite - now verify it's the same commit and PR
        let same_sha = agent
            .context
            .custom
            .get("ci_head_sha")
            .and_then(|v| v.as_str())
            .map(|sha| sha == head_sha)
            .unwrap_or(false);

        let same_pr = agent.context.pr_number == pr_number;

        if same_sha && same_pr {
            return Ok(Some(agent.id));
        }
    }

    Ok(None)
}

/// Handle a push event to main/master branch
///
/// Spawns a regression-tester agent to run the test suite.
///
/// Returns Ok(()) if event was handled successfully, Err if processing should be retried.
pub async fn handle_push_to_main(
    database: Arc<Database>,
    event: &WebhookEvent,
) -> Result<()> {
    info!(
        delivery_id = %event.delivery_id,
        "Processing push event"
    );

    // Parse payload
    let payload: Value = serde_json::from_str(&event.payload)?;

    // Extract ref - must be refs/heads/main or refs/heads/master
    let ref_name = payload
        .get("ref")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing ref field".to_string()))?;

    // Only handle pushes to main or master branch
    if ref_name != "refs/heads/main" && ref_name != "refs/heads/master" {
        debug!(ref_name = %ref_name, "Skipping push to non-main branch");
        return Ok(());
    }

    let branch_name = if ref_name == "refs/heads/main" {
        "main"
    } else {
        "master"
    };

    // Extract commit range
    let before_sha = payload
        .get("before")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing before SHA".to_string()))?
        .to_string();

    let after_sha = payload
        .get("after")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing after SHA".to_string()))?
        .to_string();

    // Extract repository info
    let repo_full_name = payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing repository name".to_string()))?
        .to_string();

    // Extract commits array to get changed files
    let commits = payload
        .get("commits")
        .and_then(|v| v.as_array())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing commits array".to_string()))?;

    // Collect all changed files from commits
    let mut changed_files = std::collections::HashSet::new();
    for commit in commits {
        // Extract added files
        if let Some(added) = commit.get("added").and_then(|v| v.as_array()) {
            for file in added {
                if let Some(f) = file.as_str() {
                    changed_files.insert(f.to_string());
                }
            }
        }
        // Extract modified files
        if let Some(modified) = commit.get("modified").and_then(|v| v.as_array()) {
            for file in modified {
                if let Some(f) = file.as_str() {
                    changed_files.insert(f.to_string());
                }
            }
        }
        // Extract removed files
        if let Some(removed) = commit.get("removed").and_then(|v| v.as_array()) {
            for file in removed {
                if let Some(f) = file.as_str() {
                    changed_files.insert(f.to_string());
                }
            }
        }
    }

    let changed_files_vec: Vec<String> = changed_files.into_iter().collect();

    info!(
        branch = %branch_name,
        before_sha = %before_sha,
        after_sha = %after_sha,
        repository = %repo_full_name,
        changed_files_count = changed_files_vec.len(),
        "Spawning regression-tester agent for push to main"
    );

    // Build custom context with push information
    let custom = serde_json::json!({
        "repository": repo_full_name,
        "event_delivery_id": event.delivery_id,
        "before_sha": before_sha,
        "after_sha": after_sha,
        "commit_range": format!("{}..{}", before_sha, after_sha),
        "changed_files": changed_files_vec,
    });

    // Create agent context
    let context = AgentContext {
        pr_number: None,
        branch_name: Some(branch_name.to_string()),
        working_directory: None, // Will use main repo directory
        custom,
        ..Default::default()
    };

    // Create regression-tester agent
    let agent = Agent::new(
        AgentType::RegressionTester,
        format!(
            "Run regression tests for {} push to {}",
            repo_full_name, branch_name
        ),
    )
    .with_context(context);

    // Save agent to database
    database.insert_agent(&agent).await?;

    info!(
        agent_id = %agent.id,
        branch = %branch_name,
        "regression-tester agent created for push to main"
    );

    // TODO: Actually spawn the agent (call orchestrate CLI or spawn process)
    // TODO: Agent should run test suite and create issue if regressions detected

    Ok(())
}

/// Handle an issues.opened event
///
/// Spawns an issue-triager agent to analyze and triage the issue.
///
/// Returns Ok(()) if event was handled successfully, Err if processing should be retried.
pub async fn handle_issue_opened(
    database: Arc<Database>,
    event: &WebhookEvent,
) -> Result<()> {
    info!(
        delivery_id = %event.delivery_id,
        "Processing issues.opened event"
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

    // Extract issue data
    let issue = payload
        .get("issue")
        .ok_or_else(|| orchestrate_core::Error::Other("Missing issue field".to_string()))?;

    let issue_number = issue
        .get("number")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing issue number".to_string()))?;

    let issue_title = issue
        .get("title")
        .and_then(|v| v.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing issue title".to_string()))?
        .to_string();

    let issue_body = issue
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract repository info
    let repo_full_name = payload
        .get("repository")
        .and_then(|r| r.get("full_name"))
        .and_then(|n| n.as_str())
        .ok_or_else(|| orchestrate_core::Error::Other("Missing repository name".to_string()))?
        .to_string();

    // Extract optional labels
    let labels: Vec<String> = issue
        .get("labels")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|label| label.get("name").and_then(|n| n.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    // Extract optional assignees
    let assignees: Vec<String> = issue
        .get("assignees")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|assignee| assignee.get("login").and_then(|l| l.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();

    info!(
        issue_number = issue_number,
        issue_title = %issue_title,
        repository = %repo_full_name,
        labels_count = labels.len(),
        assignees_count = assignees.len(),
        "Spawning issue-triager agent for new issue"
    );

    // Build custom context with issue information
    let mut custom = serde_json::json!({
        "repository": repo_full_name,
        "event_delivery_id": event.delivery_id,
        "issue_number": issue_number,
        "issue_title": issue_title.clone(),
        "issue_body": issue_body,
    });

    if !labels.is_empty() {
        custom["issue_labels"] = serde_json::json!(labels);
    }

    if !assignees.is_empty() {
        custom["issue_assignees"] = serde_json::json!(assignees);
    }

    // Create agent context
    let context = AgentContext {
        pr_number: None,
        branch_name: None,
        working_directory: None,
        custom,
        ..Default::default()
    };

    // Create issue-triager agent
    // Truncate title if too long for task description
    let title_preview = if issue_title.len() > 80 {
        format!("{}...", &issue_title[..80])
    } else {
        issue_title.clone()
    };

    let agent = Agent::new(
        AgentType::IssueTriager,
        format!(
            "Triage issue #{} - {}",
            issue_number, title_preview
        ),
    )
    .with_context(context);

    // Save agent to database
    database.insert_agent(&agent).await?;

    info!(
        agent_id = %agent.id,
        issue_number = issue_number,
        "issue-triager agent created for new issue"
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

    // CI Status Event Handler Tests

    fn create_check_run_completed_payload(
        check_id: i64,
        check_name: &str,
        conclusion: &str,
        pr_number: Option<i64>,
        head_sha: &str,
        head_branch: Option<&str>,
    ) -> String {
        let mut pull_requests = vec![];
        if let Some(pr_num) = pr_number {
            pull_requests.push(serde_json::json!({"number": pr_num}));
        }

        let mut check_suite = serde_json::json!({});
        if let Some(branch) = head_branch {
            check_suite["head_branch"] = serde_json::json!(branch);
        }

        serde_json::json!({
            "action": "completed",
            "check_run": {
                "id": check_id,
                "name": check_name,
                "status": "completed",
                "conclusion": conclusion,
                "head_sha": head_sha,
                "pull_requests": pull_requests,
                "check_suite": check_suite,
                "details_url": format!("https://example.com/checks/{}", check_id),
                "html_url": format!("https://example.com/runs/{}", check_id),
            },
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string()
    }

    fn create_check_suite_completed_payload(
        suite_id: i64,
        conclusion: &str,
        pr_number: Option<i64>,
        head_sha: &str,
        head_branch: Option<&str>,
    ) -> String {
        let mut pull_requests = vec![];
        if let Some(pr_num) = pr_number {
            pull_requests.push(serde_json::json!({"number": pr_num}));
        }

        let mut check_suite = serde_json::json!({
            "id": suite_id,
            "status": "completed",
            "conclusion": conclusion,
            "head_sha": head_sha,
            "pull_requests": pull_requests,
        });

        if let Some(branch) = head_branch {
            check_suite["head_branch"] = serde_json::json!(branch);
        }

        serde_json::json!({
            "action": "completed",
            "check_suite": check_suite,
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string()
    }

    #[tokio::test]
    async fn test_handle_ci_check_run_failure_creates_agent() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_check_run_completed_payload(
            12345,
            "build",
            "failure",
            Some(42),
            "abc123def456",
            Some("feature/test"),
        );
        let event = WebhookEvent::new(
            "delivery-check-run-1".to_string(),
            "check_run".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_ok());

        // Verify issue-fixer agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.agent_type, AgentType::IssueFixer);
        assert_eq!(agent.state, AgentState::Created);
        assert_eq!(agent.context.pr_number, Some(42));
        assert_eq!(
            agent.context.branch_name,
            Some("feature/test".to_string())
        );
        assert!(agent.task.contains("build"));
        assert!(agent.task.contains("42"));
        assert!(agent.task.contains("failure"));

        // Verify context contains CI failure details
        let custom = &agent.context.custom;
        assert_eq!(custom.get("ci_check_name").unwrap().as_str().unwrap(), "build");
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
        assert!(custom.get("ci_html_url").is_some());
    }

    #[tokio::test]
    async fn test_handle_ci_check_run_timed_out_creates_agent() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_check_run_completed_payload(
            67890,
            "test",
            "timed_out",
            Some(99),
            "def789ghi012",
            Some("fix/timeout"),
        );
        let event = WebhookEvent::new(
            "delivery-check-run-2".to_string(),
            "check_run".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_ok());

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.agent_type, AgentType::IssueFixer);
        assert!(agent.task.contains("timed_out"));
    }

    #[tokio::test]
    async fn test_handle_ci_check_run_success_skipped() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_check_run_completed_payload(
            11111,
            "lint",
            "success",
            Some(50),
            "success123",
            Some("feature/success"),
        );
        let event = WebhookEvent::new(
            "delivery-check-run-success".to_string(),
            "check_run".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_ok());

        // No agent should be created for successful check
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_ci_check_run_without_pr() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_check_run_completed_payload(
            22222,
            "deploy",
            "failure",
            None,
            "commit789",
            Some("main"),
        );
        let event = WebhookEvent::new(
            "delivery-check-run-no-pr".to_string(),
            "check_run".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_ok());

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.context.pr_number, None);
        assert_eq!(agent.context.branch_name, Some("main".to_string()));
        assert!(!agent.task.contains("PR #"));
    }

    #[tokio::test]
    async fn test_handle_ci_check_run_links_to_shepherd() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // First create a pr-shepherd agent for PR #75
        let shepherd_context = AgentContext {
            pr_number: Some(75),
            branch_name: Some("feature/with-ci".to_string()),
            ..Default::default()
        };
        let shepherd = Agent::new(AgentType::PrShepherd, "Shepherd PR #75".to_string())
            .with_context(shepherd_context);
        database.insert_agent(&shepherd).await.unwrap();

        // Now trigger CI failure for the same PR
        let payload = create_check_run_completed_payload(
            33333,
            "integration-test",
            "failure",
            Some(75),
            "ci123",
            Some("feature/with-ci"),
        );
        let event = WebhookEvent::new(
            "delivery-ci-shepherd".to_string(),
            "check_run".to_string(),
            payload,
        );

        handle_ci_status(database.clone(), &event).await.unwrap();

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
    async fn test_handle_ci_check_run_skips_duplicate() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create first fixer agent for this check
        let context = AgentContext {
            pr_number: Some(88),
            branch_name: Some("feature/dup".to_string()),
            custom: serde_json::json!({
                "ci_check_id": 44444,
                "ci_head_sha": "dup123",
            }),
            ..Default::default()
        };
        let first_fixer = Agent::new(AgentType::IssueFixer, "Fix CI".to_string())
            .with_context(context);
        database.insert_agent(&first_fixer).await.unwrap();

        // Now trigger the same CI failure again
        let payload = create_check_run_completed_payload(
            44444,
            "build",
            "failure",
            Some(88),
            "dup123",
            Some("feature/dup"),
        );
        let event = WebhookEvent::new(
            "delivery-ci-duplicate".to_string(),
            "check_run".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_ok());

        // Should still have only one agent (no duplicate)
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);
    }

    #[tokio::test]
    async fn test_handle_ci_check_run_different_commit_not_duplicate() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Create first fixer agent for this check
        let context = AgentContext {
            pr_number: Some(88),
            branch_name: Some("feature/dup".to_string()),
            custom: serde_json::json!({
                "ci_check_id": 44444,
                "ci_head_sha": "first_commit",
            }),
            ..Default::default()
        };
        let first_fixer = Agent::new(AgentType::IssueFixer, "Fix CI".to_string())
            .with_context(context);
        database.insert_agent(&first_fixer).await.unwrap();

        // Now trigger the same check but different commit (e.g., after a fix attempt)
        let payload = create_check_run_completed_payload(
            44444,
            "build",
            "failure",
            Some(88),
            "second_commit", // Different SHA
            Some("feature/dup"),
        );
        let event = WebhookEvent::new(
            "delivery-ci-different-commit".to_string(),
            "check_run".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_ok());

        // Should have two agents (different commit = not a duplicate)
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_ci_check_suite_failure_creates_agent() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_check_suite_completed_payload(
            55555,
            "failure",
            Some(100),
            "suite123",
            Some("feature/suite"),
        );
        let event = WebhookEvent::new(
            "delivery-suite-1".to_string(),
            "check_suite".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_ok());

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.agent_type, AgentType::IssueFixer);
        assert_eq!(agent.context.pr_number, Some(100));
        assert!(agent.task.contains("100"));
        assert!(agent.task.contains("suite"));

        let custom = &agent.context.custom;
        assert_eq!(custom.get("ci_suite_id").unwrap().as_i64().unwrap(), 55555);
        assert_eq!(
            custom.get("ci_conclusion").unwrap().as_str().unwrap(),
            "failure"
        );
    }

    #[tokio::test]
    async fn test_handle_ci_check_suite_success_skipped() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_check_suite_completed_payload(
            66666,
            "success",
            Some(101),
            "suite456",
            Some("feature/ok"),
        );
        let event = WebhookEvent::new(
            "delivery-suite-success".to_string(),
            "check_suite".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_ok());

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_ci_check_run_non_completed_action() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = serde_json::json!({
            "action": "requested",
            "check_run": {
                "id": 77777,
                "name": "test",
                "status": "queued",
            },
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-non-completed".to_string(),
            "check_run".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_ok());

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_ci_check_run_missing_fields() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Missing check_run field
        let payload = serde_json::json!({
            "action": "completed"
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-missing".to_string(),
            "check_run".to_string(),
            payload,
        );

        let result = handle_ci_status(database.clone(), &event).await;
        assert!(result.is_err());
    }

    // Push to Main Event Handler Tests

    fn create_push_payload(
        ref_name: &str,
        before_sha: &str,
        after_sha: &str,
        commits: Vec<serde_json::Value>,
    ) -> String {
        serde_json::json!({
            "ref": ref_name,
            "before": before_sha,
            "after": after_sha,
            "repository": {
                "full_name": "owner/repo"
            },
            "commits": commits
        })
        .to_string()
    }

    fn create_commit(
        added: Vec<&str>,
        modified: Vec<&str>,
        removed: Vec<&str>,
    ) -> serde_json::Value {
        serde_json::json!({
            "added": added,
            "modified": modified,
            "removed": removed,
        })
    }

    #[tokio::test]
    async fn test_handle_push_to_main_creates_agent() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let commits = vec![
            create_commit(vec!["file1.rs"], vec!["file2.rs"], vec![]),
            create_commit(vec![], vec!["file3.rs"], vec!["old.rs"]),
        ];

        let payload = create_push_payload(
            "refs/heads/main",
            "abc123",
            "def456",
            commits,
        );

        let event = WebhookEvent::new(
            "delivery-push-1".to_string(),
            "push".to_string(),
            payload,
        );

        let result = handle_push_to_main(database.clone(), &event).await;
        assert!(result.is_ok());

        // Verify regression-tester agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.agent_type, AgentType::RegressionTester);
        assert_eq!(agent.state, AgentState::Created);
        assert_eq!(agent.context.branch_name, Some("main".to_string()));
        assert!(agent.task.contains("owner/repo"));
        assert!(agent.task.contains("main"));

        // Verify context contains push details
        let custom = &agent.context.custom;
        assert_eq!(custom.get("repository").unwrap().as_str().unwrap(), "owner/repo");
        assert_eq!(custom.get("before_sha").unwrap().as_str().unwrap(), "abc123");
        assert_eq!(custom.get("after_sha").unwrap().as_str().unwrap(), "def456");
        assert_eq!(custom.get("commit_range").unwrap().as_str().unwrap(), "abc123..def456");

        // Verify changed files were collected
        let changed_files = custom.get("changed_files").unwrap().as_array().unwrap();
        assert_eq!(changed_files.len(), 4);

        // Convert to set for comparison (order doesn't matter)
        let file_names: std::collections::HashSet<String> = changed_files
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert!(file_names.contains("file1.rs"));
        assert!(file_names.contains("file2.rs"));
        assert!(file_names.contains("file3.rs"));
        assert!(file_names.contains("old.rs"));
    }

    #[tokio::test]
    async fn test_handle_push_to_master_creates_agent() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let commits = vec![create_commit(vec!["test.rs"], vec![], vec![])];

        let payload = create_push_payload(
            "refs/heads/master",
            "sha1",
            "sha2",
            commits,
        );

        let event = WebhookEvent::new(
            "delivery-push-master".to_string(),
            "push".to_string(),
            payload,
        );

        let result = handle_push_to_main(database.clone(), &event).await;
        assert!(result.is_ok());

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.agent_type, AgentType::RegressionTester);
        assert_eq!(agent.context.branch_name, Some("master".to_string()));
    }

    #[tokio::test]
    async fn test_handle_push_skips_feature_branch() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let commits = vec![create_commit(vec!["file.rs"], vec![], vec![])];

        let payload = create_push_payload(
            "refs/heads/feature/new-feature",
            "sha1",
            "sha2",
            commits,
        );

        let event = WebhookEvent::new(
            "delivery-push-feature".to_string(),
            "push".to_string(),
            payload,
        );

        let result = handle_push_to_main(database.clone(), &event).await;
        assert!(result.is_ok());

        // No agent should be created for feature branch
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_push_skips_develop_branch() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let commits = vec![create_commit(vec!["file.rs"], vec![], vec![])];

        let payload = create_push_payload(
            "refs/heads/develop",
            "sha1",
            "sha2",
            commits,
        );

        let event = WebhookEvent::new(
            "delivery-push-develop".to_string(),
            "push".to_string(),
            payload,
        );

        let result = handle_push_to_main(database.clone(), &event).await;
        assert!(result.is_ok());

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_push_missing_fields() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Missing commits field
        let payload = serde_json::json!({
            "ref": "refs/heads/main",
            "before": "sha1",
            "after": "sha2",
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-push-missing".to_string(),
            "push".to_string(),
            payload,
        );

        let result = handle_push_to_main(database.clone(), &event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_push_no_changed_files() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Empty commits
        let commits = vec![];

        let payload = create_push_payload(
            "refs/heads/main",
            "sha1",
            "sha2",
            commits,
        );

        let event = WebhookEvent::new(
            "delivery-push-empty".to_string(),
            "push".to_string(),
            payload,
        );

        let result = handle_push_to_main(database.clone(), &event).await;
        assert!(result.is_ok());

        // Agent should still be created even with no changed files
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        let custom = &agent.context.custom;
        let changed_files = custom.get("changed_files").unwrap().as_array().unwrap();
        assert_eq!(changed_files.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_push_deduplicates_files() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Multiple commits modifying the same files
        let commits = vec![
            create_commit(vec!["file1.rs"], vec!["file2.rs"], vec![]),
            create_commit(vec![], vec!["file1.rs", "file2.rs"], vec![]),
            create_commit(vec![], vec!["file2.rs"], vec![]),
        ];

        let payload = create_push_payload(
            "refs/heads/main",
            "sha1",
            "sha2",
            commits,
        );

        let event = WebhookEvent::new(
            "delivery-push-dedup".to_string(),
            "push".to_string(),
            payload,
        );

        handle_push_to_main(database.clone(), &event).await.unwrap();

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        let custom = &agent.context.custom;
        let changed_files = custom.get("changed_files").unwrap().as_array().unwrap();

        // Should only have 2 unique files
        assert_eq!(changed_files.len(), 2);
    }

    #[tokio::test]
    async fn test_handle_push_extracts_repository_info() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let commits = vec![create_commit(vec!["test.rs"], vec![], vec![])];

        let payload = create_push_payload(
            "refs/heads/main",
            "before123",
            "after456",
            commits,
        );

        let event = WebhookEvent::new(
            "delivery-push-repo".to_string(),
            "push".to_string(),
            payload,
        );

        handle_push_to_main(database.clone(), &event).await.unwrap();

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        let custom = &agent.context.custom;
        assert_eq!(custom.get("repository").unwrap().as_str().unwrap(), "owner/repo");
        assert_eq!(
            custom.get("event_delivery_id").unwrap().as_str().unwrap(),
            "delivery-push-repo"
        );
    }

    // Issue Created Event Handler Tests

    fn create_issue_opened_payload(
        issue_number: i64,
        title: &str,
        body: &str,
    ) -> String {
        serde_json::json!({
            "action": "opened",
            "issue": {
                "number": issue_number,
                "title": title,
                "body": body,
                "state": "open",
            },
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string()
    }

    #[tokio::test]
    async fn test_handle_issue_opened_creates_agent() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let issue_title = "Bug: Application crashes on startup";
        let issue_body = "When I run the app, it immediately crashes with error XYZ";
        let payload = create_issue_opened_payload(101, issue_title, issue_body);
        let event = WebhookEvent::new(
            "delivery-issue-1".to_string(),
            "issues".to_string(),
            payload,
        );

        let result = handle_issue_opened(database.clone(), &event).await;
        assert!(result.is_ok());

        // Verify issue-triager agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.agent_type, AgentType::IssueTriager);
        assert_eq!(agent.state, AgentState::Created);
        assert!(agent.task.contains("101"));
        assert!(agent.task.contains("Bug: Application crashes"));

        // Verify context contains issue details
        let custom = &agent.context.custom;
        assert_eq!(custom.get("issue_number").unwrap().as_i64().unwrap(), 101);
        assert_eq!(custom.get("issue_title").unwrap().as_str().unwrap(), issue_title);
        assert_eq!(custom.get("issue_body").unwrap().as_str().unwrap(), issue_body);
        assert_eq!(custom.get("repository").unwrap().as_str().unwrap(), "owner/repo");
    }

    #[tokio::test]
    async fn test_handle_issue_opened_skips_non_opened_action() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = serde_json::json!({
            "action": "closed",
            "issue": {
                "number": 102,
                "title": "Some issue",
                "body": "Issue body",
                "state": "closed",
            },
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-issue-closed".to_string(),
            "issues".to_string(),
            payload,
        );

        let result = handle_issue_opened(database.clone(), &event).await;
        assert!(result.is_ok());

        // No agent should be created for closed action
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_handle_issue_opened_handles_empty_body() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_issue_opened_payload(103, "Issue without body", "");
        let event = WebhookEvent::new(
            "delivery-issue-empty".to_string(),
            "issues".to_string(),
            payload,
        );

        let result = handle_issue_opened(database.clone(), &event).await;
        assert!(result.is_ok());

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        assert_eq!(agent.agent_type, AgentType::IssueTriager);
        let custom = &agent.context.custom;
        assert_eq!(custom.get("issue_body").unwrap().as_str().unwrap(), "");
    }

    #[tokio::test]
    async fn test_handle_issue_opened_missing_fields() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Missing issue field
        let payload = serde_json::json!({
            "action": "opened",
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-issue-missing".to_string(),
            "issues".to_string(),
            payload,
        );

        let result = handle_issue_opened(database.clone(), &event).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_handle_issue_opened_with_labels() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = serde_json::json!({
            "action": "opened",
            "issue": {
                "number": 104,
                "title": "Feature request",
                "body": "Add support for feature X",
                "state": "open",
                "labels": [
                    {"name": "enhancement"},
                    {"name": "priority:high"}
                ]
            },
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-issue-labels".to_string(),
            "issues".to_string(),
            payload,
        );

        handle_issue_opened(database.clone(), &event).await.unwrap();

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        let custom = &agent.context.custom;
        let labels = custom.get("issue_labels").unwrap().as_array().unwrap();
        assert_eq!(labels.len(), 2);
        assert_eq!(labels[0].as_str().unwrap(), "enhancement");
        assert_eq!(labels[1].as_str().unwrap(), "priority:high");
    }

    #[tokio::test]
    async fn test_handle_issue_opened_with_assignees() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = serde_json::json!({
            "action": "opened",
            "issue": {
                "number": 105,
                "title": "Fix bug",
                "body": "Bug description",
                "state": "open",
                "assignees": [
                    {"login": "dev1"},
                    {"login": "dev2"}
                ]
            },
            "repository": {
                "full_name": "owner/repo"
            }
        })
        .to_string();

        let event = WebhookEvent::new(
            "delivery-issue-assignees".to_string(),
            "issues".to_string(),
            payload,
        );

        handle_issue_opened(database.clone(), &event).await.unwrap();

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        let custom = &agent.context.custom;
        let assignees = custom.get("issue_assignees").unwrap().as_array().unwrap();
        assert_eq!(assignees.len(), 2);
        assert_eq!(assignees[0].as_str().unwrap(), "dev1");
        assert_eq!(assignees[1].as_str().unwrap(), "dev2");
    }

    #[tokio::test]
    async fn test_handle_issue_opened_extracts_repository_info() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        let payload = create_issue_opened_payload(
            106,
            "Test issue",
            "Test body",
        );
        let event = WebhookEvent::new(
            "delivery-issue-repo".to_string(),
            "issues".to_string(),
            payload,
        );

        handle_issue_opened(database.clone(), &event).await.unwrap();

        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);

        let agent = &agents[0];
        let custom = &agent.context.custom;
        assert_eq!(custom.get("repository").unwrap().as_str().unwrap(), "owner/repo");
        assert_eq!(
            custom.get("event_delivery_id").unwrap().as_str().unwrap(),
            "delivery-issue-repo"
        );
    }
}
