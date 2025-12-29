# Story 10.4: A/B Testing Framework

Status: ready-for-dev

## Story

As a **system administrator**,
I want **to run A/B tests on prompt variations**,
so that **I can systematically identify the most effective prompts**.

## Acceptance Criteria

1. **AC1**: Create experiments with multiple prompt variants in YAML format
2. **AC2**: Randomly assign agents to variants based on configured weights
3. **AC3**: Track metrics (success rate, time, tokens) per variant
4. **AC4**: Calculate statistical significance (p-value) between variants
5. **AC5**: Auto-promote winning variant when confidence threshold is met
6. **AC6**: CLI command `orchestrate experiment create --name <name> --variants <file>`

## Tasks / Subtasks

- [ ] Task 1: Create Experiment data model (AC: 1)
  - [ ] 1.1: Create Experiment struct with name, hypothesis, metric, variants, status
  - [ ] 1.2: Create ExperimentVariant struct with name, prompt_path, weight
  - [ ] 1.3: Create ExperimentStatus enum (Running, Completed, Paused)
  - [ ] 1.4: Add database migration for experiments table

- [ ] Task 2: Implement variant assignment (AC: 2)
  - [ ] 2.1: Implement weighted random assignment algorithm
  - [ ] 2.2: Store assignment in agent metadata
  - [ ] 2.3: Ensure consistent assignment for same agent context

- [ ] Task 3: Track metrics per variant (AC: 3)
  - [ ] 3.1: Create ExperimentMetrics struct
  - [ ] 3.2: Track success count, failure count, avg time, avg tokens
  - [ ] 3.3: Store metrics in database per variant

- [ ] Task 4: Statistical significance (AC: 4)
  - [ ] 4.1: Implement chi-squared test for success rate comparison
  - [ ] 4.2: Calculate p-value and confidence interval
  - [ ] 4.3: Add significance check against configured confidence level

- [ ] Task 5: Auto-promotion (AC: 5)
  - [ ] 5.1: Check for statistical significance on each update
  - [ ] 5.2: Auto-promote when min_samples reached and significant
  - [ ] 5.3: Update default prompt to winning variant

- [ ] Task 6: CLI commands (AC: 6)
  - [ ] 6.1: Add `experiment create` command
  - [ ] 6.2: Add `experiment list` command
  - [ ] 6.3: Add `experiment results` command
  - [ ] 6.4: Add `experiment promote` command

- [ ] Task 7: Write tests
  - [ ] 7.1: Unit tests for variant assignment
  - [ ] 7.2: Tests for statistical calculations
  - [ ] 7.3: Integration tests for CLI

## Dev Notes

### Experiment YAML Format (from Epic)

```yaml
experiment:
  name: code-review-prompt-v2
  hypothesis: "More specific review criteria improves fix rate"
  metric: review_fix_rate
  variants:
    - name: control
      prompt: prompts/code-reviewer.md
      weight: 50
    - name: treatment
      prompt: prompts/code-reviewer-v2.md
      weight: 50
  min_samples: 100
  confidence_level: 0.95
```

### Database Schema

```sql
CREATE TABLE experiments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    hypothesis TEXT,
    metric TEXT NOT NULL,
    variants TEXT NOT NULL, -- JSON
    min_samples INTEGER DEFAULT 100,
    confidence_level REAL DEFAULT 0.95,
    status TEXT DEFAULT 'running',
    created_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE TABLE experiment_assignments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experiment_id INTEGER NOT NULL,
    agent_id TEXT NOT NULL,
    variant_name TEXT NOT NULL,
    assigned_at TEXT NOT NULL,
    FOREIGN KEY (experiment_id) REFERENCES experiments(id)
);

CREATE TABLE experiment_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    experiment_id INTEGER NOT NULL,
    variant_name TEXT NOT NULL,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    total_time_ms INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    updated_at TEXT NOT NULL,
    FOREIGN KEY (experiment_id) REFERENCES experiments(id)
);
```

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 4]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
