//! Integration tests for pipeline CLI commands

use orchestrate_core::{Database, Pipeline, PipelineRun, PipelineRunStatus};

/// Helper to create a test database
async fn create_test_db() -> Database {
    Database::in_memory().await.unwrap()
}

/// Helper to create a sample pipeline YAML
fn create_sample_pipeline_yaml() -> String {
    r#"
name: test-pipeline
description: Test pipeline for CLI
version: 1

triggers:
  - event: pull_request.merged
    branches: [main]

stages:
  - name: validate
    agent: tester
    task: "Run tests"
    timeout: 10m
"#
    .to_string()
}

#[tokio::test]
async fn test_pipeline_create_from_yaml() {
    let db = create_test_db().await;
    let yaml = create_sample_pipeline_yaml();

    // Create pipeline
    let pipeline = Pipeline::new("test-pipeline".to_string(), yaml.clone());
    let id = db.insert_pipeline(&pipeline).await.unwrap();

    // Verify it was created
    let loaded = db.get_pipeline(id).await.unwrap().unwrap();
    assert_eq!(loaded.name, "test-pipeline");
    assert_eq!(loaded.definition, yaml);
    assert!(loaded.enabled);
}

#[tokio::test]
async fn test_pipeline_list() {
    let db = create_test_db().await;

    // Create multiple pipelines
    let pipeline1 = Pipeline::new("pipeline-1".to_string(), create_sample_pipeline_yaml());
    let pipeline2 = Pipeline::new("pipeline-2".to_string(), create_sample_pipeline_yaml());

    db.insert_pipeline(&pipeline1).await.unwrap();
    db.insert_pipeline(&pipeline2).await.unwrap();

    // List pipelines
    let pipelines = db.list_pipelines().await.unwrap();
    assert_eq!(pipelines.len(), 2);
    assert_eq!(pipelines[0].name, "pipeline-1");
    assert_eq!(pipelines[1].name, "pipeline-2");
}

#[tokio::test]
async fn test_pipeline_show() {
    let db = create_test_db().await;
    let yaml = create_sample_pipeline_yaml();

    // Create pipeline
    let pipeline = Pipeline::new("test-pipeline".to_string(), yaml.clone());
    db.insert_pipeline(&pipeline).await.unwrap();

    // Show pipeline by name
    let loaded = db.get_pipeline_by_name("test-pipeline").await.unwrap().unwrap();
    assert_eq!(loaded.name, "test-pipeline");
    assert_eq!(loaded.definition, yaml);
}

#[tokio::test]
async fn test_pipeline_update() {
    let db = create_test_db().await;
    let yaml = create_sample_pipeline_yaml();

    // Create pipeline
    let mut pipeline = Pipeline::new("test-pipeline".to_string(), yaml);
    let id = db.insert_pipeline(&pipeline).await.unwrap();

    // Update pipeline
    let new_yaml = r#"
name: test-pipeline
description: Updated pipeline
version: 2
"#
    .to_string();

    pipeline.id = Some(id);
    pipeline.definition = new_yaml.clone();
    db.update_pipeline(&pipeline).await.unwrap();

    // Verify update
    let loaded = db.get_pipeline(id).await.unwrap().unwrap();
    assert_eq!(loaded.definition, new_yaml);
}

#[tokio::test]
async fn test_pipeline_delete() {
    let db = create_test_db().await;

    // Create pipeline
    let pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let id = db.insert_pipeline(&pipeline).await.unwrap();

    // Delete pipeline
    db.delete_pipeline(id).await.unwrap();

    // Verify deletion
    let loaded = db.get_pipeline(id).await.unwrap();
    assert!(loaded.is_none());
}

#[tokio::test]
async fn test_pipeline_enable_disable() {
    let db = create_test_db().await;

    // Create pipeline
    let mut pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let id = db.insert_pipeline(&pipeline).await.unwrap();

    // Disable pipeline
    pipeline.id = Some(id);
    pipeline.enabled = false;
    db.update_pipeline(&pipeline).await.unwrap();

    let loaded = db.get_pipeline(id).await.unwrap().unwrap();
    assert!(!loaded.enabled);

    // Enable pipeline
    pipeline.enabled = true;
    db.update_pipeline(&pipeline).await.unwrap();

    let loaded = db.get_pipeline(id).await.unwrap().unwrap();
    assert!(loaded.enabled);
}

#[tokio::test]
async fn test_pipeline_run_manual_trigger() {
    let db = create_test_db().await;

    // Create pipeline
    let pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

    // Trigger manual run
    let run = PipelineRun::new(pipeline_id, Some("manual".to_string()));
    let run_id = db.insert_pipeline_run(&run).await.unwrap();

    // Verify run was created
    let loaded = db.get_pipeline_run(run_id).await.unwrap().unwrap();
    assert_eq!(loaded.pipeline_id, pipeline_id);
    assert_eq!(loaded.status, PipelineRunStatus::Pending);
    assert_eq!(loaded.trigger_event, Some("manual".to_string()));
}

#[tokio::test]
async fn test_pipeline_run_status() {
    let db = create_test_db().await;

    // Create pipeline and run
    let pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();
    let run = PipelineRun::new(pipeline_id, None);
    let run_id = db.insert_pipeline_run(&run).await.unwrap();

    // Get run status
    let loaded = db.get_pipeline_run(run_id).await.unwrap().unwrap();
    assert_eq!(loaded.status, PipelineRunStatus::Pending);
    assert!(loaded.started_at.is_none());
    assert!(loaded.completed_at.is_none());
}

#[tokio::test]
async fn test_pipeline_cancel_run() {
    let db = create_test_db().await;

    // Create pipeline and run
    let pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();
    let mut run = PipelineRun::new(pipeline_id, None);
    run.mark_running();
    let run_id = db.insert_pipeline_run(&run).await.unwrap();

    // Cancel run
    run.id = Some(run_id);
    run.mark_cancelled();
    db.update_pipeline_run(&run).await.unwrap();

    // Verify cancellation
    let loaded = db.get_pipeline_run(run_id).await.unwrap().unwrap();
    assert_eq!(loaded.status, PipelineRunStatus::Cancelled);
    assert!(loaded.completed_at.is_some());
}

#[tokio::test]
async fn test_pipeline_history() {
    let db = create_test_db().await;

    // Create pipeline
    let pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();

    // Create multiple runs
    let mut run1 = PipelineRun::new(pipeline_id, None);
    run1.mark_running();
    run1.mark_succeeded();
    db.insert_pipeline_run(&run1).await.unwrap();

    let mut run2 = PipelineRun::new(pipeline_id, None);
    run2.mark_running();
    run2.mark_failed();
    db.insert_pipeline_run(&run2).await.unwrap();

    // Get history
    let runs = db.list_pipeline_runs(pipeline_id).await.unwrap();
    assert_eq!(runs.len(), 2);
    // Runs are ordered by created_at, so check the actual statuses
    let statuses: Vec<_> = runs.iter().map(|r| r.status).collect();
    assert!(statuses.contains(&PipelineRunStatus::Succeeded));
    assert!(statuses.contains(&PipelineRunStatus::Failed));
}

#[tokio::test]
async fn test_approval_list_pending() {
    let db = create_test_db().await;

    // Create pipeline, run, and stage
    let pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();
    let run = PipelineRun::new(pipeline_id, None);
    let run_id = db.insert_pipeline_run(&run).await.unwrap();

    // Create a stage for the approval
    let stage = orchestrate_core::PipelineStage::new(run_id, "deploy".to_string());
    let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

    // Create approval request
    let approval = orchestrate_core::ApprovalRequest::new(
        stage_id,
        run_id,
        "user@example.com".to_string(),
        1,
        None,
        None,
    );
    let _created = db.create_approval_request(approval).await.unwrap();

    // List pending approvals
    let pending = db.list_pending_approvals().await.unwrap();
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].status, orchestrate_core::ApprovalStatus::Pending);
}

#[tokio::test]
async fn test_approval_approve() {
    let db = create_test_db().await;

    // Create pipeline, run, and stage
    let pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();
    let run = PipelineRun::new(pipeline_id, None);
    let run_id = db.insert_pipeline_run(&run).await.unwrap();
    let stage = orchestrate_core::PipelineStage::new(run_id, "deploy".to_string());
    let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

    // Create approval request
    let approval = orchestrate_core::ApprovalRequest::new(
        stage_id,
        run_id,
        "user@example.com".to_string(),
        1,
        None,
        None,
    );
    let mut approval = db.create_approval_request(approval).await.unwrap();
    let id = approval.id.unwrap();

    // Create approval decision
    let decision = orchestrate_core::ApprovalDecision::new(
        id,
        "user@example.com".to_string(),
        true,
        Some("LGTM".to_string()),
    );
    db.create_approval_decision(decision).await.unwrap();

    // Mark approval as approved
    approval.approval_count = 1;
    approval.mark_approved();
    db.update_approval_request(&approval).await.unwrap();

    // Verify approval
    let loaded = db.get_approval_request(id).await.unwrap().unwrap();
    assert_eq!(loaded.status, orchestrate_core::ApprovalStatus::Approved);
}

#[tokio::test]
async fn test_approval_reject() {
    let db = create_test_db().await;

    // Create pipeline, run, and stage
    let pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();
    let run = PipelineRun::new(pipeline_id, None);
    let run_id = db.insert_pipeline_run(&run).await.unwrap();
    let stage = orchestrate_core::PipelineStage::new(run_id, "deploy".to_string());
    let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

    // Create approval request
    let approval = orchestrate_core::ApprovalRequest::new(
        stage_id,
        run_id,
        "user@example.com".to_string(),
        1,
        None,
        None,
    );
    let mut approval = db.create_approval_request(approval).await.unwrap();
    let id = approval.id.unwrap();

    // Create rejection decision
    let decision = orchestrate_core::ApprovalDecision::new(
        id,
        "user@example.com".to_string(),
        false,
        Some("Needs more testing".to_string()),
    );
    db.create_approval_decision(decision).await.unwrap();

    // Mark approval as rejected
    approval.rejection_count = 1;
    approval.mark_rejected();
    db.update_approval_request(&approval).await.unwrap();

    // Verify rejection
    let loaded = db.get_approval_request(id).await.unwrap().unwrap();
    assert_eq!(loaded.status, orchestrate_core::ApprovalStatus::Rejected);
}

#[tokio::test]
async fn test_approval_delegate() {
    let db = create_test_db().await;

    // Create pipeline, run, and stage
    let pipeline = Pipeline::new("test-pipeline".to_string(), create_sample_pipeline_yaml());
    let pipeline_id = db.insert_pipeline(&pipeline).await.unwrap();
    let run = PipelineRun::new(pipeline_id, None);
    let run_id = db.insert_pipeline_run(&run).await.unwrap();
    let stage = orchestrate_core::PipelineStage::new(run_id, "deploy".to_string());
    let stage_id = db.insert_pipeline_stage(&stage).await.unwrap();

    // Create approval request
    let approval = orchestrate_core::ApprovalRequest::new(
        stage_id,
        run_id,
        "user1@example.com".to_string(),
        1,
        None,
        None,
    );
    let mut approval = db.create_approval_request(approval).await.unwrap();
    let id = approval.id.unwrap();

    // Delegate to another user
    approval.required_approvers = "user2@example.com".to_string();
    approval.mark_delegated();
    db.update_approval_request(&approval).await.unwrap();

    // Verify delegation
    let loaded = db.get_approval_request(id).await.unwrap().unwrap();
    assert_eq!(loaded.status, orchestrate_core::ApprovalStatus::Delegated);
    assert_eq!(loaded.required_approvers, "user2@example.com");
}
