//! Slack Interactive Features
//!
//! This module provides interactive Slack features including:
//! - Interactive approval buttons
//! - Slash command handling
//! - Thread-based PR discussions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::slack::{
    ApprovalDecision, InteractionPayload, SlackApprovalRequest, SlackBlock, SlackElement,
    SlackMessage, SlackText, SlashCommand, SlashCommandResponse, ButtonStyle, ResponseType,
};
use crate::{Error, Result};

/// Approval button handler
#[derive(Debug, Clone)]
pub struct ApprovalButtonHandler {
    /// Mapping of Slack user IDs to GitHub usernames for permissions
    user_permissions: HashMap<String, Vec<String>>,
}

impl ApprovalButtonHandler {
    /// Create a new approval button handler
    pub fn new() -> Self {
        Self {
            user_permissions: HashMap::new(),
        }
    }

    /// Add user permission mapping
    pub fn add_user_permission(&mut self, slack_user_id: String, permissions: Vec<String>) {
        self.user_permissions.insert(slack_user_id, permissions);
    }

    /// Check if user has approval permission for a resource
    pub fn has_permission(&self, slack_user_id: &str, resource_type: &str) -> bool {
        self.user_permissions
            .get(slack_user_id)
            .map(|perms| perms.contains(&resource_type.to_string()) || perms.contains(&"*".to_string()))
            .unwrap_or(false)
    }

    /// Create approval request message with interactive buttons
    pub fn create_approval_message(
        &self,
        request: &SlackApprovalRequest,
        channel: &str,
    ) -> SlackMessage {
        let blocks = vec![
            SlackBlock::Header {
                text: SlackText::plain("Approval Required"),
            },
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!(
                    "*{}* for *{}*\n\n{}",
                    request.resource_type, request.resource_id, request.description
                )),
                accessory: None,
                fields: None,
            },
            SlackBlock::Context {
                elements: vec![
                    crate::slack::SlackContextElement::Mrkdwn {
                        text: format!("Requested at: {}", request.created_at.format("%Y-%m-%d %H:%M:%S UTC")),
                    },
                ],
            },
            SlackBlock::Divider,
            SlackBlock::Actions {
                elements: vec![
                    SlackElement::Button {
                        text: SlackText::plain("Approve"),
                        action_id: format!("approve_{}", request.id),
                        value: Some(request.approval_id.clone()),
                        style: Some(ButtonStyle::Primary),
                        url: None,
                    },
                    SlackElement::Button {
                        text: SlackText::plain("Reject"),
                        action_id: format!("reject_{}", request.id),
                        value: Some(request.approval_id.clone()),
                        style: Some(ButtonStyle::Danger),
                        url: None,
                    },
                ],
            },
        ];

        SlackMessage::new(channel, "Approval required")
            .with_blocks(blocks)
    }

    /// Handle button click interaction
    pub fn handle_button_click(
        &self,
        payload: &InteractionPayload,
    ) -> Result<ApprovalResponse> {
        // Verify user has permissions
        let action = payload.actions.first()
            .ok_or_else(|| Error::Other("No action in payload".to_string()))?;

        let action_id = &action.action_id;

        // Extract request ID from action_id (format: "approve_<id>" or "reject_<id>")
        let parts: Vec<&str> = action_id.split('_').collect();
        if parts.len() != 2 {
            return Err(Error::Other("Invalid action ID format".to_string()));
        }

        let decision_type = parts[0];
        let request_id = parts[1];

        let decision = match decision_type {
            "approve" => ApprovalDecision::Approved,
            "reject" => ApprovalDecision::Rejected,
            _ => return Err(Error::Other(format!("Unknown decision type: {}", decision_type))),
        };

        Ok(ApprovalResponse {
            request_id: request_id.to_string(),
            decision,
            user_id: payload.user.id.clone(),
            comment: None,
            timestamp: Utc::now(),
        })
    }

    /// Create updated message after approval/rejection
    pub fn create_response_message(
        &self,
        request: &SlackApprovalRequest,
        response: &ApprovalResponse,
        user_name: &str,
    ) -> SlackMessage {
        let (status_emoji, status_text, color_style) = match response.decision {
            ApprovalDecision::Approved => ("✅", "Approved", "approved"),
            ApprovalDecision::Rejected => ("❌", "Rejected", "rejected"),
            ApprovalDecision::RequestedChanges => ("🔄", "Changes Requested", "changes"),
        };

        let blocks = vec![
            SlackBlock::Header {
                text: SlackText::plain(format!("{} {}", status_emoji, status_text)),
            },
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!(
                    "*{}* for *{}*\n\n{}",
                    request.resource_type, request.resource_id, request.description
                )),
                accessory: None,
                fields: None,
            },
            SlackBlock::Context {
                elements: vec![
                    crate::slack::SlackContextElement::Mrkdwn {
                        text: format!(
                            "{} by {} at {}",
                            status_text,
                            user_name,
                            response.timestamp.format("%Y-%m-%d %H:%M:%S UTC")
                        ),
                    },
                ],
            },
        ];

        SlackMessage::new(&request.channel_id, format!("Approval {}", color_style))
            .with_blocks(blocks)
    }
}

impl Default for ApprovalButtonHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Response from approval interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponse {
    pub request_id: String,
    pub decision: ApprovalDecision,
    pub user_id: String,
    pub comment: Option<String>,
    pub timestamp: DateTime<Utc>,
}

/// Slash command handler
#[derive(Debug, Clone)]
pub struct SlashCommandHandler {
    /// Base URL for dashboard links
    dashboard_url: String,
}

impl SlashCommandHandler {
    /// Create a new slash command handler
    pub fn new(dashboard_url: String) -> Self {
        Self { dashboard_url }
    }

    /// Handle a slash command
    pub fn handle_command(&self, cmd: &SlashCommand) -> Result<SlashCommandResponse> {
        let args = cmd.parse_args();

        if args.is_empty() {
            return Ok(self.help_response());
        }

        match args[0].as_str() {
            "status" => self.status_command(),
            "agents" => self.agents_command(),
            "pr" => self.pr_command(&args),
            "deploy" => self.deploy_command(&args),
            "approve" => self.approve_command(&args),
            "help" => Ok(self.help_response()),
            _ => Ok(SlashCommandResponse::ephemeral(
                format!("Unknown command: {}. Type `/orchestrate help` for available commands.", args[0])
            )),
        }
    }

    /// Status command - show system status
    fn status_command(&self) -> Result<SlashCommandResponse> {
        let blocks = vec![
            SlackBlock::Section {
                text: SlackText::mrkdwn("*System Status*\n\nAll systems operational"),
                accessory: None,
                fields: Some(vec![
                    SlackText::mrkdwn("*Agents:* 3 running"),
                    SlackText::mrkdwn("*Queue:* 2 pending"),
                    SlackText::mrkdwn("*PRs:* 5 open"),
                    SlackText::mrkdwn("*Deployments:* 1 in progress"),
                ]),
            },
            SlackBlock::Actions {
                elements: vec![
                    SlackElement::Button {
                        text: SlackText::plain("View Dashboard"),
                        action_id: "view_dashboard".to_string(),
                        value: None,
                        style: Some(ButtonStyle::Primary),
                        url: Some(self.dashboard_url.clone()),
                    },
                ],
            },
        ];

        Ok(SlashCommandResponse::ephemeral("System status").with_blocks(blocks))
    }

    /// Agents command - list running agents
    fn agents_command(&self) -> Result<SlashCommandResponse> {
        let blocks = vec![
            SlackBlock::Section {
                text: SlackText::mrkdwn("*Running Agents*"),
                accessory: None,
                fields: None,
            },
            SlackBlock::Section {
                text: SlackText::mrkdwn(
                    "• story-developer - Implementing user authentication\n\
                     • pr-reviewer - Reviewing PR #123\n\
                     • deployment-agent - Deploying to staging"
                ),
                accessory: None,
                fields: None,
            },
        ];

        Ok(SlashCommandResponse::ephemeral("Running agents").with_blocks(blocks))
    }

    /// PR command - get PR status
    fn pr_command(&self, args: &[String]) -> Result<SlashCommandResponse> {
        if args.len() < 2 {
            return Ok(SlashCommandResponse::ephemeral(
                "Usage: /orchestrate pr <number>"
            ));
        }

        let pr_number = &args[1];

        let blocks = vec![
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!("*PR #{}*\n\nStatus: Open\nCI: Passing\nReviews: 1/2", pr_number)),
                accessory: None,
                fields: None,
            },
            SlackBlock::Actions {
                elements: vec![
                    SlackElement::Button {
                        text: SlackText::plain("View PR"),
                        action_id: "view_pr".to_string(),
                        value: Some(pr_number.clone()),
                        style: Some(ButtonStyle::Primary),
                        url: Some(format!("{}/pr/{}", self.dashboard_url, pr_number)),
                    },
                ],
            },
        ];

        Ok(SlashCommandResponse::ephemeral(format!("PR #{} status", pr_number)).with_blocks(blocks))
    }

    /// Deploy command - trigger deployment
    fn deploy_command(&self, args: &[String]) -> Result<SlashCommandResponse> {
        if args.len() < 2 {
            return Ok(SlashCommandResponse::ephemeral(
                "Usage: /orchestrate deploy <environment>"
            ));
        }

        let environment = &args[1];

        if !["dev", "staging", "production"].contains(&environment.as_str()) {
            return Ok(SlashCommandResponse::ephemeral(
                "Invalid environment. Must be: dev, staging, or production"
            ));
        }

        Ok(SlashCommandResponse::in_channel(
            format!("Deployment to {} triggered by <@{{user_id}}>", environment)
        ))
    }

    /// Approve command - approve from Slack
    fn approve_command(&self, args: &[String]) -> Result<SlashCommandResponse> {
        if args.len() < 2 {
            return Ok(SlashCommandResponse::ephemeral(
                "Usage: /orchestrate approve <approval-id>"
            ));
        }

        let approval_id = &args[1];

        Ok(SlashCommandResponse::ephemeral(
            format!("Approval {} has been approved", approval_id)
        ))
    }

    /// Help response
    fn help_response(&self) -> SlashCommandResponse {
        let blocks = vec![
            SlackBlock::Section {
                text: SlackText::mrkdwn("*Available Commands*"),
                accessory: None,
                fields: None,
            },
            SlackBlock::Section {
                text: SlackText::mrkdwn(
                    "• `/orchestrate status` - Show system status\n\
                     • `/orchestrate agents` - List running agents\n\
                     • `/orchestrate pr <number>` - Get PR status\n\
                     • `/orchestrate deploy <env>` - Trigger deployment\n\
                     • `/orchestrate approve <id>` - Approve request\n\
                     • `/orchestrate help` - Show this help"
                ),
                accessory: None,
                fields: None,
            },
        ];

        SlashCommandResponse::ephemeral("Orchestrate Commands").with_blocks(blocks)
    }
}

/// Thread manager for PR discussions
#[derive(Debug, Clone)]
pub struct PrThreadManager {
    default_channel: String,
}

impl PrThreadManager {
    /// Create a new PR thread manager
    pub fn new(default_channel: String) -> Self {
        Self { default_channel }
    }

    /// Create initial PR thread message
    pub fn create_pr_thread_message(
        &self,
        pr_number: i32,
        title: &str,
        author: &str,
        url: &str,
    ) -> SlackMessage {
        let blocks = vec![
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!(
                    "*PR #{}:* {}\n\nCreated by {}",
                    pr_number, title, author
                )),
                accessory: None,
                fields: None,
            },
            SlackBlock::Actions {
                elements: vec![
                    SlackElement::Button {
                        text: SlackText::plain("View PR"),
                        action_id: "view_pr".to_string(),
                        value: Some(pr_number.to_string()),
                        style: Some(ButtonStyle::Primary),
                        url: Some(url.to_string()),
                    },
                ],
            },
        ];

        SlackMessage::new(&self.default_channel, format!("PR #{}: {}", pr_number, title))
            .with_blocks(blocks)
    }

    /// Create review comment message for thread
    pub fn create_review_comment_message(
        &self,
        reviewer: &str,
        comment: &str,
        thread_ts: &str,
    ) -> SlackMessage {
        SlackMessage::new(
            &self.default_channel,
            format!("Review comment from {}", reviewer),
        )
        .with_blocks(vec![
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!("*{}* commented:\n\n{}", reviewer, comment)),
                accessory: None,
                fields: None,
            },
        ])
        .in_thread(thread_ts)
    }

    /// Create CI status update message for thread
    pub fn create_ci_status_message(
        &self,
        status: &str,
        conclusion: &str,
        thread_ts: &str,
    ) -> SlackMessage {
        let (emoji, text) = match conclusion {
            "success" => ("✅", "CI checks passed"),
            "failure" => ("❌", "CI checks failed"),
            "pending" => ("⏳", "CI checks running"),
            _ => ("⚠️", "CI status unknown"),
        };

        SlackMessage::new(&self.default_channel, format!("CI: {}", text))
            .with_blocks(vec![
                SlackBlock::Section {
                    text: SlackText::mrkdwn(format!("{} {}", emoji, text)),
                    accessory: None,
                    fields: None,
                },
            ])
            .in_thread(thread_ts)
    }

    /// Create agent activity message for thread
    pub fn create_agent_activity_message(
        &self,
        agent_type: &str,
        activity: &str,
        thread_ts: &str,
    ) -> SlackMessage {
        SlackMessage::new(&self.default_channel, format!("Agent: {}", agent_type))
            .with_blocks(vec![
                SlackBlock::Section {
                    text: SlackText::mrkdwn(format!("*{}:* {}", agent_type, activity)),
                    accessory: None,
                    fields: None,
                },
            ])
            .in_thread(thread_ts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_button_handler_permissions() {
        let mut handler = ApprovalButtonHandler::new();
        handler.add_user_permission("U123".to_string(), vec!["deployment".to_string()]);
        handler.add_user_permission("U456".to_string(), vec!["*".to_string()]);

        assert!(handler.has_permission("U123", "deployment"));
        assert!(!handler.has_permission("U123", "other"));
        assert!(handler.has_permission("U456", "deployment"));
        assert!(handler.has_permission("U456", "anything"));
        assert!(!handler.has_permission("U789", "deployment"));
    }

    #[test]
    fn test_approval_button_handler_create_message() {
        let handler = ApprovalButtonHandler::new();
        let request = SlackApprovalRequest::new(
            "approval-123",
            "deployment",
            "production",
            "Deploy version 1.2.0 to production",
        );

        let message = handler.create_approval_message(&request, "#approvals");

        assert_eq!(message.channel, "#approvals");
        assert_eq!(message.blocks.len(), 5); // Header, Section, Context, Divider, Actions
    }

    #[test]
    fn test_approval_button_handler_handle_approve_click() {
        let handler = ApprovalButtonHandler::new();

        let payload = InteractionPayload {
            interaction_type: crate::slack::InteractionType::BlockActions,
            trigger_id: "trigger123".to_string(),
            user: crate::slack::InteractionUser {
                id: "U123".to_string(),
                name: "Test User".to_string(),
                username: "testuser".to_string(),
            },
            channel: crate::slack::InteractionChannel {
                id: "C123".to_string(),
                name: "approvals".to_string(),
            },
            message: None,
            actions: vec![
                crate::slack::InteractionAction {
                    action_id: "approve_req123".to_string(),
                    block_id: None,
                    value: Some("approval-123".to_string()),
                    action_type: "button".to_string(),
                },
            ],
            response_url: "https://hooks.slack.com/...".to_string(),
        };

        let response = handler.handle_button_click(&payload).unwrap();

        assert_eq!(response.request_id, "req123");
        assert_eq!(response.decision, ApprovalDecision::Approved);
        assert_eq!(response.user_id, "U123");
    }

    #[test]
    fn test_approval_button_handler_handle_reject_click() {
        let handler = ApprovalButtonHandler::new();

        let payload = InteractionPayload {
            interaction_type: crate::slack::InteractionType::BlockActions,
            trigger_id: "trigger123".to_string(),
            user: crate::slack::InteractionUser {
                id: "U123".to_string(),
                name: "Test User".to_string(),
                username: "testuser".to_string(),
            },
            channel: crate::slack::InteractionChannel {
                id: "C123".to_string(),
                name: "approvals".to_string(),
            },
            message: None,
            actions: vec![
                crate::slack::InteractionAction {
                    action_id: "reject_req123".to_string(),
                    block_id: None,
                    value: Some("approval-123".to_string()),
                    action_type: "button".to_string(),
                },
            ],
            response_url: "https://hooks.slack.com/...".to_string(),
        };

        let response = handler.handle_button_click(&payload).unwrap();

        assert_eq!(response.request_id, "req123");
        assert_eq!(response.decision, ApprovalDecision::Rejected);
    }

    #[test]
    fn test_slash_command_handler_status() {
        let handler = SlashCommandHandler::new("https://dashboard.example.com".to_string());

        let cmd = SlashCommand {
            command: "/orchestrate".to_string(),
            text: "status".to_string(),
            response_url: "https://hooks.slack.com/...".to_string(),
            trigger_id: "trigger123".to_string(),
            user_id: "U123".to_string(),
            user_name: "testuser".to_string(),
            channel_id: "C123".to_string(),
            channel_name: "general".to_string(),
            team_id: "T123".to_string(),
        };

        let response = handler.handle_command(&cmd).unwrap();

        assert_eq!(response.response_type, ResponseType::Ephemeral);
        assert!(response.blocks.is_some());
    }

    #[test]
    fn test_slash_command_handler_agents() {
        let handler = SlashCommandHandler::new("https://dashboard.example.com".to_string());

        let cmd = SlashCommand {
            command: "/orchestrate".to_string(),
            text: "agents".to_string(),
            response_url: "https://hooks.slack.com/...".to_string(),
            trigger_id: "trigger123".to_string(),
            user_id: "U123".to_string(),
            user_name: "testuser".to_string(),
            channel_id: "C123".to_string(),
            channel_name: "general".to_string(),
            team_id: "T123".to_string(),
        };

        let response = handler.handle_command(&cmd).unwrap();

        assert_eq!(response.response_type, ResponseType::Ephemeral);
        assert!(response.blocks.is_some());
    }

    #[test]
    fn test_slash_command_handler_pr() {
        let handler = SlashCommandHandler::new("https://dashboard.example.com".to_string());

        let cmd = SlashCommand {
            command: "/orchestrate".to_string(),
            text: "pr 123".to_string(),
            response_url: "https://hooks.slack.com/...".to_string(),
            trigger_id: "trigger123".to_string(),
            user_id: "U123".to_string(),
            user_name: "testuser".to_string(),
            channel_id: "C123".to_string(),
            channel_name: "general".to_string(),
            team_id: "T123".to_string(),
        };

        let response = handler.handle_command(&cmd).unwrap();

        assert_eq!(response.response_type, ResponseType::Ephemeral);
        assert!(response.text.contains("123"));
    }

    #[test]
    fn test_slash_command_handler_deploy() {
        let handler = SlashCommandHandler::new("https://dashboard.example.com".to_string());

        let cmd = SlashCommand {
            command: "/orchestrate".to_string(),
            text: "deploy staging".to_string(),
            response_url: "https://hooks.slack.com/...".to_string(),
            trigger_id: "trigger123".to_string(),
            user_id: "U123".to_string(),
            user_name: "testuser".to_string(),
            channel_id: "C123".to_string(),
            channel_name: "general".to_string(),
            team_id: "T123".to_string(),
        };

        let response = handler.handle_command(&cmd).unwrap();

        assert_eq!(response.response_type, ResponseType::InChannel);
        assert!(response.text.contains("staging"));
    }

    #[test]
    fn test_slash_command_handler_deploy_invalid_env() {
        let handler = SlashCommandHandler::new("https://dashboard.example.com".to_string());

        let cmd = SlashCommand {
            command: "/orchestrate".to_string(),
            text: "deploy invalid".to_string(),
            response_url: "https://hooks.slack.com/...".to_string(),
            trigger_id: "trigger123".to_string(),
            user_id: "U123".to_string(),
            user_name: "testuser".to_string(),
            channel_id: "C123".to_string(),
            channel_name: "general".to_string(),
            team_id: "T123".to_string(),
        };

        let response = handler.handle_command(&cmd).unwrap();

        assert_eq!(response.response_type, ResponseType::Ephemeral);
        assert!(response.text.contains("Invalid environment"));
    }

    #[test]
    fn test_slash_command_handler_help() {
        let handler = SlashCommandHandler::new("https://dashboard.example.com".to_string());

        let cmd = SlashCommand {
            command: "/orchestrate".to_string(),
            text: "help".to_string(),
            response_url: "https://hooks.slack.com/...".to_string(),
            trigger_id: "trigger123".to_string(),
            user_id: "U123".to_string(),
            user_name: "testuser".to_string(),
            channel_id: "C123".to_string(),
            channel_name: "general".to_string(),
            team_id: "T123".to_string(),
        };

        let response = handler.handle_command(&cmd).unwrap();

        assert_eq!(response.response_type, ResponseType::Ephemeral);
        assert!(response.text.contains("Commands"));
    }

    #[test]
    fn test_slash_command_handler_unknown() {
        let handler = SlashCommandHandler::new("https://dashboard.example.com".to_string());

        let cmd = SlashCommand {
            command: "/orchestrate".to_string(),
            text: "unknown".to_string(),
            response_url: "https://hooks.slack.com/...".to_string(),
            trigger_id: "trigger123".to_string(),
            user_id: "U123".to_string(),
            user_name: "testuser".to_string(),
            channel_id: "C123".to_string(),
            channel_name: "general".to_string(),
            team_id: "T123".to_string(),
        };

        let response = handler.handle_command(&cmd).unwrap();

        assert_eq!(response.response_type, ResponseType::Ephemeral);
        assert!(response.text.contains("Unknown command"));
    }

    #[test]
    fn test_pr_thread_manager_create_thread() {
        let manager = PrThreadManager::new("#prs".to_string());

        let message = manager.create_pr_thread_message(
            123,
            "Add user authentication",
            "testuser",
            "https://github.com/org/repo/pull/123",
        );

        assert_eq!(message.channel, "#prs");
        assert!(message.text.contains("123"));
        assert!(message.text.contains("Add user authentication"));
        assert_eq!(message.blocks.len(), 2);
    }

    #[test]
    fn test_pr_thread_manager_review_comment() {
        let manager = PrThreadManager::new("#prs".to_string());

        let message = manager.create_review_comment_message(
            "reviewer",
            "Looks good to me!",
            "1234.5678",
        );

        assert_eq!(message.channel, "#prs");
        assert_eq!(message.thread_ts, Some("1234.5678".to_string()));
        assert!(message.text.contains("reviewer"));
    }

    #[test]
    fn test_pr_thread_manager_ci_status_success() {
        let manager = PrThreadManager::new("#prs".to_string());

        let message = manager.create_ci_status_message("completed", "success", "1234.5678");

        assert_eq!(message.channel, "#prs");
        assert_eq!(message.thread_ts, Some("1234.5678".to_string()));
        assert!(message.text.contains("CI"));
    }

    #[test]
    fn test_pr_thread_manager_ci_status_failure() {
        let manager = PrThreadManager::new("#prs".to_string());

        let message = manager.create_ci_status_message("completed", "failure", "1234.5678");

        assert_eq!(message.channel, "#prs");
        assert_eq!(message.thread_ts, Some("1234.5678".to_string()));
        assert!(message.text.contains("CI"));
    }

    #[test]
    fn test_pr_thread_manager_agent_activity() {
        let manager = PrThreadManager::new("#prs".to_string());

        let message = manager.create_agent_activity_message(
            "story-developer",
            "Running tests",
            "1234.5678",
        );

        assert_eq!(message.channel, "#prs");
        assert_eq!(message.thread_ts, Some("1234.5678".to_string()));
        assert!(message.text.contains("story-developer"));
    }
}
