//! Tests for deployment CLI commands

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn setup_test_env() -> (TempDir, String) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db").to_string_lossy().to_string();
    (temp_dir, db_path)
}

#[test]
fn test_deploy_command_requires_env() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("deploy")
        .arg("deploy")
        .arg("--version")
        .arg("1.0.0");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_deploy_command_requires_version() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("deploy")
        .arg("deploy")
        .arg("--env")
        .arg("staging");

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_deploy_status_command() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("deploy")
        .arg("status")
        .arg("--env")
        .arg("staging");

    // Should either succeed or fail with environment not found
    let _ = cmd.output();
}

#[test]
fn test_deploy_history_command() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("deploy")
        .arg("history")
        .arg("--env")
        .arg("staging");

    let _ = cmd.output();
}

#[test]
fn test_deploy_history_with_limit() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("deploy")
        .arg("history")
        .arg("--env")
        .arg("staging")
        .arg("--limit")
        .arg("10");

    let _ = cmd.output();
}

#[test]
fn test_deploy_diff_command() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("deploy")
        .arg("diff")
        .arg("--env")
        .arg("staging")
        .arg("--version")
        .arg("1.0.1");

    let _ = cmd.output();
}

#[test]
fn test_deploy_validate_command() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("deploy")
        .arg("validate")
        .arg("--env")
        .arg("staging");

    let _ = cmd.output();
}

#[test]
fn test_deploy_rollback_command() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("deploy")
        .arg("rollback")
        .arg("--env")
        .arg("staging");

    let _ = cmd.output();
}

#[test]
fn test_deploy_rollback_with_version() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("deploy")
        .arg("rollback")
        .arg("--env")
        .arg("staging")
        .arg("--version")
        .arg("1.0.0");

    let _ = cmd.output();
}

#[test]
fn test_release_prepare_command() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("release")
        .arg("prepare")
        .arg("--type")
        .arg("patch");

    let _ = cmd.output();
}

#[test]
fn test_release_create_command() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("release")
        .arg("create")
        .arg("--version")
        .arg("1.0.0");

    let _ = cmd.output();
}

#[test]
fn test_release_publish_command() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("release")
        .arg("publish")
        .arg("--version")
        .arg("1.0.0");

    let _ = cmd.output();
}

#[test]
fn test_release_notes_command() {
    let (_temp, db_path) = setup_test_env();

    let mut cmd = Command::cargo_bin("orchestrate").unwrap();
    cmd.arg("--db-path")
        .arg(&db_path)
        .arg("release")
        .arg("notes")
        .arg("--from")
        .arg("v1.0.0")
        .arg("--to")
        .arg("v1.1.0");

    let _ = cmd.output();
}
