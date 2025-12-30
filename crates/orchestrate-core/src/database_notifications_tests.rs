//! Database integration tests for notification channels

#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::notifications::*;
    use crate::alerting::*;
    use tempfile::TempDir;

    async fn setup_test_db() -> (Database, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");
        let db = Database::new(db_path.to_str().unwrap()).await.unwrap();
        (db, temp_dir)
    }

    #[tokio::test]
    async fn test_create_and_get_notification_channel() {
        let (db, _temp_dir) = setup_test_db().await;

        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let channel = ChannelConfig::new(
            "slack-critical",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );

        let id = db.create_notification_channel(&channel).await.unwrap();
        assert!(id > 0);

        let retrieved = db.get_notification_channel(id).await.unwrap();
        assert_eq!(retrieved.name, "slack-critical");
        assert_eq!(retrieved.channel_type, ChannelType::Slack);
        assert!(retrieved.enabled);
        assert_eq!(retrieved.rate_limit_per_hour, 60);
    }

    #[tokio::test]
    async fn test_get_notification_channel_by_name() {
        let (db, _temp_dir) = setup_test_db().await;

        let email_config = EmailConfig::new(
            "smtp.example.com",
            587,
            "user@example.com",
            "password",
            "alerts@example.com",
            vec!["recipient@example.com".to_string()],
        );
        let channel = ChannelConfig::new(
            "email-alerts",
            ChannelType::Email,
            serde_json::to_value(&email_config).unwrap(),
        );

        db.create_notification_channel(&channel).await.unwrap();

        let retrieved = db.get_notification_channel_by_name("email-alerts").await.unwrap();
        assert_eq!(retrieved.name, "email-alerts");
        assert_eq!(retrieved.channel_type, ChannelType::Email);
    }

    #[tokio::test]
    async fn test_list_notification_channels() {
        let (db, _temp_dir) = setup_test_db().await;

        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let channel1 = ChannelConfig::new(
            "slack-1",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );

        let pd_config = PagerDutyConfig::new("integration_key_123");
        let channel2 = ChannelConfig::new(
            "pagerduty-1",
            ChannelType::PagerDuty,
            serde_json::to_value(&pd_config).unwrap(),
        );

        db.create_notification_channel(&channel1).await.unwrap();
        db.create_notification_channel(&channel2).await.unwrap();

        let channels = db.list_notification_channels().await.unwrap();
        assert_eq!(channels.len(), 2);
    }

    #[tokio::test]
    async fn test_list_enabled_notification_channels() {
        let (db, _temp_dir) = setup_test_db().await;

        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let channel1 = ChannelConfig::new(
            "slack-enabled",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        ).with_enabled(true);

        let channel2 = ChannelConfig::new(
            "slack-disabled",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        ).with_enabled(false);

        let id1 = db.create_notification_channel(&channel1).await.unwrap();
        let id2 = db.create_notification_channel(&channel2).await.unwrap();

        // Disable channel2
        db.set_notification_channel_enabled(id2, false).await.unwrap();

        let enabled_channels = db.list_enabled_notification_channels().await.unwrap();
        assert_eq!(enabled_channels.len(), 1);
        assert_eq!(enabled_channels[0].name, "slack-enabled");
    }

    #[tokio::test]
    async fn test_update_notification_channel() {
        let (db, _temp_dir) = setup_test_db().await;

        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let mut channel = ChannelConfig::new(
            "slack-test",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );

        let id = db.create_notification_channel(&channel).await.unwrap();
        channel.id = Some(id);
        channel.name = "slack-updated".to_string();
        channel.rate_limit_per_hour = 100;

        db.update_notification_channel(&channel).await.unwrap();

        let retrieved = db.get_notification_channel(id).await.unwrap();
        assert_eq!(retrieved.name, "slack-updated");
        assert_eq!(retrieved.rate_limit_per_hour, 100);
    }

    #[tokio::test]
    async fn test_set_notification_channel_enabled() {
        let (db, _temp_dir) = setup_test_db().await;

        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let channel = ChannelConfig::new(
            "slack-test",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );

        let id = db.create_notification_channel(&channel).await.unwrap();

        db.set_notification_channel_enabled(id, false).await.unwrap();
        let retrieved = db.get_notification_channel(id).await.unwrap();
        assert!(!retrieved.enabled);

        db.set_notification_channel_enabled(id, true).await.unwrap();
        let retrieved = db.get_notification_channel(id).await.unwrap();
        assert!(retrieved.enabled);
    }

    #[tokio::test]
    async fn test_delete_notification_channel() {
        let (db, _temp_dir) = setup_test_db().await;

        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let channel = ChannelConfig::new(
            "slack-test",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );

        let id = db.create_notification_channel(&channel).await.unwrap();
        db.delete_notification_channel(id).await.unwrap();

        let result = db.get_notification_channel(id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_upsert_message_template() {
        let (db, _temp_dir) = setup_test_db().await;

        let template = MessageTemplate::new(
            ChannelType::Slack,
            AlertSeverity::Critical,
            SLACK_CRITICAL_TEMPLATE,
        );

        let id1 = db.upsert_message_template(&template).await.unwrap();
        assert!(id1 > 0);

        // Upsert again should update, not create new
        let template2 = MessageTemplate::new(
            ChannelType::Slack,
            AlertSeverity::Critical,
            "Updated template",
        );

        let id2 = db.upsert_message_template(&template2).await.unwrap();
        // ID might be different in SQLite ON CONFLICT, so just check we can retrieve it
        let retrieved = db.get_message_template(ChannelType::Slack, AlertSeverity::Critical).await.unwrap();
        assert_eq!(retrieved.template, "Updated template");
    }

    #[tokio::test]
    async fn test_get_message_template() {
        let (db, _temp_dir) = setup_test_db().await;

        let template = MessageTemplate::new(
            ChannelType::Email,
            AlertSeverity::Warning,
            "Warning: {{rule_name}}",
        );

        db.upsert_message_template(&template).await.unwrap();

        let retrieved = db.get_message_template(ChannelType::Email, AlertSeverity::Warning).await.unwrap();
        assert_eq!(retrieved.channel_type, ChannelType::Email);
        assert_eq!(retrieved.severity, AlertSeverity::Warning);
        assert_eq!(retrieved.template, "Warning: {{rule_name}}");
    }

    #[tokio::test]
    async fn test_list_message_templates() {
        let (db, _temp_dir) = setup_test_db().await;

        let template1 = MessageTemplate::new(
            ChannelType::Slack,
            AlertSeverity::Critical,
            SLACK_CRITICAL_TEMPLATE,
        );
        let template2 = MessageTemplate::new(
            ChannelType::Slack,
            AlertSeverity::Warning,
            SLACK_WARNING_TEMPLATE,
        );
        let template3 = MessageTemplate::new(
            ChannelType::Email,
            AlertSeverity::Critical,
            "Email critical",
        );

        db.upsert_message_template(&template1).await.unwrap();
        db.upsert_message_template(&template2).await.unwrap();
        db.upsert_message_template(&template3).await.unwrap();

        let templates = db.list_message_templates().await.unwrap();
        assert_eq!(templates.len(), 3);
    }

    #[tokio::test]
    async fn test_delete_message_template() {
        let (db, _temp_dir) = setup_test_db().await;

        let template = MessageTemplate::new(
            ChannelType::Slack,
            AlertSeverity::Info,
            SLACK_INFO_TEMPLATE,
        );

        db.upsert_message_template(&template).await.unwrap();
        db.delete_message_template(ChannelType::Slack, AlertSeverity::Info).await.unwrap();

        let result = db.get_message_template(ChannelType::Slack, AlertSeverity::Info).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_log_notification() {
        let (db, _temp_dir) = setup_test_db().await;

        // Create an alert rule and alert first
        let rule = AlertRule::new(
            "test-rule",
            "metric > 100",
            AlertSeverity::Warning,
            vec!["slack".to_string()],
        );
        let created_rule = db.create_alert_rule(rule).await.unwrap();
        let rule_id = created_rule.id.unwrap();

        let fingerprint = crate::alerting::generate_fingerprint("test-rule", "metric > 100");
        let alert = Alert::new(rule_id, fingerprint);
        let created_alert = db.create_alert(alert).await.unwrap();
        let alert_id = created_alert.id.unwrap();

        // Create a channel
        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let channel = ChannelConfig::new(
            "slack-test",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );
        let channel_id = db.create_notification_channel(&channel).await.unwrap();

        // Log notification
        let log_id = db.log_notification(alert_id, channel_id, "sent", None).await.unwrap();
        assert!(log_id > 0);
    }

    #[tokio::test]
    async fn test_get_notification_logs_by_alert() {
        let (db, _temp_dir) = setup_test_db().await;

        // Create alert
        let rule = AlertRule::new(
            "test-rule",
            "metric > 100",
            AlertSeverity::Warning,
            vec!["slack".to_string()],
        );
        let created_rule = db.create_alert_rule(rule).await.unwrap();
        let rule_id = created_rule.id.unwrap();
        let fingerprint = crate::alerting::generate_fingerprint("test-rule", "metric > 100");
        let alert = Alert::new(rule_id, fingerprint);
        let created_alert = db.create_alert(alert).await.unwrap();
        let alert_id = created_alert.id.unwrap();

        // Create channels
        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let channel1 = ChannelConfig::new(
            "slack-1",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );
        let channel2 = ChannelConfig::new(
            "slack-2",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );
        let channel_id1 = db.create_notification_channel(&channel1).await.unwrap();
        let channel_id2 = db.create_notification_channel(&channel2).await.unwrap();

        // Log notifications
        db.log_notification(alert_id, channel_id1, "sent", None).await.unwrap();
        db.log_notification(alert_id, channel_id2, "failed", Some("Connection timeout")).await.unwrap();

        let logs = db.get_notification_logs_by_alert(alert_id).await.unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].alert_id, alert_id);
    }

    #[tokio::test]
    async fn test_get_notification_logs_by_channel() {
        let (db, _temp_dir) = setup_test_db().await;

        // Create alerts
        let rule = AlertRule::new(
            "test-rule",
            "metric > 100",
            AlertSeverity::Warning,
            vec!["slack".to_string()],
        );
        let created_rule = db.create_alert_rule(rule).await.unwrap();
        let rule_id = created_rule.id.unwrap();

        let fingerprint1 = crate::alerting::generate_fingerprint("test-rule", "metric > 100");
        let alert1 = Alert::new(rule_id, fingerprint1);
        let created_alert1 = db.create_alert(alert1).await.unwrap();
        let alert_id1 = created_alert1.id.unwrap();

        let fingerprint2 = crate::alerting::generate_fingerprint("test-rule-2", "metric > 200");
        let alert2 = Alert::new(rule_id, fingerprint2);
        let created_alert2 = db.create_alert(alert2).await.unwrap();
        let alert_id2 = created_alert2.id.unwrap();

        // Create channel
        let slack_config = SlackConfig::new("https://hooks.slack.com/services/T00/B00/XXX");
        let channel = ChannelConfig::new(
            "slack-test",
            ChannelType::Slack,
            serde_json::to_value(&slack_config).unwrap(),
        );
        let channel_id = db.create_notification_channel(&channel).await.unwrap();

        // Log notifications for both alerts to same channel
        db.log_notification(alert_id1, channel_id, "sent", None).await.unwrap();
        db.log_notification(alert_id2, channel_id, "sent", None).await.unwrap();

        let logs = db.get_notification_logs_by_channel(channel_id).await.unwrap();
        assert_eq!(logs.len(), 2);
        assert_eq!(logs[0].channel_id, channel_id);
    }
}
