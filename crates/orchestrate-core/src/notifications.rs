//! Notification Channels for Alerting
//!
//! This module provides notification channel integrations for sending alerts:
//! - Slack webhook integration
//! - Email (SMTP) integration
//! - PagerDuty integration
//! - Generic webhook integration
//! - Message templates per channel
//! - Rate limiting to prevent spam

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

use crate::alerting::{Alert, AlertRule, AlertSeverity};

/// Notification channel configuration error
#[derive(Debug, Error)]
pub enum NotificationError {
    #[error("Invalid channel configuration: {0}")]
    InvalidConfiguration(String),

    #[error("Rate limit exceeded for channel: {0}")]
    RateLimitExceeded(String),

    #[error("Failed to send notification: {0}")]
    SendError(String),

    #[error("Template error: {0}")]
    TemplateError(String),

    #[error("Channel not found: {0}")]
    ChannelNotFound(String),
}

/// Result type for notification operations
pub type Result<T> = std::result::Result<T, NotificationError>;

/// Notification channel type
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChannelType {
    Slack,
    Email,
    PagerDuty,
    Webhook,
}

impl std::fmt::Display for ChannelType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Slack => write!(f, "slack"),
            Self::Email => write!(f, "email"),
            Self::PagerDuty => write!(f, "pagerduty"),
            Self::Webhook => write!(f, "webhook"),
        }
    }
}

impl std::str::FromStr for ChannelType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "slack" => Ok(Self::Slack),
            "email" => Ok(Self::Email),
            "pagerduty" => Ok(Self::PagerDuty),
            "webhook" => Ok(Self::Webhook),
            _ => Err(format!("Invalid channel type: {}", s)),
        }
    }
}

/// Slack webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    pub webhook_url: String,
    pub username: Option<String>,
    pub channel: Option<String>,
    pub icon_emoji: Option<String>,
}

impl SlackConfig {
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            webhook_url: webhook_url.into(),
            username: None,
            channel: None,
            icon_emoji: None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.webhook_url.is_empty() {
            return Err(NotificationError::InvalidConfiguration(
                "Slack webhook URL cannot be empty".to_string(),
            ));
        }
        if !self.webhook_url.starts_with("https://hooks.slack.com/") {
            return Err(NotificationError::InvalidConfiguration(
                "Invalid Slack webhook URL format".to_string(),
            ));
        }
        Ok(())
    }
}

/// Email (SMTP) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    pub smtp_host: String,
    pub smtp_port: u16,
    pub smtp_username: String,
    pub smtp_password: String,
    pub from_address: String,
    pub to_addresses: Vec<String>,
    pub use_tls: bool,
}

impl EmailConfig {
    pub fn new(
        smtp_host: impl Into<String>,
        smtp_port: u16,
        smtp_username: impl Into<String>,
        smtp_password: impl Into<String>,
        from_address: impl Into<String>,
        to_addresses: Vec<String>,
    ) -> Self {
        Self {
            smtp_host: smtp_host.into(),
            smtp_port,
            smtp_username: smtp_username.into(),
            smtp_password: smtp_password.into(),
            from_address: from_address.into(),
            to_addresses,
            use_tls: true,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.smtp_host.is_empty() {
            return Err(NotificationError::InvalidConfiguration(
                "SMTP host cannot be empty".to_string(),
            ));
        }
        if self.smtp_port == 0 {
            return Err(NotificationError::InvalidConfiguration(
                "SMTP port must be greater than 0".to_string(),
            ));
        }
        if self.from_address.is_empty() {
            return Err(NotificationError::InvalidConfiguration(
                "From address cannot be empty".to_string(),
            ));
        }
        if self.to_addresses.is_empty() {
            return Err(NotificationError::InvalidConfiguration(
                "At least one recipient is required".to_string(),
            ));
        }
        Ok(())
    }
}

/// PagerDuty configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PagerDutyConfig {
    pub integration_key: String,
    pub severity_mapping: HashMap<String, String>,
}

impl PagerDutyConfig {
    pub fn new(integration_key: impl Into<String>) -> Self {
        let mut severity_mapping = HashMap::new();
        severity_mapping.insert("critical".to_string(), "critical".to_string());
        severity_mapping.insert("warning".to_string(), "warning".to_string());
        severity_mapping.insert("info".to_string(), "info".to_string());

        Self {
            integration_key: integration_key.into(),
            severity_mapping,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.integration_key.is_empty() {
            return Err(NotificationError::InvalidConfiguration(
                "PagerDuty integration key cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

/// Generic webhook notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationWebhookConfig {
    pub url: String,
    pub method: String,
    pub headers: HashMap<String, String>,
    pub template: Option<String>,
}

impl NotificationWebhookConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            method: "POST".to_string(),
            headers: HashMap::new(),
            template: None,
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.url.is_empty() {
            return Err(NotificationError::InvalidConfiguration(
                "Webhook URL cannot be empty".to_string(),
            ));
        }
        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(NotificationError::InvalidConfiguration(
                "Webhook URL must start with http:// or https://".to_string(),
            ));
        }
        if self.method != "POST" && self.method != "PUT" {
            return Err(NotificationError::InvalidConfiguration(
                "Webhook method must be POST or PUT".to_string(),
            ));
        }
        Ok(())
    }
}

/// Notification channel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelConfig {
    pub id: Option<i64>,
    pub name: String,
    pub channel_type: ChannelType,
    pub enabled: bool,
    pub rate_limit_per_hour: i64,
    pub config: serde_json::Value,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl ChannelConfig {
    pub fn new(name: impl Into<String>, channel_type: ChannelType, config: serde_json::Value) -> Self {
        Self {
            id: None,
            name: name.into(),
            channel_type,
            enabled: true,
            rate_limit_per_hour: 60,
            config,
            created_at: None,
            updated_at: None,
        }
    }

    pub fn with_rate_limit(mut self, rate_limit: i64) -> Self {
        self.rate_limit_per_hour = rate_limit;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Validate the channel configuration
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(NotificationError::InvalidConfiguration(
                "Channel name cannot be empty".to_string(),
            ));
        }

        match self.channel_type {
            ChannelType::Slack => {
                let slack_config: SlackConfig = serde_json::from_value(self.config.clone())
                    .map_err(|e| NotificationError::InvalidConfiguration(format!("Invalid Slack config: {}", e)))?;
                slack_config.validate()?;
            }
            ChannelType::Email => {
                let email_config: EmailConfig = serde_json::from_value(self.config.clone())
                    .map_err(|e| NotificationError::InvalidConfiguration(format!("Invalid Email config: {}", e)))?;
                email_config.validate()?;
            }
            ChannelType::PagerDuty => {
                let pd_config: PagerDutyConfig = serde_json::from_value(self.config.clone())
                    .map_err(|e| NotificationError::InvalidConfiguration(format!("Invalid PagerDuty config: {}", e)))?;
                pd_config.validate()?;
            }
            ChannelType::Webhook => {
                let webhook_config: NotificationWebhookConfig = serde_json::from_value(self.config.clone())
                    .map_err(|e| NotificationError::InvalidConfiguration(format!("Invalid Webhook config: {}", e)))?;
                webhook_config.validate()?;
            }
        }

        Ok(())
    }
}

/// Message template for formatting alert notifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageTemplate {
    pub id: Option<i64>,
    pub channel_type: ChannelType,
    pub severity: AlertSeverity,
    pub template: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl MessageTemplate {
    pub fn new(channel_type: ChannelType, severity: AlertSeverity, template: impl Into<String>) -> Self {
        Self {
            id: None,
            channel_type,
            severity,
            template: template.into(),
            created_at: None,
            updated_at: None,
        }
    }

    /// Render template with alert data
    pub fn render(&self, rule: &AlertRule, alert: &Alert, trigger_value: Option<&str>) -> Result<String> {
        let mut rendered = self.template.clone();

        // Replace template variables
        rendered = rendered.replace("{{rule_name}}", &rule.name);
        rendered = rendered.replace("{{severity}}", &rule.severity.to_string().to_uppercase());
        rendered = rendered.replace("{{condition}}", &rule.condition);
        rendered = rendered.replace("{{trigger_value}}", trigger_value.unwrap_or("N/A"));

        if let Some(value_json) = &alert.trigger_value {
            if let Some(value_str) = value_json.as_str() {
                rendered = rendered.replace("{{current_value}}", value_str);
            }
        }

        let triggered_at = alert.triggered_at.format("%Y-%m-%d %H:%M:%S UTC").to_string();
        rendered = rendered.replace("{{triggered_at}}", &triggered_at);

        if let Some(alert_id) = alert.id {
            rendered = rendered.replace("{{alert_id}}", &alert_id.to_string());
        }

        Ok(rendered)
    }
}

/// Rate limiter for notifications
#[derive(Debug)]
pub struct RateLimiter {
    notifications: HashMap<String, Vec<DateTime<Utc>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            notifications: HashMap::new(),
        }
    }

    /// Check if a notification is allowed based on rate limit
    pub fn check_limit(&mut self, channel_name: &str, rate_limit_per_hour: i64) -> bool {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);

        // Get or create notification history for this channel
        let history = self.notifications.entry(channel_name.to_string()).or_insert_with(Vec::new);

        // Remove notifications older than 1 hour
        history.retain(|timestamp| *timestamp > one_hour_ago);

        // Check if we're under the rate limit
        if (history.len() as i64) < rate_limit_per_hour {
            history.push(now);
            true
        } else {
            false
        }
    }

    /// Get current notification count for a channel in the last hour
    pub fn get_count(&self, channel_name: &str) -> usize {
        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);

        self.notifications
            .get(channel_name)
            .map(|history| {
                history.iter().filter(|timestamp| **timestamp > one_hour_ago).count()
            })
            .unwrap_or(0)
    }

    /// Clear notification history for a channel
    pub fn clear(&mut self, channel_name: &str) {
        self.notifications.remove(channel_name);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Default Slack message template for critical alerts
pub const SLACK_CRITICAL_TEMPLATE: &str = r#"🚨 CRITICAL: {{rule_name}}

{{condition}}

Current value: {{current_value}}
Triggered at: {{triggered_at}}

[View Dashboard](https://orchestrate.example.com/alerts/{{alert_id}})"#;

/// Default Slack message template for warning alerts
pub const SLACK_WARNING_TEMPLATE: &str = r#"⚠️ WARNING: {{rule_name}}

{{condition}}

Current value: {{current_value}}
Triggered at: {{triggered_at}}

[View Dashboard](https://orchestrate.example.com/alerts/{{alert_id}})"#;

/// Default Slack message template for info alerts
pub const SLACK_INFO_TEMPLATE: &str = r#"ℹ️ INFO: {{rule_name}}

{{condition}}

Current value: {{current_value}}
Triggered at: {{triggered_at}}

[View Dashboard](https://orchestrate.example.com/alerts/{{alert_id}})"#;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_channel_type_to_string() {
        assert_eq!(ChannelType::Slack.to_string(), "slack");
        assert_eq!(ChannelType::Email.to_string(), "email");
        assert_eq!(ChannelType::PagerDuty.to_string(), "pagerduty");
        assert_eq!(ChannelType::Webhook.to_string(), "webhook");
    }

    #[test]
    fn test_channel_type_from_string() {
        use std::str::FromStr;
        assert_eq!(ChannelType::from_str("slack").unwrap(), ChannelType::Slack);
        assert_eq!(ChannelType::from_str("email").unwrap(), ChannelType::Email);
        assert_eq!(ChannelType::from_str("pagerduty").unwrap(), ChannelType::PagerDuty);
        assert_eq!(ChannelType::from_str("webhook").unwrap(), ChannelType::Webhook);
        assert!(ChannelType::from_str("invalid").is_err());
    }

    #[test]
    fn test_slack_config_validation_success() {
        let config = SlackConfig::new("https://hooks.slack.com/services/T00000000/B00000000/XXXXXXXXXXXX");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_slack_config_validation_empty_url() {
        let config = SlackConfig::new("");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_slack_config_validation_invalid_url() {
        let config = SlackConfig::new("https://example.com/webhook");
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid Slack webhook URL"));
    }

    #[test]
    fn test_email_config_validation_success() {
        let config = EmailConfig::new(
            "smtp.example.com",
            587,
            "user@example.com",
            "password",
            "alerts@example.com",
            vec!["recipient@example.com".to_string()],
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_email_config_validation_empty_host() {
        let config = EmailConfig::new(
            "",
            587,
            "user@example.com",
            "password",
            "alerts@example.com",
            vec!["recipient@example.com".to_string()],
        );
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_email_config_validation_no_recipients() {
        let config = EmailConfig::new(
            "smtp.example.com",
            587,
            "user@example.com",
            "password",
            "alerts@example.com",
            vec![],
        );
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("At least one recipient"));
    }

    #[test]
    fn test_pagerduty_config_validation_success() {
        let config = PagerDutyConfig::new("integration_key_12345");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_pagerduty_config_validation_empty_key() {
        let config = PagerDutyConfig::new("");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_webhook_config_validation_success() {
        let config = NotificationWebhookConfig::new("https://example.com/webhook");
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_webhook_config_validation_invalid_url() {
        let config = NotificationWebhookConfig::new("not-a-url");
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must start with http"));
    }

    #[test]
    fn test_webhook_config_validation_invalid_method() {
        let mut config = NotificationWebhookConfig::new("https://example.com/webhook");
        config.method = "GET".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("must be POST or PUT"));
    }

    #[test]
    fn test_channel_config_creation() {
        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let config = ChannelConfig::new(
            "slack-critical",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );

        assert_eq!(config.name, "slack-critical");
        assert_eq!(config.channel_type, ChannelType::Slack);
        assert!(config.enabled);
        assert_eq!(config.rate_limit_per_hour, 60);
    }

    #[test]
    fn test_channel_config_validation_slack() {
        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let config = ChannelConfig::new(
            "slack-critical",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_channel_config_validation_invalid_slack() {
        let slack_config = SlackConfig::new("invalid-url");
        let config = ChannelConfig::new(
            "slack-critical",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );

        assert!(config.validate().is_err());
    }

    #[test]
    fn test_message_template_creation() {
        let template = MessageTemplate::new(
            ChannelType::Slack,
            AlertSeverity::Critical,
            SLACK_CRITICAL_TEMPLATE,
        );

        assert_eq!(template.channel_type, ChannelType::Slack);
        assert_eq!(template.severity, AlertSeverity::Critical);
        assert!(template.template.contains("CRITICAL"));
    }

    #[test]
    fn test_message_template_render() {
        let template = MessageTemplate::new(
            ChannelType::Slack,
            AlertSeverity::Critical,
            SLACK_CRITICAL_TEMPLATE,
        );

        let rule = AlertRule::new(
            "high-failure-rate",
            "rate(orchestrate_agent_failures_total[5m]) > 0.2",
            AlertSeverity::Critical,
            vec!["slack".to_string()],
        );

        let mut alert = Alert::new(1, "fingerprint123");
        alert.id = Some(123);
        alert.trigger_value = Some(json!("35%"));

        let rendered = template.render(&rule, &alert, Some("35%")).unwrap();

        assert!(rendered.contains("CRITICAL: high-failure-rate"));
        assert!(rendered.contains("rate(orchestrate_agent_failures_total[5m]) > 0.2"));
        assert!(rendered.contains("35%"));
        assert!(rendered.contains("/alerts/123"));
    }

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let mut limiter = RateLimiter::new();

        // Should allow up to the limit
        for _ in 0..5 {
            assert!(limiter.check_limit("test-channel", 10));
        }

        // Count should be 5
        assert_eq!(limiter.get_count("test-channel"), 5);
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let mut limiter = RateLimiter::new();

        // Fill up to the limit
        for _ in 0..10 {
            assert!(limiter.check_limit("test-channel", 10));
        }

        // Should block the next one
        assert!(!limiter.check_limit("test-channel", 10));

        // Count should be 10
        assert_eq!(limiter.get_count("test-channel"), 10);
    }

    #[test]
    fn test_rate_limiter_clear() {
        let mut limiter = RateLimiter::new();

        limiter.check_limit("test-channel", 10);
        assert_eq!(limiter.get_count("test-channel"), 1);

        limiter.clear("test-channel");
        assert_eq!(limiter.get_count("test-channel"), 0);
    }

    #[test]
    fn test_rate_limiter_independent_channels() {
        let mut limiter = RateLimiter::new();

        limiter.check_limit("channel-1", 10);
        limiter.check_limit("channel-2", 10);

        assert_eq!(limiter.get_count("channel-1"), 1);
        assert_eq!(limiter.get_count("channel-2"), 1);
    }
}
