//! Integration tests for pipeline parser
//!
//! Tests Story 2 acceptance criteria:
//! - Parse pipeline name, description, triggers
//! - Parse stage definitions with agent, task, conditions
//! - Support `on_failure` actions (halt, continue, rollback)
//! - Support `requires_approval` flag per stage
//! - Support `parallel` stage groups
//! - Support `when` conditions for conditional execution
//! - Validate pipeline structure on create/update

use orchestrate_core::{FailureAction, PipelineDefinition};

#[test]
fn test_acceptance_criteria_parse_name_description_triggers() {
    let yaml = r#"
name: ci-pipeline
description: Continuous integration pipeline
version: 1

triggers:
  - event: pull_request.opened
    branches: [main, develop]
  - event: push
    branches: [main]

stages:
  - name: test
    agent: tester
    task: Run tests
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();

    // Verify name
    assert_eq!(pipeline.name, "ci-pipeline");

    // Verify description
    assert_eq!(pipeline.description, "Continuous integration pipeline");

    // Verify triggers
    assert_eq!(pipeline.triggers.len(), 2);
    assert_eq!(pipeline.triggers[0].event, "pull_request.opened");
    assert_eq!(pipeline.triggers[0].branches, vec!["main", "develop"]);
    assert_eq!(pipeline.triggers[1].event, "push");
    assert_eq!(pipeline.triggers[1].branches, vec!["main"]);
}

#[test]
fn test_acceptance_criteria_parse_stage_definitions() {
    let yaml = r#"
name: test-pipeline
description: Test pipeline

stages:
  - name: build
    agent: builder
    task: Build the project
    timeout: 30m

  - name: test
    agent: tester
    task: Run tests
    depends_on: [build]

  - name: deploy
    agent: deployer
    task: Deploy application
    environment: staging
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();

    assert_eq!(pipeline.stages.len(), 3);

    // Build stage
    let build = &pipeline.stages[0];
    assert_eq!(build.name, "build");
    assert_eq!(build.agent, "builder");
    assert_eq!(build.task, "Build the project");
    assert_eq!(build.timeout, Some("30m".to_string()));

    // Test stage with dependencies
    let test = &pipeline.stages[1];
    assert_eq!(test.name, "test");
    assert_eq!(test.depends_on, vec!["build"]);

    // Deploy stage with environment
    let deploy = &pipeline.stages[2];
    assert_eq!(deploy.environment, Some("staging".to_string()));
}

#[test]
fn test_acceptance_criteria_on_failure_actions() {
    let yaml = r#"
name: failure-handling-pipeline
description: Pipeline with different failure actions

stages:
  - name: validate
    agent: validator
    task: Validate input
    on_failure: halt

  - name: build
    agent: builder
    task: Build project
    on_failure: continue

  - name: deploy
    agent: deployer
    task: Deploy
    on_failure: rollback
    rollback_to: validate
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();

    // Halt action
    assert_eq!(pipeline.stages[0].on_failure, Some(FailureAction::Halt));

    // Continue action
    assert_eq!(pipeline.stages[1].on_failure, Some(FailureAction::Continue));

    // Rollback action with target
    assert_eq!(pipeline.stages[2].on_failure, Some(FailureAction::Rollback));
    assert_eq!(pipeline.stages[2].rollback_to, Some("validate".to_string()));
}

#[test]
fn test_acceptance_criteria_requires_approval() {
    let yaml = r#"
name: approval-pipeline
description: Pipeline with approval gates

stages:
  - name: deploy-staging
    agent: deployer
    task: Deploy to staging
    requires_approval: false

  - name: deploy-prod
    agent: deployer
    task: Deploy to production
    requires_approval: true
    approvers: [team-lead, devops, security]
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();

    // Staging doesn't require approval
    assert!(!pipeline.stages[0].requires_approval);
    assert_eq!(pipeline.stages[0].approvers.len(), 0);

    // Production requires approval
    assert!(pipeline.stages[1].requires_approval);
    assert_eq!(pipeline.stages[1].approvers, vec!["team-lead", "devops", "security"]);
}

#[test]
fn test_acceptance_criteria_parallel_stages() {
    let yaml = r#"
name: parallel-pipeline
description: Pipeline with parallel execution

stages:
  - name: validate
    agent: validator
    task: Validate code

  - name: lint
    agent: linter
    task: Lint code
    parallel_with: validate

  - name: security-scan
    agent: security-scanner
    task: Security scan
    parallel_with: validate

  - name: deploy
    agent: deployer
    task: Deploy
    depends_on: [validate, lint, security-scan]
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();

    // Validate is the base stage
    assert!(pipeline.stages[0].parallel_with.is_none());

    // Lint runs in parallel with validate
    assert_eq!(pipeline.stages[1].parallel_with, Some("validate".to_string()));

    // Security scan runs in parallel with validate
    assert_eq!(pipeline.stages[2].parallel_with, Some("validate".to_string()));

    // Deploy waits for all parallel stages
    assert_eq!(pipeline.stages[3].depends_on.len(), 3);
}

#[test]
fn test_acceptance_criteria_when_conditions() {
    let yaml = r#"
name: conditional-pipeline
description: Pipeline with conditional execution

stages:
  - name: deploy-docs
    agent: doc-deployer
    task: Deploy documentation
    when:
      paths: ["docs/**", "README.md"]

  - name: deploy-backend
    agent: backend-deployer
    task: Deploy backend
    when:
      branch: [main]
      paths: ["backend/**"]

  - name: full-test
    agent: tester
    task: Run full tests
    when:
      labels: ["needs-full-test"]
      or:
        paths: ["src/core/**"]

  - name: security-check
    agent: security
    task: Security check
    when:
      variable:
        env: production
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();

    // Deploy docs based on paths
    let docs_when = pipeline.stages[0].when.as_ref().unwrap();
    assert_eq!(docs_when.paths, Some(vec!["docs/**".to_string(), "README.md".to_string()]));

    // Deploy backend based on branch and paths
    let backend_when = pipeline.stages[1].when.as_ref().unwrap();
    assert_eq!(backend_when.branch, Some(vec!["main".to_string()]));
    assert_eq!(backend_when.paths, Some(vec!["backend/**".to_string()]));

    // Full test with OR condition
    let test_when = pipeline.stages[2].when.as_ref().unwrap();
    assert_eq!(test_when.labels, Some(vec!["needs-full-test".to_string()]));
    assert!(test_when.or.is_some());

    // Security check based on variable
    let security_when = pipeline.stages[3].when.as_ref().unwrap();
    assert!(security_when.variable.is_some());
    let vars = security_when.variable.as_ref().unwrap();
    assert_eq!(vars.get("env"), Some(&"production".to_string()));
}

#[test]
fn test_acceptance_criteria_validate_pipeline_structure() {
    // Valid pipeline should pass
    let valid_yaml = r#"
name: valid-pipeline
description: Valid pipeline
stages:
  - name: build
    agent: builder
    task: Build
"#;
    assert!(PipelineDefinition::from_yaml_str(valid_yaml).is_ok());

    // Empty name should fail
    let empty_name = r#"
name: ""
description: Test
stages:
  - name: build
    agent: builder
    task: Build
"#;
    assert!(PipelineDefinition::from_yaml_str(empty_name).is_err());

    // No stages should fail
    let no_stages = r#"
name: test
description: Test
stages: []
"#;
    assert!(PipelineDefinition::from_yaml_str(no_stages).is_err());

    // Invalid dependency should fail
    let invalid_dep = r#"
name: test
description: Test
stages:
  - name: build
    agent: builder
    task: Build
    depends_on: [nonexistent]
"#;
    assert!(PipelineDefinition::from_yaml_str(invalid_dep).is_err());

    // Circular dependency should fail
    let circular = r#"
name: test
description: Test
stages:
  - name: stage1
    agent: agent1
    task: Task 1
    depends_on: [stage2]
  - name: stage2
    agent: agent2
    task: Task 2
    depends_on: [stage1]
"#;
    assert!(PipelineDefinition::from_yaml_str(circular).is_err());

    // Invalid rollback should fail
    let invalid_rollback = r#"
name: test
description: Test
stages:
  - name: deploy
    agent: deployer
    task: Deploy
    rollback_to: build
"#;
    assert!(PipelineDefinition::from_yaml_str(invalid_rollback).is_err());

    // Approval without approvers should fail
    let no_approvers = r#"
name: test
description: Test
stages:
  - name: deploy
    agent: deployer
    task: Deploy
    requires_approval: true
"#;
    assert!(PipelineDefinition::from_yaml_str(no_approvers).is_err());
}

#[test]
fn test_complete_feature_pipeline_example() {
    // The exact example from the epic documentation
    let yaml = r#"
name: feature-complete
description: Full deployment pipeline after feature merge
version: 1

triggers:
  - event: pull_request.merged
    branches: [main]

variables:
  environment: staging

stages:
  - name: validate
    agent: regression-tester
    task: "Run full test suite"
    timeout: 30m
    on_failure: halt

  - name: security-scan
    agent: security-scanner
    task: "Run security analysis"
    parallel_with: validate
    on_failure: halt

  - name: deploy-staging
    agent: deployer
    task: "Deploy to ${environment}"
    environment: staging
    depends_on: [validate, security-scan]

  - name: smoke-test
    agent: smoke-tester
    task: "Run smoke tests on staging"
    depends_on: [deploy-staging]
    on_failure: rollback
    rollback_to: deploy-staging

  - name: deploy-prod
    agent: deployer
    task: "Deploy to production"
    environment: production
    requires_approval: true
    approvers: [team-lead, devops]
    depends_on: [smoke-test]
"#;

    let pipeline = PipelineDefinition::from_yaml_str(yaml).unwrap();

    // Validate all aspects of the example pipeline
    assert_eq!(pipeline.name, "feature-complete");
    assert_eq!(pipeline.description, "Full deployment pipeline after feature merge");
    assert_eq!(pipeline.version, 1);

    // Triggers
    assert_eq!(pipeline.triggers.len(), 1);
    assert_eq!(pipeline.triggers[0].event, "pull_request.merged");
    assert_eq!(pipeline.triggers[0].branches, vec!["main"]);

    // Variables
    assert_eq!(pipeline.variables.get("environment"), Some(&"staging".to_string()));

    // Stages
    assert_eq!(pipeline.stages.len(), 5);

    // Validate stage
    let validate = &pipeline.stages[0];
    assert_eq!(validate.name, "validate");
    assert_eq!(validate.agent, "regression-tester");
    assert_eq!(validate.timeout, Some("30m".to_string()));
    assert_eq!(validate.on_failure, Some(FailureAction::Halt));

    // Security scan (parallel)
    let security = &pipeline.stages[1];
    assert_eq!(security.parallel_with, Some("validate".to_string()));
    assert_eq!(security.on_failure, Some(FailureAction::Halt));

    // Deploy staging (depends on parallel stages)
    let deploy_staging = &pipeline.stages[2];
    assert_eq!(deploy_staging.depends_on, vec!["validate", "security-scan"]);
    assert_eq!(deploy_staging.environment, Some("staging".to_string()));

    // Smoke test (with rollback)
    let smoke_test = &pipeline.stages[3];
    assert_eq!(smoke_test.depends_on, vec!["deploy-staging"]);
    assert_eq!(smoke_test.on_failure, Some(FailureAction::Rollback));
    assert_eq!(smoke_test.rollback_to, Some("deploy-staging".to_string()));

    // Deploy prod (with approval)
    let deploy_prod = &pipeline.stages[4];
    assert!(deploy_prod.requires_approval);
    assert_eq!(deploy_prod.approvers, vec!["team-lead", "devops"]);
    assert_eq!(deploy_prod.environment, Some("production".to_string()));
    assert_eq!(deploy_prod.depends_on, vec!["smoke-test"]);
}

#[test]
fn test_serialization_roundtrip() {
    let original_yaml = r#"
name: test-pipeline
description: Test pipeline
version: 1
triggers:
  - event: push
    branches: [main]
variables:
  key: value
stages:
  - name: build
    agent: builder
    task: Build
    timeout: 10m
    on_failure: halt
"#;

    // Parse from YAML
    let pipeline = PipelineDefinition::from_yaml_str(original_yaml).unwrap();

    // Serialize back to YAML
    let serialized = pipeline.to_yaml_string().unwrap();

    // Parse again
    let reparsed = PipelineDefinition::from_yaml_str(&serialized).unwrap();

    // Should be identical
    assert_eq!(pipeline, reparsed);
}
