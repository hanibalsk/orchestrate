//! Tests for schedule CLI commands

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn setup_test_env() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db").to_string_lossy().to_string();
    (temp_dir, db_path)
}

#[test]
fn test_schedule_add_basic() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("0 2 * * *")
        .arg("--agent")
        .arg("StoryDeveloper")
        .arg("--task")
        .arg("Run daily backup");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Schedule 'test-schedule' added"));
}

#[test]
fn test_schedule_add_with_invalid_cron() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("invalid cron")
        .arg("--agent")
        .arg("StoryDeveloper")
        .arg("--task")
        .arg("Task");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Invalid cron expression"));
}

#[test]
fn test_schedule_list_empty() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No schedules found"));
}

#[test]
fn test_schedule_list_with_schedules() {
    let (_temp, db_path) = setup_test_env();

    // Add a schedule first
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("daily-backup")
        .arg("--cron")
        .arg("0 2 * * *")
        .arg("--agent")
        .arg("BackgroundController")
        .arg("--task")
        .arg("Run backup");
    cmd.assert().success();

    // List schedules
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("daily-backup"))
        .stdout(predicate::str::contains("0 2 * * *"))
        .stdout(predicate::str::contains("enabled"));
}

#[test]
fn test_schedule_show() {
    let (_temp, db_path) = setup_test_env();

    // Add a schedule first
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("@daily")
        .arg("--agent")
        .arg("TestAgent")
        .arg("--task")
        .arg("Test task");
    cmd.assert().success();

    // Show schedule details
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("show")
        .arg("test-schedule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("test-schedule"))
        .stdout(predicate::str::contains("@daily"))
        .stdout(predicate::str::contains("TestAgent"))
        .stdout(predicate::str::contains("Test task"))
        .stdout(predicate::str::contains("Enabled: true"));
}

#[test]
fn test_schedule_show_not_found() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("show")
        .arg("nonexistent");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Schedule not found"));
}

#[test]
fn test_schedule_pause() {
    let (_temp, db_path) = setup_test_env();

    // Add a schedule first
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("@hourly")
        .arg("--agent")
        .arg("TestAgent")
        .arg("--task")
        .arg("Test");
    cmd.assert().success();

    // Pause the schedule
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("pause")
        .arg("test-schedule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Schedule 'test-schedule' paused"));

    // Verify it's paused
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("show")
        .arg("test-schedule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Enabled: false"));
}

#[test]
fn test_schedule_resume() {
    let (_temp, db_path) = setup_test_env();

    // Add and pause a schedule
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("@hourly")
        .arg("--agent")
        .arg("TestAgent")
        .arg("--task")
        .arg("Test");
    cmd.assert().success();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("pause")
        .arg("test-schedule");
    cmd.assert().success();

    // Resume the schedule
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("resume")
        .arg("test-schedule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Schedule 'test-schedule' resumed"));

    // Verify it's enabled
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("show")
        .arg("test-schedule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Enabled: true"));
}

#[test]
fn test_schedule_delete() {
    let (_temp, db_path) = setup_test_env();

    // Add a schedule first
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("@daily")
        .arg("--agent")
        .arg("TestAgent")
        .arg("--task")
        .arg("Test");
    cmd.assert().success();

    // Delete the schedule
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("delete")
        .arg("test-schedule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Schedule 'test-schedule' deleted"));

    // Verify it's gone
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("show")
        .arg("test-schedule");

    cmd.assert().failure();
}

#[test]
fn test_schedule_run_now() {
    let (_temp, db_path) = setup_test_env();

    // Add a schedule first
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("@daily")
        .arg("--agent")
        .arg("StoryDeveloper")
        .arg("--task")
        .arg("Test task");
    cmd.assert().success();

    // Trigger immediate run
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("run-now")
        .arg("test-schedule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Triggered schedule 'test-schedule'"));
}

#[test]
fn test_schedule_history_empty() {
    let (_temp, db_path) = setup_test_env();

    // Add a schedule first
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("@daily")
        .arg("--agent")
        .arg("TestAgent")
        .arg("--task")
        .arg("Test");
    cmd.assert().success();

    // Check history
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("history")
        .arg("test-schedule");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("No execution history"));
}

#[test]
fn test_schedule_add_duplicate_name() {
    let (_temp, db_path) = setup_test_env();

    // Add first schedule
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("@daily")
        .arg("--agent")
        .arg("TestAgent")
        .arg("--task")
        .arg("Test");
    cmd.assert().success();

    // Try to add duplicate
    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("schedule")
        .arg("add")
        .arg("--name")
        .arg("test-schedule")
        .arg("--cron")
        .arg("@hourly")
        .arg("--agent")
        .arg("TestAgent")
        .arg("--task")
        .arg("Test");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("already exists"));
}
