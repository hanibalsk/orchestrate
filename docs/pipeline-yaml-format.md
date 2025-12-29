# Pipeline YAML Format

This document describes the YAML format for defining event-driven pipelines in Orchestrate.

## Overview

Pipelines are defined in YAML and stored in the database. They orchestrate multiple agents in sequence or parallel, with conditional logic, approval gates, and rollback capabilities.

## Pipeline Structure

```yaml
name: pipeline-name
description: Pipeline description
version: 1

triggers:
  - event: event-type
    branches: [branch-list]

variables:
  key: value

stages:
  - name: stage-name
    agent: agent-name
    task: "Task description"
    # ... stage configuration
```

## Top-Level Fields

### Required Fields

- `name` (string): Unique pipeline identifier
- `description` (string): Human-readable description
- `stages` (array): List of stage definitions (must have at least one)

### Optional Fields

- `version` (integer): Pipeline version (default: 1)
- `triggers` (array): Event triggers that activate this pipeline
- `variables` (object): Key-value pairs for pipeline-wide variables

## Triggers

Triggers define which events activate the pipeline:

```yaml
triggers:
  - event: pull_request.merged
    branches: [main, develop]
  - event: push
    branches: [main]
```

Fields:
- `event` (string): Event type (e.g., "pull_request.merged", "push")
- `branches` (array): Branch names to trigger on (optional)

## Variables

Define pipeline-wide variables that can be referenced in tasks:

```yaml
variables:
  environment: staging
  region: us-west-2
```

Variables can be referenced in task descriptions using `${variable_name}` syntax.

## Stages

Stages define the individual steps in the pipeline.

### Required Stage Fields

- `name` (string): Unique stage name within the pipeline
- `agent` (string): Agent to execute this stage
- `task` (string): Task description for the agent

### Optional Stage Fields

- `timeout` (string): Maximum execution time (e.g., "30m", "1h")
- `on_failure` (string): Action on failure ("halt", "continue", "rollback")
- `rollback_to` (string): Stage name to rollback to (requires `on_failure: rollback`)
- `requires_approval` (boolean): Whether this stage requires human approval
- `approvers` (array): List of approver identifiers (required if `requires_approval: true`)
- `environment` (string): Environment identifier (e.g., "staging", "production")
- `depends_on` (array): List of stage names that must complete before this stage
- `parallel_with` (string): Stage name to run in parallel with
- `when` (object): Conditional execution rules

## Failure Actions

Control what happens when a stage fails:

```yaml
stages:
  - name: validate
    agent: validator
    task: Validate input
    on_failure: halt  # Stop the entire pipeline

  - name: optional-check
    agent: checker
    task: Optional check
    on_failure: continue  # Continue to next stage

  - name: deploy
    agent: deployer
    task: Deploy application
    on_failure: rollback
    rollback_to: backup  # Rollback to backup stage
```

## Approval Gates

Pause pipeline execution for human approval:

```yaml
stages:
  - name: deploy-prod
    agent: deployer
    task: Deploy to production
    requires_approval: true
    approvers: [team-lead, devops, security]
```

At least one approver must be specified when `requires_approval: true`.

## Stage Dependencies

Define execution order using `depends_on`:

```yaml
stages:
  - name: build
    agent: builder
    task: Build application

  - name: test
    agent: tester
    task: Run tests
    depends_on: [build]

  - name: deploy
    agent: deployer
    task: Deploy
    depends_on: [test]
```

## Parallel Execution

Run stages in parallel using `parallel_with`:

```yaml
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
```

All stages running in parallel must complete before dependent stages execute.

## Conditional Execution

Use `when` conditions to execute stages conditionally:

```yaml
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
```

### Condition Types

- `branch` (array): Match if branch is in list
- `paths` (array): Match if any changed file matches patterns
- `labels` (array): Match if any label is present
- `variable` (object): Match if variable values match
- `or` (object): Alternative condition (logical OR)

Conditions at the same level are combined with AND logic. Use nested `or` for alternative conditions.

## Complete Example

```yaml
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
```

## Validation Rules

The pipeline parser validates:

1. Pipeline name is not empty
2. At least one stage is defined
3. Each stage has a name, agent, and task
4. All `depends_on` references exist
5. All `parallel_with` references exist
6. All `rollback_to` references exist
7. `rollback_to` is only used with `on_failure: rollback`
8. `requires_approval: true` has at least one approver
9. No circular dependencies in stage graph

## Error Messages

Common validation errors:

- "Pipeline name cannot be empty"
- "Pipeline must have at least one stage"
- "Stage name cannot be empty"
- "Stage 'X' must specify an agent"
- "Stage 'X' must specify a task"
- "Stage 'X' depends on non-existent stage 'Y'"
- "Stage 'X' parallel_with non-existent stage 'Y'"
- "Stage 'X' has rollback_to but on_failure is not 'rollback'"
- "Stage 'X' rollback_to non-existent stage 'Y'"
- "Stage 'X' requires approval but has no approvers"
- "Circular dependency detected involving stage 'X'"

## API Usage

### Parse from YAML String

```rust
use orchestrate_core::PipelineDefinition;

let yaml = r#"
name: test-pipeline
description: Test pipeline
stages:
  - name: build
    agent: builder
    task: Build project
"#;

let pipeline = PipelineDefinition::from_yaml_str(yaml)?;
```

### Parse from File

```rust
use orchestrate_core::PipelineDefinition;

let pipeline = PipelineDefinition::from_yaml_file("pipeline.yaml")?;
```

### Serialize to YAML

```rust
let yaml = pipeline.to_yaml_string()?;
```

### Validation

Validation occurs automatically during parsing. To validate an existing pipeline:

```rust
pipeline.validate()?;
```
