//! Integration tests for Pipeline Execution Engine
//!
//! Tests complete pipeline execution scenarios end-to-end.

use orchestrate_core::{
    Database, ExecutionContext, FailureAction, PipelineDefinition, PipelineExecutor,
    PipelineRunStatus, PipelineStageStatus, StageDefinition,
};
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::test]
async fn test_complete_ci_cd_pipeline() {
    let database = Arc::new(Database::in_memory().await.unwrap());
    let executor = PipelineExecutor::new(database.clone());

    // Create a realistic CI/CD pipeline
    let mut variables = HashMap::new();
    variables.insert("environment".to_string(), "staging".to_string());
    variables.insert("version".to_string(), "1.2.3".to_string());

    let definition = PipelineDefinition {
        name: "ci-cd-pipeline".to_string(),
        description: "Complete CI/CD pipeline".to_string(),
        version: 1,
        triggers: vec![],
        variables,
        stages: vec![
            // Stage 1: Lint (no dependencies)
            StageDefinition {
                name: "lint".to_string(),
                agent: "linter".to_string(),
                task: "Run code linter".to_string(),
                timeout: Some("5m".to_string()),
                on_failure: Some(FailureAction::Halt),
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec![],
                parallel_with: None,
                when: None,
            },
            // Stage 2: Test (runs in parallel with lint)
            StageDefinition {
                name: "test".to_string(),
                agent: "tester".to_string(),
                task: "Run unit tests".to_string(),
                timeout: Some("10m".to_string()),
                on_failure: Some(FailureAction::Halt),
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec![],
                parallel_with: Some("lint".to_string()),
                when: None,
            },
            // Stage 3: Build (depends on lint and test)
            StageDefinition {
                name: "build".to_string(),
                agent: "builder".to_string(),
                task: "Build version ${version}".to_string(), // Variable substitution
                timeout: Some("15m".to_string()),
                on_failure: Some(FailureAction::Halt),
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec!["lint".to_string(), "test".to_string()],
                parallel_with: None,
                when: None,
            },
            // Stage 4: Security scan (depends on build)
            StageDefinition {
                name: "security-scan".to_string(),
                agent: "security-scanner".to_string(),
                task: "Run security scan".to_string(),
                timeout: Some("10m".to_string()),
                on_failure: Some(FailureAction::Halt),
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec!["build".to_string()],
                parallel_with: None,
                when: None,
            },
            // Stage 5: Deploy to staging (depends on security scan)
            StageDefinition {
                name: "deploy-staging".to_string(),
                agent: "deployer".to_string(),
                task: "Deploy to ${environment}".to_string(), // Variable substitution
                timeout: Some("20m".to_string()),
                on_failure: Some(FailureAction::Halt),
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: Some("staging".to_string()),
                depends_on: vec!["security-scan".to_string()],
                parallel_with: None,
                when: None,
            },
            // Stage 6: Smoke tests (depends on deploy)
            StageDefinition {
                name: "smoke-test".to_string(),
                agent: "smoke-tester".to_string(),
                task: "Run smoke tests on ${environment}".to_string(),
                timeout: Some("5m".to_string()),
                on_failure: Some(FailureAction::Halt),
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec!["deploy-staging".to_string()],
                parallel_with: None,
                when: None,
            },
        ],
    };

    // Create pipeline in database
    let pipeline = orchestrate_core::Pipeline::new(
        definition.name.clone(),
        definition.to_yaml_string().unwrap(),
    );
    let pipeline_id = database.insert_pipeline(&pipeline).await.unwrap();

    // Create and execute a run
    let run_id = executor
        .create_run(pipeline_id, Some("pull_request.merged".to_string()))
        .await
        .unwrap();

    let result = executor.execute_run(run_id, &definition).await;
    assert!(result.is_ok(), "Pipeline execution should succeed");

    // Verify run status
    let run = database.get_pipeline_run(run_id).await.unwrap().unwrap();
    assert_eq!(run.status, PipelineRunStatus::Succeeded);
    assert!(run.started_at.is_some());
    assert!(run.completed_at.is_some());
    assert_eq!(run.trigger_event, Some("pull_request.merged".to_string()));

    // Verify all stages executed
    let stages = database.list_pipeline_stages(run_id).await.unwrap();
    assert_eq!(stages.len(), 6);

    // Verify each stage succeeded
    for stage in &stages {
        assert_eq!(
            stage.status,
            PipelineStageStatus::Succeeded,
            "Stage {} should have succeeded",
            stage.stage_name
        );
        assert!(stage.started_at.is_some());
        assert!(stage.completed_at.is_some());
    }

    // Verify stages exist with correct names
    let stage_names: Vec<String> = stages.iter().map(|s| s.stage_name.clone()).collect();
    assert!(stage_names.contains(&"lint".to_string()));
    assert!(stage_names.contains(&"test".to_string()));
    assert!(stage_names.contains(&"build".to_string()));
    assert!(stage_names.contains(&"security-scan".to_string()));
    assert!(stage_names.contains(&"deploy-staging".to_string()));
    assert!(stage_names.contains(&"smoke-test".to_string()));
}

#[tokio::test]
async fn test_pipeline_with_timeout() {
    let database = Arc::new(Database::in_memory().await.unwrap());
    let executor = PipelineExecutor::new(database.clone());

    let definition = PipelineDefinition {
        name: "timeout-pipeline".to_string(),
        description: "Pipeline with timeout".to_string(),
        version: 1,
        triggers: vec![],
        variables: HashMap::new(),
        stages: vec![StageDefinition {
            name: "quick-task".to_string(),
            agent: "worker".to_string(),
            task: "Quick task".to_string(),
            timeout: Some("1h".to_string()), // 1 hour timeout
            on_failure: None,
            rollback_to: None,
            requires_approval: false,
            approvers: vec![],
            environment: None,
            depends_on: vec![],
            parallel_with: None,
            when: None,
        }],
    };

    let pipeline = orchestrate_core::Pipeline::new(
        definition.name.clone(),
        definition.to_yaml_string().unwrap(),
    );
    let pipeline_id = database.insert_pipeline(&pipeline).await.unwrap();
    let run_id = executor.create_run(pipeline_id, None).await.unwrap();

    let result = executor.execute_run(run_id, &definition).await;
    assert!(result.is_ok(), "Pipeline with timeout should succeed");

    let run = database.get_pipeline_run(run_id).await.unwrap().unwrap();
    assert_eq!(run.status, PipelineRunStatus::Succeeded);
}

#[tokio::test]
async fn test_execution_context_variable_substitution() {
    let mut context = ExecutionContext::new();
    context.set_variable("env".to_string(), "production".to_string());
    context.set_variable("region".to_string(), "us-west-2".to_string());
    context.set_variable("version".to_string(), "2.0.0".to_string());

    let template = "Deploy version ${version} to ${env} in ${region}";
    let result = context.substitute_variables(template);

    assert_eq!(result, "Deploy version 2.0.0 to production in us-west-2");
}

#[tokio::test]
async fn test_multiple_parallel_stages() {
    let database = Arc::new(Database::in_memory().await.unwrap());
    let executor = PipelineExecutor::new(database.clone());

    let definition = PipelineDefinition {
        name: "parallel-pipeline".to_string(),
        description: "Pipeline with multiple parallel stages".to_string(),
        version: 1,
        triggers: vec![],
        variables: HashMap::new(),
        stages: vec![
            StageDefinition {
                name: "init".to_string(),
                agent: "initializer".to_string(),
                task: "Initialize".to_string(),
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec![],
                parallel_with: None,
                when: None,
            },
            // Three stages that run in parallel after init
            StageDefinition {
                name: "parallel-a".to_string(),
                agent: "worker-a".to_string(),
                task: "Task A".to_string(),
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec!["init".to_string()],
                parallel_with: None,
                when: None,
            },
            StageDefinition {
                name: "parallel-b".to_string(),
                agent: "worker-b".to_string(),
                task: "Task B".to_string(),
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec!["init".to_string()],
                parallel_with: Some("parallel-a".to_string()),
                when: None,
            },
            StageDefinition {
                name: "parallel-c".to_string(),
                agent: "worker-c".to_string(),
                task: "Task C".to_string(),
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec!["init".to_string()],
                parallel_with: Some("parallel-a".to_string()),
                when: None,
            },
            // Final stage that depends on all parallel stages
            StageDefinition {
                name: "finalize".to_string(),
                agent: "finalizer".to_string(),
                task: "Finalize".to_string(),
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec![
                    "parallel-a".to_string(),
                    "parallel-b".to_string(),
                    "parallel-c".to_string(),
                ],
                parallel_with: None,
                when: None,
            },
        ],
    };

    let pipeline = orchestrate_core::Pipeline::new(
        definition.name.clone(),
        definition.to_yaml_string().unwrap(),
    );
    let pipeline_id = database.insert_pipeline(&pipeline).await.unwrap();
    let run_id = executor.create_run(pipeline_id, None).await.unwrap();

    let result = executor.execute_run(run_id, &definition).await;
    assert!(result.is_ok(), "Parallel pipeline should succeed");

    let stages = database.list_pipeline_stages(run_id).await.unwrap();
    assert_eq!(stages.len(), 5);

    // All stages should have succeeded
    for stage in stages {
        assert_eq!(stage.status, PipelineStageStatus::Succeeded);
    }
}

#[tokio::test]
async fn test_complex_dependency_graph() {
    let database = Arc::new(Database::in_memory().await.unwrap());
    let executor = PipelineExecutor::new(database.clone());

    // Build a diamond-shaped dependency graph:
    //     start
    //    /     \
    //  left   right
    //    \     /
    //      end

    let definition = PipelineDefinition {
        name: "diamond-pipeline".to_string(),
        description: "Pipeline with diamond dependency graph".to_string(),
        version: 1,
        triggers: vec![],
        variables: HashMap::new(),
        stages: vec![
            StageDefinition {
                name: "start".to_string(),
                agent: "starter".to_string(),
                task: "Start".to_string(),
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec![],
                parallel_with: None,
                when: None,
            },
            StageDefinition {
                name: "left".to_string(),
                agent: "left-worker".to_string(),
                task: "Left path".to_string(),
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec!["start".to_string()],
                parallel_with: None,
                when: None,
            },
            StageDefinition {
                name: "right".to_string(),
                agent: "right-worker".to_string(),
                task: "Right path".to_string(),
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec!["start".to_string()],
                parallel_with: Some("left".to_string()),
                when: None,
            },
            StageDefinition {
                name: "end".to_string(),
                agent: "ender".to_string(),
                task: "End".to_string(),
                timeout: None,
                on_failure: None,
                rollback_to: None,
                requires_approval: false,
                approvers: vec![],
                environment: None,
                depends_on: vec!["left".to_string(), "right".to_string()],
                parallel_with: None,
                when: None,
            },
        ],
    };

    let pipeline = orchestrate_core::Pipeline::new(
        definition.name.clone(),
        definition.to_yaml_string().unwrap(),
    );
    let pipeline_id = database.insert_pipeline(&pipeline).await.unwrap();
    let run_id = executor.create_run(pipeline_id, None).await.unwrap();

    let result = executor.execute_run(run_id, &definition).await;
    assert!(result.is_ok(), "Diamond pipeline should succeed");

    let stages = database.list_pipeline_stages(run_id).await.unwrap();
    assert_eq!(stages.len(), 4);

    // Verify execution order by checking timestamps
    let start = stages.iter().find(|s| s.stage_name == "start").unwrap();
    let left = stages.iter().find(|s| s.stage_name == "left").unwrap();
    let right = stages.iter().find(|s| s.stage_name == "right").unwrap();
    let end = stages.iter().find(|s| s.stage_name == "end").unwrap();

    // Start should complete before left and right begin
    assert!(start.completed_at.unwrap() <= left.started_at.unwrap());
    assert!(start.completed_at.unwrap() <= right.started_at.unwrap());

    // Left and right should complete before end begins
    assert!(left.completed_at.unwrap() <= end.started_at.unwrap());
    assert!(right.completed_at.unwrap() <= end.started_at.unwrap());
}
