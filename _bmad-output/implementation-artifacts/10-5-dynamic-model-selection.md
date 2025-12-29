# Story 10.5: Dynamic Model Selection

Status: ready-for-dev

## Story

As a **system administrator**,
I want **the system to automatically select the optimal model based on task characteristics**,
so that **I can balance cost and quality effectively**.

## Acceptance Criteria

1. **AC1**: Classify tasks by complexity (simple, medium, complex)
2. **AC2**: Track success rate by model per task type
3. **AC3**: Track cost by model per task type
4. **AC4**: Recommend optimal model for new tasks based on patterns
5. **AC5**: Support cost/quality tradeoff preference configuration
6. **AC6**: Auto-select model when spawning agents based on learned patterns

## Tasks / Subtasks

- [ ] Task 1: Task complexity classification (AC: 1)
  - [ ] 1.1: Create TaskComplexity enum (Simple, Medium, Complex)
  - [ ] 1.2: Implement complexity classifier based on task description
  - [ ] 1.3: Use heuristics: word count, code references, file count
  - [ ] 1.4: Allow manual complexity override

- [ ] Task 2: Model performance tracking (AC: 2, 3)
  - [ ] 2.1: Track success rate per model per task type
  - [ ] 2.2: Track token usage and cost per model per task type
  - [ ] 2.3: Store in model_performance table
  - [ ] 2.4: Update on agent completion

- [ ] Task 3: Model recommendation engine (AC: 4)
  - [ ] 3.1: Implement scoring algorithm for model selection
  - [ ] 3.2: Consider success rate, cost, and task complexity
  - [ ] 3.3: Return recommended model with confidence

- [ ] Task 4: Cost/quality configuration (AC: 5)
  - [ ] 4.1: Add model_selection config to config.yaml
  - [ ] 4.2: Support optimization goals: cost, quality, balanced
  - [ ] 4.3: Add max_cost_per_task constraint

- [ ] Task 5: Auto-selection integration (AC: 6)
  - [ ] 5.1: Integrate into agent spawn logic
  - [ ] 5.2: Override manual selection when auto-select enabled
  - [ ] 5.3: Log selection reasoning

- [ ] Task 6: Write tests
  - [ ] 6.1: Tests for complexity classification
  - [ ] 6.2: Tests for model recommendation
  - [ ] 6.3: Tests for configuration parsing

## Dev Notes

### Model Selection Config (from Epic)

```yaml
model_selection:
  rules:
    - task_type: simple_fix
      preferred: claude-3-haiku
      fallback: claude-3-sonnet
    - task_type: complex_refactor
      preferred: claude-3-opus
    - task_type: code_review
      preferred: claude-3-sonnet
  optimization:
    goal: balanced  # cost, quality, balanced
    max_cost_per_task: 0.50
```

### Database Schema

```sql
CREATE TABLE model_performance (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model TEXT NOT NULL,
    task_type TEXT NOT NULL,
    complexity TEXT NOT NULL,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    total_cost_cents INTEGER DEFAULT 0,
    avg_completion_time_ms INTEGER,
    updated_at TEXT NOT NULL,
    UNIQUE(model, task_type, complexity)
);
```

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 5]
- [Source: crates/orchestrate-claude/src/lib.rs] - Claude model handling

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
