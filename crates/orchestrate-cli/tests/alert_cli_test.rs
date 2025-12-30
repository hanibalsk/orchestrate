//! Tests for alert CLI commands

use assert_cmd::Command;
use orchestrate_core::{Alert, AlertRule, AlertSeverity, AlertStatus, Database};
use predicates::prelude::*;
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a temporary database
async fn setup_test_db() -> (TempDir, PathBuf, Database) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let db = Database::new(&db_path).await.unwrap();
    (temp_dir, db_path, db)
}

#[tokio::test]
async fn test_alert_rules_list_empty() {
    let (_temp_dir, db_path, _db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("rules")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No alert rules found"));
}

#[tokio::test]
async fn test_alert_rules_list_with_rules() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Create test alert rules
    let rule1 = AlertRule::new(
        "high-queue-depth",
        "orchestrate_queue_depth{queue='webhook_events'} > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    );
    db.create_alert_rule(rule1).await.unwrap();

    let rule2 = AlertRule::new(
        "high-failure-rate",
        "rate(orchestrate_agent_failures_total[5m]) > 0.2",
        AlertSeverity::Critical,
        vec!["slack".to_string(), "pagerduty".to_string()],
    );
    db.create_alert_rule(rule2).await.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("rules")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ALERT RULES"))
        .stdout(predicate::str::contains("high-queue-depth"))
        .stdout(predicate::str::contains("high-failure-rate"))
        .stdout(predicate::str::contains("warning"))
        .stdout(predicate::str::contains("critical"));
}

#[tokio::test]
async fn test_alert_rules_create() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("rules")
        .arg("create")
        .arg("--name")
        .arg("test-rule")
        .arg("--condition")
        .arg("metric > 100")
        .arg("--channel")
        .arg("slack");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Alert rule created"))
        .stdout(predicate::str::contains("test-rule"));

    // Verify rule was created in database
    let rules = db.list_alert_rules().await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].name, "test-rule");
    assert_eq!(rules[0].condition, "metric > 100");
    assert_eq!(rules[0].channels.len(), 1);
    assert_eq!(rules[0].channels[0], "slack");
}

#[tokio::test]
async fn test_alert_rules_create_with_multiple_channels() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("rules")
        .arg("create")
        .arg("--name")
        .arg("critical-rule")
        .arg("--condition")
        .arg("metric > 500")
        .arg("--channel")
        .arg("slack")
        .arg("--channel")
        .arg("pagerduty")
        .arg("--severity")
        .arg("critical");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Alert rule created"));

    // Verify rule was created with multiple channels
    let rules = db.list_alert_rules().await.unwrap();
    assert_eq!(rules.len(), 1);
    assert_eq!(rules[0].channels.len(), 2);
    assert_eq!(rules[0].severity, AlertSeverity::Critical);
}

#[tokio::test]
async fn test_alert_rules_enable() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Create a disabled rule
    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    )
    .with_enabled(false);
    let rule = db.create_alert_rule(rule).await.unwrap();
    let rule_id = rule.id.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("rules")
        .arg("enable")
        .arg("test-rule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Alert rule enabled"))
        .stdout(predicate::str::contains("test-rule"));

    // Verify rule is now enabled
    let updated_rule = db.get_alert_rule(rule_id).await.unwrap().unwrap();
    assert!(updated_rule.enabled);
}

#[tokio::test]
async fn test_alert_rules_disable() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Create an enabled rule
    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    );
    let rule = db.create_alert_rule(rule).await.unwrap();
    let rule_id = rule.id.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("rules")
        .arg("disable")
        .arg("test-rule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Alert rule disabled"))
        .stdout(predicate::str::contains("test-rule"));

    // Verify rule is now disabled
    let updated_rule = db.get_alert_rule(rule_id).await.unwrap().unwrap();
    assert!(!updated_rule.enabled);
}

#[tokio::test]
async fn test_alert_rules_delete() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Create a rule
    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    );
    db.create_alert_rule(rule).await.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("rules")
        .arg("delete")
        .arg("test-rule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Alert rule deleted"))
        .stdout(predicate::str::contains("test-rule"));

    // Verify rule was deleted
    let rules = db.list_alert_rules().await.unwrap();
    assert_eq!(rules.len(), 0);
}

#[tokio::test]
async fn test_alert_list_empty() {
    let (_temp_dir, db_path, _db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No alerts found"));
}

#[tokio::test]
async fn test_alert_list_with_alerts() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Create a rule first
    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    );
    let rule = db.create_alert_rule(rule).await.unwrap();

    // Create alerts
    let alert1 = Alert::new(rule.id.unwrap(), "fingerprint1");
    db.create_alert(alert1).await.unwrap();

    let alert2 = Alert::new(rule.id.unwrap(), "fingerprint2");
    db.create_alert(alert2).await.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ALERTS"))
        .stdout(predicate::str::contains("active"));
}

#[tokio::test]
async fn test_alert_list_filter_by_status() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Create a rule
    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    );
    let rule = db.create_alert_rule(rule).await.unwrap();

    // Create active alert
    let alert1 = Alert::new(rule.id.unwrap(), "fingerprint1");
    db.create_alert(alert1).await.unwrap();

    // Create resolved alert
    let alert2 = Alert::new(rule.id.unwrap(), "fingerprint2");
    let alert2 = db.create_alert(alert2).await.unwrap();
    db.resolve_alert(alert2.id.unwrap()).await.unwrap();

    // Filter for active only
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("list")
        .arg("--status")
        .arg("active");

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Should only show active alerts
    assert!(stdout.contains("active"));
    assert!(!stdout.contains("resolved"));
}

#[tokio::test]
async fn test_alert_acknowledge() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Create a rule and alert
    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    );
    let rule = db.create_alert_rule(rule).await.unwrap();

    let alert = Alert::new(rule.id.unwrap(), "fingerprint1");
    let alert = db.create_alert(alert).await.unwrap();
    let alert_id = alert.id.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("acknowledge")
        .arg(alert_id.to_string());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Alert acknowledged"))
        .stdout(predicate::str::contains(&alert_id.to_string()));

    // Verify alert is acknowledged
    let updated_alert = db.get_alert(alert_id).await.unwrap().unwrap();
    assert_eq!(updated_alert.status, AlertStatus::Acknowledged);
    assert!(updated_alert.acknowledged_at.is_some());
}

#[tokio::test]
async fn test_alert_silence() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Create a rule
    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    );
    db.create_alert_rule(rule).await.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("silence")
        .arg("test-rule")
        .arg("--duration")
        .arg("1h");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Alert rule silenced"))
        .stdout(predicate::str::contains("test-rule"))
        .stdout(predicate::str::contains("1h"));
}

#[tokio::test]
async fn test_alert_test() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Create a rule
    let rule = AlertRule::new(
        "test-rule",
        "metric > 100",
        AlertSeverity::Warning,
        vec!["slack".to_string()],
    );
    db.create_alert_rule(rule).await.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("test")
        .arg("test-rule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Testing alert delivery"))
        .stdout(predicate::str::contains("test-rule"));
}

#[tokio::test]
async fn test_alert_rules_create_missing_name() {
    let (_temp_dir, db_path, _db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("rules")
        .arg("create")
        .arg("--condition")
        .arg("metric > 100")
        .arg("--channel")
        .arg("slack");

    cmd.assert()
        .failure();
}

#[tokio::test]
async fn test_alert_rules_enable_nonexistent() {
    let (_temp_dir, db_path, _db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("alert")
        .arg("rules")
        .arg("enable")
        .arg("nonexistent-rule");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("does not exist")));
}
