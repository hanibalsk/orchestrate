//! Slack Integration Module
//!
//! This module provides Slack integration capabilities including:
//! - OAuth connection management
//! - Rich message notifications with blocks
//! - Interactive buttons and actions
//! - Slash command handling
//! - Thread-based conversations
//! - User mapping (GitHub <-> Slack)

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Slack workspace connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConnection {
    pub id: String,
    pub team_id: String,
    pub team_name: String,
    pub bot_token: String,
    pub bot_user_id: String,
    pub app_id: String,
    pub connected_at: DateTime<Utc>,
    pub connected_by: String,
    pub is_active: bool,
    pub scopes: Vec<String>,
}

impl SlackConnection {
    pub fn new(
        team_id: impl Into<String>,
        team_name: impl Into<String>,
        bot_token: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            team_id: team_id.into(),
            team_name: team_name.into(),
            bot_token: bot_token.into(),
            bot_user_id: String::new(),
            app_id: String::new(),
            connected_at: Utc::now(),
            connected_by: String::new(),
            is_active: true,
            scopes: Vec::new(),
        }
    }

    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope)
    }
}

/// Notification type for routing to channels
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    AgentStarted,
    AgentCompleted,
    AgentFailed,
    PrCreated,
    PrReviewRequested,
    PrCommented,
    PrMerged,
    PrClosed,
    CiPassed,
    CiFailed,
    DeploymentStarted,
    DeploymentCompleted,
    DeploymentFailed,
    ApprovalRequired,
    AlertFired,
    Custom(String),
}

impl std::fmt::Display for NotificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AgentStarted => write!(f, "agent_started"),
            Self::AgentCompleted => write!(f, "agent_completed"),
            Self::AgentFailed => write!(f, "agent_failed"),
            Self::PrCreated => write!(f, "pr_created"),
            Self::PrReviewRequested => write!(f, "pr_review_requested"),
            Self::PrCommented => write!(f, "pr_commented"),
            Self::PrMerged => write!(f, "pr_merged"),
            Self::PrClosed => write!(f, "pr_closed"),
            Self::CiPassed => write!(f, "ci_passed"),
            Self::CiFailed => write!(f, "ci_failed"),
            Self::DeploymentStarted => write!(f, "deployment_started"),
            Self::DeploymentCompleted => write!(f, "deployment_completed"),
            Self::DeploymentFailed => write!(f, "deployment_failed"),
            Self::ApprovalRequired => write!(f, "approval_required"),
            Self::AlertFired => write!(f, "alert_fired"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

/// Channel configuration for notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub default_channel: String,
    pub channel_mappings: HashMap<NotificationType, String>,
}

impl ChannelConfig {
    pub fn new(default_channel: impl Into<String>) -> Self {
        Self {
            default_channel: default_channel.into(),
            channel_mappings: HashMap::new(),
        }
    }

    pub fn with_mapping(mut self, notification_type: NotificationType, channel: impl Into<String>) -> Self {
        self.channel_mappings.insert(notification_type, channel.into());
        self
    }

    pub fn get_channel(&self, notification_type: &NotificationType) -> &str {
        self.channel_mappings
            .get(notification_type)
            .unwrap_or(&self.default_channel)
    }
}

impl Default for ChannelConfig {
    fn default() -> Self {
        Self::new("#orchestrate")
    }
}

/// Slack message block types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackBlock {
    Section {
        text: SlackText,
        accessory: Option<Box<SlackBlock>>,
        fields: Option<Vec<SlackText>>,
    },
    Divider,
    Actions {
        elements: Vec<SlackElement>,
    },
    Context {
        elements: Vec<SlackContextElement>,
    },
    Header {
        text: SlackText,
    },
    Image {
        image_url: String,
        alt_text: String,
    },
}

/// Slack text object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackText {
    #[serde(rename = "type")]
    pub text_type: TextType,
    pub text: String,
    pub emoji: Option<bool>,
}

impl SlackText {
    pub fn mrkdwn(text: impl Into<String>) -> Self {
        Self {
            text_type: TextType::Mrkdwn,
            text: text.into(),
            emoji: None,
        }
    }

    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text_type: TextType::PlainText,
            text: text.into(),
            emoji: Some(true),
        }
    }
}

/// Text type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TextType {
    PlainText,
    Mrkdwn,
}

/// Interactive elements
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackElement {
    Button {
        text: SlackText,
        action_id: String,
        value: Option<String>,
        style: Option<ButtonStyle>,
        url: Option<String>,
    },
    StaticSelect {
        action_id: String,
        placeholder: SlackText,
        options: Vec<SlackOption>,
    },
}

/// Button style
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ButtonStyle {
    Primary,
    Danger,
}

/// Select option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackOption {
    pub text: SlackText,
    pub value: String,
}

/// Context element
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SlackContextElement {
    Mrkdwn { text: String },
    PlainText { text: String, emoji: Option<bool> },
    Image { image_url: String, alt_text: String },
}

/// A Slack message to send
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    pub channel: String,
    pub text: String,
    pub blocks: Vec<SlackBlock>,
    pub thread_ts: Option<String>,
    pub reply_broadcast: Option<bool>,
    pub unfurl_links: Option<bool>,
    pub unfurl_media: Option<bool>,
}

impl SlackMessage {
    pub fn new(channel: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            channel: channel.into(),
            text: text.into(),
            blocks: Vec::new(),
            thread_ts: None,
            reply_broadcast: None,
            unfurl_links: Some(false),
            unfurl_media: Some(true),
        }
    }

    pub fn with_blocks(mut self, blocks: Vec<SlackBlock>) -> Self {
        self.blocks = blocks;
        self
    }

    pub fn in_thread(mut self, thread_ts: impl Into<String>) -> Self {
        self.thread_ts = Some(thread_ts.into());
        self
    }

    pub fn broadcast_reply(mut self) -> Self {
        self.reply_broadcast = Some(true);
        self
    }
}

/// Sent message response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentMessage {
    pub ok: bool,
    pub channel: String,
    pub ts: String,
    pub message_id: String,
}

/// Slash command request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    pub command: String,
    pub text: String,
    pub response_url: String,
    pub trigger_id: String,
    pub user_id: String,
    pub user_name: String,
    pub channel_id: String,
    pub channel_name: String,
    pub team_id: String,
}

impl SlashCommand {
    pub fn parse_args(&self) -> Vec<String> {
        self.text
            .split_whitespace()
            .map(|s| s.to_string())
            .collect()
    }
}

/// Slash command response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommandResponse {
    pub response_type: ResponseType,
    pub text: String,
    pub blocks: Option<Vec<SlackBlock>>,
}

impl SlashCommandResponse {
    pub fn ephemeral(text: impl Into<String>) -> Self {
        Self {
            response_type: ResponseType::Ephemeral,
            text: text.into(),
            blocks: None,
        }
    }

    pub fn in_channel(text: impl Into<String>) -> Self {
        Self {
            response_type: ResponseType::InChannel,
            text: text.into(),
            blocks: None,
        }
    }

    pub fn with_blocks(mut self, blocks: Vec<SlackBlock>) -> Self {
        self.blocks = Some(blocks);
        self
    }
}

/// Response type for slash commands
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ResponseType {
    Ephemeral,
    InChannel,
}

/// Interactive action payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionPayload {
    #[serde(rename = "type")]
    pub interaction_type: InteractionType,
    pub trigger_id: String,
    pub user: InteractionUser,
    pub channel: InteractionChannel,
    pub message: Option<InteractionMessage>,
    pub actions: Vec<InteractionAction>,
    pub response_url: String,
}

/// Interaction type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InteractionType {
    BlockActions,
    ViewSubmission,
    Shortcut,
}

/// User in interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionUser {
    pub id: String,
    pub name: String,
    pub username: String,
}

/// Channel in interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionChannel {
    pub id: String,
    pub name: String,
}

/// Message in interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionMessage {
    pub ts: String,
    pub text: String,
}

/// Action in interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractionAction {
    pub action_id: String,
    pub block_id: Option<String>,
    pub value: Option<String>,
    #[serde(rename = "type")]
    pub action_type: String,
}

/// User mapping between GitHub and Slack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMapping {
    pub id: String,
    pub github_username: String,
    pub slack_user_id: String,
    pub slack_username: String,
    pub notify_on_pr: bool,
    pub notify_on_mention: bool,
    pub notify_on_failure: bool,
    pub created_at: DateTime<Utc>,
}

impl UserMapping {
    pub fn new(
        github_username: impl Into<String>,
        slack_user_id: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            github_username: github_username.into(),
            slack_user_id: slack_user_id.into(),
            slack_username: String::new(),
            notify_on_pr: true,
            notify_on_mention: true,
            notify_on_failure: true,
            created_at: Utc::now(),
        }
    }
}

/// Thread tracking for PRs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrThread {
    pub id: String,
    pub pr_number: i32,
    pub channel_id: String,
    pub thread_ts: String,
    pub created_at: DateTime<Utc>,
    pub last_updated: DateTime<Utc>,
    pub is_archived: bool,
}

impl PrThread {
    pub fn new(
        pr_number: i32,
        channel_id: impl Into<String>,
        thread_ts: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pr_number,
            channel_id: channel_id.into(),
            thread_ts: thread_ts.into(),
            created_at: now,
            last_updated: now,
            is_archived: false,
        }
    }

    pub fn archive(&mut self) {
        self.is_archived = true;
        self.last_updated = Utc::now();
    }
}

/// Notification settings per user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    pub user_id: String,
    pub enabled_types: Vec<NotificationType>,
    pub muted_until: Option<DateTime<Utc>>,
    pub dm_for_urgent: bool,
    pub digest_mode: DigestMode,
}

impl NotificationSettings {
    pub fn new(user_id: impl Into<String>) -> Self {
        Self {
            user_id: user_id.into(),
            enabled_types: vec![
                NotificationType::AgentCompleted,
                NotificationType::AgentFailed,
                NotificationType::PrCreated,
                NotificationType::PrMerged,
                NotificationType::ApprovalRequired,
            ],
            muted_until: None,
            dm_for_urgent: true,
            digest_mode: DigestMode::Instant,
        }
    }

    pub fn is_enabled(&self, notification_type: &NotificationType) -> bool {
        if let Some(until) = self.muted_until {
            if Utc::now() < until {
                return false;
            }
        }
        self.enabled_types.contains(notification_type)
    }

    pub fn mute(&mut self, until: DateTime<Utc>) {
        self.muted_until = Some(until);
    }

    pub fn unmute(&mut self) {
        self.muted_until = None;
    }
}

/// Digest mode for batching notifications
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DigestMode {
    Instant,
    Hourly,
    Daily,
}

/// Notification template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationTemplate {
    pub id: String,
    pub name: String,
    pub notification_type: NotificationType,
    pub template_blocks: String,
    pub fallback_text: String,
}

impl NotificationTemplate {
    pub fn new(
        name: impl Into<String>,
        notification_type: NotificationType,
        fallback_text: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            notification_type,
            template_blocks: String::new(),
            fallback_text: fallback_text.into(),
        }
    }
}

/// Approval request for interactive buttons
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackApprovalRequest {
    pub id: String,
    pub approval_id: String,
    pub channel_id: String,
    pub message_ts: String,
    pub requester_slack_id: String,
    pub resource_type: String,
    pub resource_id: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
    pub responder_slack_id: Option<String>,
    pub decision: Option<ApprovalDecision>,
}

impl SlackApprovalRequest {
    pub fn new(
        approval_id: impl Into<String>,
        resource_type: impl Into<String>,
        resource_id: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            approval_id: approval_id.into(),
            channel_id: String::new(),
            message_ts: String::new(),
            requester_slack_id: String::new(),
            resource_type: resource_type.into(),
            resource_id: resource_id.into(),
            description: description.into(),
            created_at: Utc::now(),
            responded_at: None,
            responder_slack_id: None,
            decision: None,
        }
    }

    pub fn respond(&mut self, responder_id: impl Into<String>, decision: ApprovalDecision) {
        self.responded_at = Some(Utc::now());
        self.responder_slack_id = Some(responder_id.into());
        self.decision = Some(decision);
    }

    pub fn is_pending(&self) -> bool {
        self.decision.is_none()
    }
}

/// Approval decision
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approved,
    Rejected,
    RequestedChanges,
}

/// Build common message templates
pub mod templates {
    use super::*;

    pub fn agent_completed_message(
        agent_type: &str,
        task: &str,
        duration: &str,
        tokens: u64,
        dashboard_url: &str,
    ) -> SlackMessage {
        let blocks = vec![
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!("✅ Agent *{}* completed task", agent_type)),
                accessory: None,
                fields: None,
            },
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!("*Task:* {}", task)),
                accessory: None,
                fields: None,
            },
            SlackBlock::Context {
                elements: vec![
                    SlackContextElement::Mrkdwn {
                        text: format!("Duration: {} | Tokens: {}", duration, tokens),
                    },
                ],
            },
            SlackBlock::Actions {
                elements: vec![
                    SlackElement::Button {
                        text: SlackText::plain("View Details"),
                        action_id: "view_agent".to_string(),
                        value: None,
                        style: None,
                        url: Some(dashboard_url.to_string()),
                    },
                ],
            },
        ];

        SlackMessage::new("#orchestrate", format!("Agent {} completed", agent_type))
            .with_blocks(blocks)
    }

    pub fn approval_request_message(
        resource_type: &str,
        resource_id: &str,
        description: &str,
        requester: &str,
    ) -> SlackMessage {
        let blocks = vec![
            SlackBlock::Header {
                text: SlackText::plain("⚠️ Approval Required"),
            },
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!(
                    "{} to *{}* is pending approval.\n\n{}",
                    resource_type, resource_id, description
                )),
                accessory: None,
                fields: None,
            },
            SlackBlock::Context {
                elements: vec![
                    SlackContextElement::Mrkdwn {
                        text: format!("Requested by: <@{}>", requester),
                    },
                ],
            },
            SlackBlock::Divider,
            SlackBlock::Actions {
                elements: vec![
                    SlackElement::Button {
                        text: SlackText::plain("Approve ✓"),
                        action_id: "approve".to_string(),
                        value: Some(resource_id.to_string()),
                        style: Some(ButtonStyle::Primary),
                        url: None,
                    },
                    SlackElement::Button {
                        text: SlackText::plain("Reject ✗"),
                        action_id: "reject".to_string(),
                        value: Some(resource_id.to_string()),
                        style: Some(ButtonStyle::Danger),
                        url: None,
                    },
                ],
            },
        ];

        SlackMessage::new(
            "#approvals",
            format!("Approval required for {} {}", resource_type, resource_id),
        )
        .with_blocks(blocks)
    }

    pub fn pr_created_message(
        pr_number: i32,
        title: &str,
        branch: &str,
        target: &str,
        files_changed: u32,
        additions: u32,
        deletions: u32,
        pr_url: &str,
    ) -> SlackMessage {
        let blocks = vec![
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!(
                    "🔀 *New PR Created*\n\n*#{}:* {}",
                    pr_number, title
                )),
                accessory: None,
                fields: None,
            },
            SlackBlock::Section {
                text: SlackText::mrkdwn(format!(
                    "*Branch:* {} → {}\n*Files:* {} changed (+{} / -{})",
                    branch, target, files_changed, additions, deletions
                )),
                accessory: None,
                fields: None,
            },
            SlackBlock::Context {
                elements: vec![
                    SlackContextElement::Mrkdwn {
                        text: "Status: ⏳ Waiting for CI".to_string(),
                    },
                ],
            },
            SlackBlock::Actions {
                elements: vec![
                    SlackElement::Button {
                        text: SlackText::plain("View PR"),
                        action_id: "view_pr".to_string(),
                        value: Some(pr_number.to_string()),
                        style: Some(ButtonStyle::Primary),
                        url: Some(pr_url.to_string()),
                    },
                ],
            },
        ];

        SlackMessage::new("#prs", format!("New PR #{}: {}", pr_number, title))
            .with_blocks(blocks)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_connection() {
        let conn = SlackConnection::new("T12345", "Test Team", "xoxb-token")
            .with_scopes(vec!["chat:write".to_string(), "commands".to_string()]);

        assert_eq!(conn.team_id, "T12345");
        assert!(conn.has_scope("chat:write"));
        assert!(!conn.has_scope("admin"));
    }

    #[test]
    fn test_channel_config() {
        let config = ChannelConfig::new("#orchestrate")
            .with_mapping(NotificationType::DeploymentStarted, "#deployments")
            .with_mapping(NotificationType::AlertFired, "#alerts");

        assert_eq!(config.get_channel(&NotificationType::DeploymentStarted), "#deployments");
        assert_eq!(config.get_channel(&NotificationType::AlertFired), "#alerts");
        assert_eq!(config.get_channel(&NotificationType::AgentCompleted), "#orchestrate");
    }

    #[test]
    fn test_slack_message_builder() {
        let message = SlackMessage::new("#test", "Test message")
            .with_blocks(vec![SlackBlock::Divider])
            .in_thread("1234.5678")
            .broadcast_reply();

        assert_eq!(message.channel, "#test");
        assert_eq!(message.thread_ts, Some("1234.5678".to_string()));
        assert_eq!(message.reply_broadcast, Some(true));
        assert_eq!(message.blocks.len(), 1);
    }

    #[test]
    fn test_slash_command_parsing() {
        let cmd = SlashCommand {
            command: "/orchestrate".to_string(),
            text: "status --verbose".to_string(),
            response_url: "https://hooks.slack.com/...".to_string(),
            trigger_id: "trigger123".to_string(),
            user_id: "U12345".to_string(),
            user_name: "testuser".to_string(),
            channel_id: "C12345".to_string(),
            channel_name: "general".to_string(),
            team_id: "T12345".to_string(),
        };

        let args = cmd.parse_args();
        assert_eq!(args, vec!["status", "--verbose"]);
    }

    #[test]
    fn test_slash_command_response() {
        let response = SlashCommandResponse::ephemeral("Only you can see this")
            .with_blocks(vec![SlackBlock::Divider]);

        assert_eq!(response.response_type, ResponseType::Ephemeral);
        assert!(response.blocks.is_some());
    }

    #[test]
    fn test_user_mapping() {
        let mapping = UserMapping::new("github-user", "U12345");

        assert_eq!(mapping.github_username, "github-user");
        assert_eq!(mapping.slack_user_id, "U12345");
        assert!(mapping.notify_on_pr);
    }

    #[test]
    fn test_pr_thread() {
        let mut thread = PrThread::new(123, "C12345", "1234.5678");

        assert_eq!(thread.pr_number, 123);
        assert!(!thread.is_archived);

        thread.archive();
        assert!(thread.is_archived);
    }

    #[test]
    fn test_notification_settings() {
        let mut settings = NotificationSettings::new("user123");

        assert!(settings.is_enabled(&NotificationType::AgentCompleted));
        assert!(!settings.is_enabled(&NotificationType::CiPassed));

        let until = Utc::now() + chrono::Duration::hours(1);
        settings.mute(until);
        assert!(!settings.is_enabled(&NotificationType::AgentCompleted));

        settings.unmute();
        assert!(settings.is_enabled(&NotificationType::AgentCompleted));
    }

    #[test]
    fn test_approval_request() {
        let mut request = SlackApprovalRequest::new(
            "approval-123",
            "deployment",
            "production",
            "Deploy version 1.2.0",
        );

        assert!(request.is_pending());

        request.respond("U12345", ApprovalDecision::Approved);
        assert!(!request.is_pending());
        assert_eq!(request.decision, Some(ApprovalDecision::Approved));
    }

    #[test]
    fn test_agent_completed_template() {
        let message = templates::agent_completed_message(
            "story-developer",
            "Implement login",
            "15m 32s",
            45230,
            "https://example.com/agent/123",
        );

        assert!(message.text.contains("story-developer"));
        assert!(!message.blocks.is_empty());
    }

    #[test]
    fn test_pr_created_template() {
        let message = templates::pr_created_message(
            123,
            "Add user authentication",
            "feature/auth",
            "main",
            12,
            450,
            120,
            "https://github.com/org/repo/pull/123",
        );

        assert!(message.text.contains("#123"));
        assert_eq!(message.channel, "#prs");
    }
}
