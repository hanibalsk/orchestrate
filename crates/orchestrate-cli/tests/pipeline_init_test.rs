use anyhow::Result;
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_pipeline_init_ci() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_file = temp_dir.path().join("ci-pipeline.yaml");

    let mut cmd = Command::cargo_bin("orchestrate")?;
    cmd.arg("pipeline")
        .arg("init")
        .arg("ci")
        .arg("--output")
        .arg(&output_file);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Pipeline template 'ci' initialized"))
        .stdout(predicate::str::contains(output_file.display().to_string()));

    // Verify file was created
    assert!(output_file.exists());

    // Verify file contains expected content
    let content = fs::read_to_string(&output_file)?;
    assert!(content.contains("name: ci-pipeline"));
    assert!(content.contains("lint"));
    assert!(content.contains("test"));
    assert!(content.contains("build"));

    Ok(())
}

#[test]
fn test_pipeline_init_cd() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_file = temp_dir.path().join("cd-pipeline.yaml");

    let mut cmd = Command::cargo_bin("orchestrate")?;
    cmd.arg("pipeline")
        .arg("init")
        .arg("cd")
        .arg("--output")
        .arg(&output_file);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Pipeline template 'cd' initialized"));

    // Verify file contains expected content
    let content = fs::read_to_string(&output_file)?;
    assert!(content.contains("name: cd-pipeline"));
    assert!(content.contains("deploy-staging"));
    assert!(content.contains("deploy-prod"));

    Ok(())
}

#[test]
fn test_pipeline_init_release() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_file = temp_dir.path().join("release-pipeline.yaml");

    let mut cmd = Command::cargo_bin("orchestrate")?;
    cmd.arg("pipeline")
        .arg("init")
        .arg("release")
        .arg("--output")
        .arg(&output_file);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Pipeline template 'release' initialized"));

    // Verify file contains expected content
    let content = fs::read_to_string(&output_file)?;
    assert!(content.contains("name: release-pipeline"));
    assert!(content.contains("version"));
    assert!(content.contains("changelog"));

    Ok(())
}

#[test]
fn test_pipeline_init_security() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_file = temp_dir.path().join("security-pipeline.yaml");

    let mut cmd = Command::cargo_bin("orchestrate")?;
    cmd.arg("pipeline")
        .arg("init")
        .arg("security")
        .arg("--output")
        .arg(&output_file);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Pipeline template 'security' initialized"));

    // Verify file contains expected content
    let content = fs::read_to_string(&output_file)?;
    assert!(content.contains("name: security-pipeline"));
    assert!(content.contains("scan"));
    assert!(content.contains("report"));

    Ok(())
}

#[test]
fn test_pipeline_init_invalid_template() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_file = temp_dir.path().join("invalid.yaml");

    let mut cmd = Command::cargo_bin("orchestrate")?;
    cmd.arg("pipeline")
        .arg("init")
        .arg("invalid-template")
        .arg("--output")
        .arg(&output_file);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Template not found"));

    // Verify file was not created
    assert!(!output_file.exists());

    Ok(())
}

#[test]
fn test_pipeline_init_list_templates() -> Result<()> {
    let mut cmd = Command::cargo_bin("orchestrate")?;
    cmd.arg("pipeline")
        .arg("init")
        .arg("--list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Available pipeline templates:"))
        .stdout(predicate::str::contains("ci"))
        .stdout(predicate::str::contains("cd"))
        .stdout(predicate::str::contains("release"))
        .stdout(predicate::str::contains("security"));

    Ok(())
}

#[test]
fn test_pipeline_init_default_filename() -> Result<()> {
    let temp_dir = TempDir::new()?;

    // Change to temp directory
    std::env::set_current_dir(&temp_dir)?;

    let mut cmd = Command::cargo_bin("orchestrate")?;
    cmd.arg("pipeline")
        .arg("init")
        .arg("ci");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("ci-pipeline.yaml"));

    // Verify file was created with default name
    let default_file = temp_dir.path().join("ci-pipeline.yaml");
    assert!(default_file.exists());

    Ok(())
}

#[test]
fn test_pipeline_init_overwrite_existing_file() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_file = temp_dir.path().join("test.yaml");

    // Create existing file
    fs::write(&output_file, "existing content")?;

    let mut cmd = Command::cargo_bin("orchestrate")?;
    cmd.arg("pipeline")
        .arg("init")
        .arg("ci")
        .arg("--output")
        .arg(&output_file);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("File already exists"));

    // Verify original content preserved
    let content = fs::read_to_string(&output_file)?;
    assert_eq!(content, "existing content");

    Ok(())
}

#[test]
fn test_pipeline_init_force_overwrite() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let output_file = temp_dir.path().join("test.yaml");

    // Create existing file
    fs::write(&output_file, "existing content")?;

    let mut cmd = Command::cargo_bin("orchestrate")?;
    cmd.arg("pipeline")
        .arg("init")
        .arg("ci")
        .arg("--output")
        .arg(&output_file)
        .arg("--force");

    cmd.assert()
        .success();

    // Verify content was overwritten
    let content = fs::read_to_string(&output_file)?;
    assert!(content.contains("name: ci-pipeline"));
    assert_ne!(content, "existing content");

    Ok(())
}
