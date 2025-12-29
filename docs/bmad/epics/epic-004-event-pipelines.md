# Epic 004: Event-Driven Pipelines

Implement complex multi-stage workflows triggered by events with conditional branching.

**Priority:** High
**Effort:** Large
**Use Cases:** UC-103, UC-104

## Overview

Enable definition of sophisticated pipelines that orchestrate multiple agents in sequence or parallel, with conditional logic, approval gates, and rollback capabilities. Pipelines are defined in YAML and triggered by events or manually.

## Stories

### Story 1: Pipeline Data Model

Create database schema and domain model for pipelines.

**Acceptance Criteria:**
- [x] Create `pipelines` table: id, name, definition (YAML), enabled, created_at
- [x] Create `pipeline_runs` table: id, pipeline_id, status, trigger_event, started_at, completed_at
- [x] Create `pipeline_stages` table: id, run_id, stage_name, status, agent_id, started_at, completed_at
- [x] Implement Pipeline, PipelineRun, PipelineStage structs
- [x] Add migration for tables

**Pipeline Run Statuses:** Pending, Running, WaitingApproval, Succeeded, Failed, Cancelled

### Story 2: Pipeline YAML Parser

Parse pipeline definitions from YAML.

**Acceptance Criteria:**
- [ ] Parse pipeline name, description, triggers
- [ ] Parse stage definitions with agent, task, conditions
- [ ] Support `on_failure` actions (halt, continue, rollback)
- [ ] Support `requires_approval` flag per stage
- [ ] Support `parallel` stage groups
- [ ] Support `when` conditions for conditional execution
- [ ] Validate pipeline structure on create/update

**Example Pipeline:**
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
    depends_on: deploy-staging
    on_failure: rollback
    rollback_to: deploy-staging

  - name: deploy-prod
    agent: deployer
    task: "Deploy to production"
    environment: production
    requires_approval: true
    approvers: [team-lead, devops]
    depends_on: smoke-test
```

### Story 3: Pipeline Execution Engine

Execute pipelines by running stages in order.

**Acceptance Criteria:**
- [ ] Create pipeline run from trigger
- [ ] Execute stages respecting dependencies (DAG)
- [ ] Run parallel stages concurrently
- [ ] Spawn agents for each stage
- [ ] Track stage status and timing
- [ ] Handle stage timeouts
- [ ] Support stage retry on failure
- [ ] Pass variables between stages

### Story 4: Conditional Execution

Support conditional stage execution based on context.

**Acceptance Criteria:**
- [ ] Parse `when` conditions in stage definitions
- [ ] Support conditions: branch, path, label, variable
- [ ] Skip stages when condition not met
- [ ] Log reason for skipped stages
- [ ] Support complex conditions (and, or, not)

**Examples:**
```yaml
stages:
  - name: deploy-docs
    when:
      paths: ["docs/**", "README.md"]
    agent: doc-deployer

  - name: full-test
    when:
      labels: ["needs-full-test"]
      or:
        paths: ["src/core/**"]
```

### Story 5: Approval Gates

Implement human-in-the-loop approval for critical stages.

**Acceptance Criteria:**
- [ ] Pause pipeline at stages with `requires_approval: true`
- [ ] Create approval request in database
- [ ] Notify approvers via configured channel
- [ ] Support multiple approvers with quorum
- [ ] Implement approval timeout with default action
- [ ] Support delegation of approval
- [ ] Audit trail for approval decisions

**Approval CLI:**
```bash
orchestrate approval list --pending
orchestrate approval approve <id> --comment "LGTM"
orchestrate approval reject <id> --reason "Needs more testing"
orchestrate approval delegate <id> --to user@example.com
```

### Story 6: Rollback Support

Implement rollback when stages fail.

**Acceptance Criteria:**
- [ ] Define `on_failure: rollback` behavior
- [ ] Track rollback targets per stage
- [ ] Execute rollback agent/task when failure occurs
- [ ] Support manual rollback trigger
- [ ] Record rollback in pipeline run history
- [ ] Prevent rollback loops

### Story 7: Pipeline CLI Commands

Add CLI commands for pipeline management.

**Acceptance Criteria:**
- [ ] `orchestrate pipeline create <file.yaml>` - Create pipeline from YAML
- [ ] `orchestrate pipeline list` - List all pipelines
- [ ] `orchestrate pipeline show <name>` - Show pipeline definition
- [ ] `orchestrate pipeline update <name> <file.yaml>` - Update pipeline
- [ ] `orchestrate pipeline delete <name>` - Delete pipeline
- [ ] `orchestrate pipeline enable/disable <name>` - Toggle pipeline
- [ ] `orchestrate pipeline run <name> [--dry-run]` - Trigger pipeline manually
- [ ] `orchestrate pipeline status <run-id>` - Show run status
- [ ] `orchestrate pipeline cancel <run-id>` - Cancel running pipeline
- [ ] `orchestrate pipeline history <name>` - Show run history

### Story 8: Pipeline REST API

Add REST endpoints for pipelines.

**Acceptance Criteria:**
- [ ] `GET /api/pipelines` - List pipelines
- [ ] `POST /api/pipelines` - Create pipeline
- [ ] `GET /api/pipelines/:name` - Get pipeline
- [ ] `PUT /api/pipelines/:name` - Update pipeline
- [ ] `DELETE /api/pipelines/:name` - Delete pipeline
- [ ] `POST /api/pipelines/:name/run` - Trigger run
- [ ] `GET /api/pipelines/:name/runs` - List runs
- [ ] `GET /api/pipeline-runs/:id` - Get run details
- [ ] `POST /api/pipeline-runs/:id/cancel` - Cancel run
- [ ] `GET /api/approvals` - List pending approvals
- [ ] `POST /api/approvals/:id/approve` - Approve
- [ ] `POST /api/approvals/:id/reject` - Reject

### Story 9: Pipeline Dashboard UI

Add pipeline visualization to web dashboard.

**Acceptance Criteria:**
- [ ] Pipeline list page with status indicators
- [ ] Pipeline detail page with YAML editor
- [ ] Run visualization showing stage DAG
- [ ] Stage status with colors (green/yellow/red)
- [ ] Live updates via WebSocket
- [ ] Approval modal for pending approvals
- [ ] Manual trigger button
- [ ] Run history timeline

### Story 10: Built-in Pipeline Templates

Provide starter pipeline templates.

**Acceptance Criteria:**
- [ ] CI pipeline (lint → test → build)
- [ ] CD pipeline (deploy staging → smoke → deploy prod)
- [ ] Release pipeline (version bump → changelog → release)
- [ ] Security pipeline (scan → report → fix)
- [ ] `orchestrate pipeline init <template>` command

## Definition of Done

- [ ] All stories completed and tested
- [ ] Complex pipeline integration tests
- [ ] Documentation with pipeline examples
- [ ] Performance tested with 20-stage pipelines
- [ ] Approval workflow e2e tested
