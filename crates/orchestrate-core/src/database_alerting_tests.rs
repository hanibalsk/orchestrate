//! Database tests for alerting operations

use crate::alerting::{Alert, AlertRule, AlertSeverity, AlertStatus, generate_fingerprint};
use crate::database::Database;

#[tokio::test]
async fn test_create_and_get_alert_rule() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "high-failure-rate",
        "rate(orchestrate_agent_failures_total[5m]) > 0.2",
        AlertSeverity::Critical,
        vec!["slack".to_string(), "pagerduty".to_string()],
    );

    let created_rule = db.create_alert_rule(rule).await.unwrap();
    assert!(created_rule.id.is_some());
    assert_eq!(created_rule.name, "high-failure-rate");
    assert_eq!(created_rule.severity, AlertSeverity::Critical);
    assert_eq!(created_rule.channels.len(), 2);
    assert!(created_rule.enabled);

    let fetched_rule = db
        .get_alert_rule(created_rule.id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched_rule.name, "high-failure-rate");
    assert_eq!(fetched_rule.condition, "rate(orchestrate_agent_failures_total[5m]) > 0.2");
}

#[tokio::test]
async fn test_get_alert_rule_by_name() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "queue-backup",
        "orchestrate_queue_depth{queue='webhook_events'} > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    );

    db.create_alert_rule(rule).await.unwrap();

    let fetched_rule = db
        .get_alert_rule_by_name("queue-backup")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched_rule.name, "queue-backup");
    assert_eq!(fetched_rule.severity, AlertSeverity::Warning);
}

#[tokio::test]
async fn test_list_alert_rules() {
    let db = Database::in_memory().await.unwrap();

    let rule1 = AlertRule::new(
        "rule-1",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );
    let rule2 = AlertRule::new(
        "rule-2",
        "metric < 50",
        AlertSeverity::Info,
        vec!["slack".to_string()],
    );

    db.create_alert_rule(rule1).await.unwrap();
    db.create_alert_rule(rule2).await.unwrap();

    let rules = db.list_alert_rules().await.unwrap();
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].name, "rule-1"); // Sorted by name
    assert_eq!(rules[1].name, "rule-2");
}

#[tokio::test]
async fn test_list_enabled_alert_rules() {
    let db = Database::in_memory().await.unwrap();

    let rule1 = AlertRule::new(
        "enabled-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );
    let rule2 = AlertRule::new(
        "disabled-rule",
        "metric < 50",
        AlertSeverity::Info,
        vec!["slack".to_string()],
    )
    .with_enabled(false);

    db.create_alert_rule(rule1).await.unwrap();
    db.create_alert_rule(rule2).await.unwrap();

    let enabled_rules = db.list_enabled_alert_rules().await.unwrap();
    assert_eq!(enabled_rules.len(), 1);
    assert_eq!(enabled_rules[0].name, "enabled-rule");
}

#[tokio::test]
async fn test_update_alert_rule() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );

    let mut created_rule = db.create_alert_rule(rule).await.unwrap();

    // Update the rule
    created_rule.condition = "metric > 200".to_string();
    created_rule.severity = AlertSeverity::Critical;
    created_rule.channels = vec!["slack".to_string(), "pagerduty".to_string()];

    db.update_alert_rule(&created_rule).await.unwrap();

    let updated_rule = db
        .get_alert_rule(created_rule.id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(updated_rule.condition, "metric > 200");
    assert_eq!(updated_rule.severity, AlertSeverity::Critical);
    assert_eq!(updated_rule.channels.len(), 2);
}

#[tokio::test]
async fn test_set_alert_rule_enabled() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );

    let created_rule = db.create_alert_rule(rule).await.unwrap();
    let rule_id = created_rule.id.unwrap();

    // Disable the rule
    db.set_alert_rule_enabled(rule_id, false).await.unwrap();
    let disabled_rule = db.get_alert_rule(rule_id).await.unwrap().unwrap();
    assert!(!disabled_rule.enabled);

    // Re-enable the rule
    db.set_alert_rule_enabled(rule_id, true).await.unwrap();
    let enabled_rule = db.get_alert_rule(rule_id).await.unwrap().unwrap();
    assert!(enabled_rule.enabled);
}

#[tokio::test]
async fn test_delete_alert_rule() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );

    let created_rule = db.create_alert_rule(rule).await.unwrap();
    let rule_id = created_rule.id.unwrap();

    db.delete_alert_rule(rule_id).await.unwrap();

    let deleted_rule = db.get_alert_rule(rule_id).await.unwrap();
    assert!(deleted_rule.is_none());
}

#[tokio::test]
async fn test_create_and_get_alert() {
    let db = Database::in_memory().await.unwrap();

    // First create a rule
    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Critical,
        vec!["slack".to_string()],
    );
    let created_rule = db.create_alert_rule(rule).await.unwrap();
    let rule_id = created_rule.id.unwrap();

    // Create an alert
    let fingerprint = generate_fingerprint("test-rule", "metric > 100");
    let alert = Alert::new(rule_id, &fingerprint)
        .with_trigger_value(serde_json::json!({"value": 150}))
        .with_metadata(serde_json::json!({"source": "test"}));

    let created_alert = db.create_alert(alert).await.unwrap();
    assert!(created_alert.id.is_some());
    assert_eq!(created_alert.rule_id, rule_id);
    assert_eq!(created_alert.status, AlertStatus::Active);
    assert!(created_alert.is_active());

    let fetched_alert = db
        .get_alert(created_alert.id.unwrap())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched_alert.fingerprint, fingerprint);
    assert_eq!(
        fetched_alert.trigger_value,
        Some(serde_json::json!({"value": 150}))
    );
}

#[tokio::test]
async fn test_get_active_alert_by_fingerprint() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );
    let created_rule = db.create_alert_rule(rule).await.unwrap();
    let rule_id = created_rule.id.unwrap();

    let fingerprint = generate_fingerprint("test-rule", "metric > 100");
    let alert = Alert::new(rule_id, &fingerprint);

    db.create_alert(alert).await.unwrap();

    // Should find the active alert
    let found_alert = db
        .get_active_alert_by_fingerprint(&fingerprint)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(found_alert.fingerprint, fingerprint);
    assert_eq!(found_alert.status, AlertStatus::Active);
}

#[tokio::test]
async fn test_alert_deduplication() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );
    let created_rule = db.create_alert_rule(rule).await.unwrap();
    let rule_id = created_rule.id.unwrap();

    let fingerprint = generate_fingerprint("test-rule", "metric > 100");

    // Create first alert
    let alert1 = Alert::new(rule_id, &fingerprint);
    db.create_alert(alert1).await.unwrap();

    // Check if there's already an active alert with this fingerprint
    let existing_alert = db
        .get_active_alert_by_fingerprint(&fingerprint)
        .await
        .unwrap();
    assert!(existing_alert.is_some());

    // In a real system, we would not create a duplicate alert
    // Instead, we might update the existing one or increment a counter
}

#[tokio::test]
async fn test_list_alerts_by_status() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );
    let created_rule = db.create_alert_rule(rule).await.unwrap();
    let rule_id = created_rule.id.unwrap();

    // Create active alert
    let alert1 = Alert::new(rule_id, generate_fingerprint("test-rule", "1"));
    let created_alert1 = db.create_alert(alert1).await.unwrap();

    // Create and resolve another alert
    let alert2 = Alert::new(rule_id, generate_fingerprint("test-rule", "2"));
    let created_alert2 = db.create_alert(alert2).await.unwrap();
    db.resolve_alert(created_alert2.id.unwrap()).await.unwrap();

    // List active alerts
    let active_alerts = db
        .list_alerts_by_status(AlertStatus::Active)
        .await
        .unwrap();
    assert_eq!(active_alerts.len(), 1);
    assert_eq!(active_alerts[0].id, created_alert1.id);

    // List resolved alerts
    let resolved_alerts = db
        .list_alerts_by_status(AlertStatus::Resolved)
        .await
        .unwrap();
    assert_eq!(resolved_alerts.len(), 1);
    assert_eq!(resolved_alerts[0].id, created_alert2.id);
}

#[tokio::test]
async fn test_list_alerts_by_rule() {
    let db = Database::in_memory().await.unwrap();

    let rule1 = AlertRule::new(
        "rule-1",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );
    let rule2 = AlertRule::new(
        "rule-2",
        "metric < 50",
        AlertSeverity::Info,
        vec!["slack".to_string()],
    );

    let created_rule1 = db.create_alert_rule(rule1).await.unwrap();
    let created_rule2 = db.create_alert_rule(rule2).await.unwrap();

    // Create alerts for both rules
    let alert1 = Alert::new(
        created_rule1.id.unwrap(),
        generate_fingerprint("rule-1", "1"),
    );
    let alert2 = Alert::new(
        created_rule1.id.unwrap(),
        generate_fingerprint("rule-1", "2"),
    );
    let alert3 = Alert::new(
        created_rule2.id.unwrap(),
        generate_fingerprint("rule-2", "1"),
    );

    db.create_alert(alert1).await.unwrap();
    db.create_alert(alert2).await.unwrap();
    db.create_alert(alert3).await.unwrap();

    let rule1_alerts = db
        .list_alerts_by_rule(created_rule1.id.unwrap())
        .await
        .unwrap();
    assert_eq!(rule1_alerts.len(), 2);

    let rule2_alerts = db
        .list_alerts_by_rule(created_rule2.id.unwrap())
        .await
        .unwrap();
    assert_eq!(rule2_alerts.len(), 1);
}

#[tokio::test]
async fn test_acknowledge_alert() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Critical,
        vec!["pagerduty".to_string()],
    );
    let created_rule = db.create_alert_rule(rule).await.unwrap();

    let alert = Alert::new(
        created_rule.id.unwrap(),
        generate_fingerprint("test-rule", "1"),
    );
    let created_alert = db.create_alert(alert).await.unwrap();
    let alert_id = created_alert.id.unwrap();

    // Acknowledge the alert
    db.acknowledge_alert(alert_id, "user@example.com")
        .await
        .unwrap();

    let acknowledged_alert = db.get_alert(alert_id).await.unwrap().unwrap();
    assert_eq!(acknowledged_alert.status, AlertStatus::Acknowledged);
    assert!(acknowledged_alert.acknowledged_at.is_some());
    assert_eq!(
        acknowledged_alert.acknowledged_by,
        Some("user@example.com".to_string())
    );
}

#[tokio::test]
async fn test_resolve_alert() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );
    let created_rule = db.create_alert_rule(rule).await.unwrap();

    let alert = Alert::new(
        created_rule.id.unwrap(),
        generate_fingerprint("test-rule", "1"),
    );
    let created_alert = db.create_alert(alert).await.unwrap();
    let alert_id = created_alert.id.unwrap();

    // Resolve the alert
    db.resolve_alert(alert_id).await.unwrap();

    let resolved_alert = db.get_alert(alert_id).await.unwrap().unwrap();
    assert_eq!(resolved_alert.status, AlertStatus::Resolved);
    assert!(resolved_alert.resolved_at.is_some());
    assert!(resolved_alert.is_resolved());
}

#[tokio::test]
async fn test_increment_alert_notification_count() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Critical,
        vec!["slack".to_string()],
    );
    let created_rule = db.create_alert_rule(rule).await.unwrap();

    let alert = Alert::new(
        created_rule.id.unwrap(),
        generate_fingerprint("test-rule", "1"),
    );
    let created_alert = db.create_alert(alert).await.unwrap();
    let alert_id = created_alert.id.unwrap();

    // Increment notification count
    db.increment_alert_notification_count(alert_id)
        .await
        .unwrap();
    db.increment_alert_notification_count(alert_id)
        .await
        .unwrap();

    let updated_alert = db.get_alert(alert_id).await.unwrap().unwrap();
    assert_eq!(updated_alert.notification_count, 2);
    assert!(updated_alert.last_notified_at.is_some());
}

#[tokio::test]
async fn test_cascade_delete_alerts_on_rule_delete() {
    let db = Database::in_memory().await.unwrap();

    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["email".to_string()],
    );
    let created_rule = db.create_alert_rule(rule).await.unwrap();
    let rule_id = created_rule.id.unwrap();

    // Create an alert for the rule
    let alert = Alert::new(rule_id, generate_fingerprint("test-rule", "1"));
    let created_alert = db.create_alert(alert).await.unwrap();
    let alert_id = created_alert.id.unwrap();

    // Delete the rule (should cascade delete the alert)
    db.delete_alert_rule(rule_id).await.unwrap();

    // Alert should also be deleted
    let deleted_alert = db.get_alert(alert_id).await.unwrap();
    assert!(deleted_alert.is_none());
}
