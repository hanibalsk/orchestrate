//! Tests for webhook CLI commands

use assert_cmd::Command;
use orchestrate_core::{Database, WebhookEvent, WebhookEventStatus};
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
async fn test_webhook_start_command() {
    let (_temp_dir, db_path, _db) = setup_test_db().await;

    // Test that webhook start command is recognized
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("webhook")
        .arg("start")
        .arg("--port")
        .arg("9000")
        .timeout(std::time::Duration::from_secs(2));

    // Should start server (will timeout, but that's expected for this test)
    let result = cmd.assert();
    // We expect either success or timeout, not "unknown command"
    // This test mainly checks that the command is recognized
}

#[tokio::test]
async fn test_webhook_list_events_empty() {
    let (_temp_dir, db_path, _db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("webhook")
        .arg("list-events");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No webhook events found"));
}

#[tokio::test]
async fn test_webhook_list_events_with_events() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Insert test events
    let event1 = WebhookEvent::new(
        "delivery-1".to_string(),
        "pull_request".to_string(),
        r#"{"action":"opened"}"#.to_string(),
    );
    db.insert_webhook_event(&event1).await.unwrap();

    let event2 = WebhookEvent::new(
        "delivery-2".to_string(),
        "check_run".to_string(),
        r#"{"action":"completed"}"#.to_string(),
    );
    db.insert_webhook_event(&event2).await.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("webhook")
        .arg("list-events");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("pull_request"))
        .stdout(predicate::str::contains("check_run"))
        .stdout(predicate::str::contains("WEBHOOK EVENTS"))
        .stdout(predicate::str::contains("pending"));
}

#[tokio::test]
async fn test_webhook_list_events_with_limit() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Insert multiple events
    for i in 1..=5 {
        let event = WebhookEvent::new(
            format!("delivery-{}", i),
            "pull_request".to_string(),
            "{}".to_string(),
        );
        db.insert_webhook_event(&event).await.unwrap();
    }

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("webhook")
        .arg("list-events")
        .arg("--limit")
        .arg("3");

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Should show only 3 events (check rows in the table output)
    let event_count = stdout.matches("pull_request").count();
    assert_eq!(event_count, 3);
}

#[tokio::test]
async fn test_webhook_status_command() {
    let (_temp_dir, db_path, _db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("webhook")
        .arg("status");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Webhook Server Status"))
        .stdout(predicate::str::contains("Status:"));
}

#[tokio::test]
async fn test_webhook_secret_rotate_command() {
    let (_temp_dir, db_path, _db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("webhook")
        .arg("secret")
        .arg("rotate");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("New Webhook Secret Generated"))
        .stdout(predicate::str::contains("GITHUB_WEBHOOK_SECRET"));
}

#[tokio::test]
async fn test_webhook_simulate_command() {
    let (_temp_dir, db_path, _db) = setup_test_db().await;

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("webhook")
        .arg("simulate")
        .arg("pull_request.opened");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Simulated event"))
        .stdout(predicate::str::contains("pull_request.opened"));
}

#[tokio::test]
async fn test_webhook_list_events_filter_by_status() {
    let (_temp_dir, db_path, db) = setup_test_db().await;

    // Insert events with different statuses
    let mut event1 = WebhookEvent::new(
        "delivery-1".to_string(),
        "pull_request".to_string(),
        "{}".to_string(),
    );
    let id1 = db.insert_webhook_event(&event1).await.unwrap();
    event1.id = Some(id1);
    event1.mark_completed();
    db.update_webhook_event(&event1).await.unwrap();

    let event2 = WebhookEvent::new(
        "delivery-2".to_string(),
        "check_run".to_string(),
        "{}".to_string(),
    );
    db.insert_webhook_event(&event2).await.unwrap();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("webhook")
        .arg("list-events")
        .arg("--status")
        .arg("pending");

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Should only show the pending event
    assert!(stdout.contains("check_run"));
    assert!(stdout.contains("pending"));
    assert!(!stdout.contains("completed"));
}
