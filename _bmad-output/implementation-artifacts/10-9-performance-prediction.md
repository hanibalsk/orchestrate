# Story 10.9: Performance Prediction

Status: ready-for-dev

## Story

As a **user**,
I want **to predict task outcomes before execution**,
so that **I can make informed decisions about task complexity and resource allocation**.

## Acceptance Criteria

1. **AC1**: Build prediction model from historical agent run data
2. **AC2**: Predict success probability for new tasks
3. **AC3**: Predict token usage range
4. **AC4**: Predict completion time range
5. **AC5**: Warn when task has low success probability
6. **AC6**: CLI command `orchestrate predict --task <description>`

## Tasks / Subtasks

- [ ] Task 1: Historical data model (AC: 1)
  - [ ] 1.1: Create TaskHistory struct with task type, description, outcome, metrics
  - [ ] 1.2: Aggregate historical data by task characteristics
  - [ ] 1.3: Build feature vectors for prediction

- [ ] Task 2: Success probability prediction (AC: 2)
  - [ ] 2.1: Implement similarity-based prediction
  - [ ] 2.2: Weight by task type match
  - [ ] 2.3: Consider agent type performance

- [ ] Task 3: Token usage prediction (AC: 3)
  - [ ] 3.1: Calculate token usage distribution by task type
  - [ ] 3.2: Return min/max range based on percentiles
  - [ ] 3.3: Consider task complexity

- [ ] Task 4: Completion time prediction (AC: 4)
  - [ ] 4.1: Calculate completion time distribution
  - [ ] 4.2: Return time range estimate
  - [ ] 4.3: Factor in model latency

- [ ] Task 5: Low probability warning (AC: 5)
  - [ ] 5.1: Add warning threshold configuration
  - [ ] 5.2: Display risk factors for low-probability tasks
  - [ ] 5.3: Suggest task decomposition for complex tasks

- [ ] Task 6: CLI command (AC: 6)
  - [ ] 6.1: Implement `predict` command
  - [ ] 6.2: Parse task description
  - [ ] 6.3: Format prediction output
  - [ ] 6.4: Include recommendations

- [ ] Task 7: Write tests
  - [ ] 7.1: Tests for prediction accuracy (on test data)
  - [ ] 7.2: Tests for range calculations
  - [ ] 7.3: Tests for CLI output

## Dev Notes

### Prediction Output Format (from Epic)

```
Task Prediction

Description: "Implement OAuth2 login flow"

Predictions:
  Success probability: 78%
  Estimated tokens: 45,000 - 65,000
  Estimated duration: 25-40 minutes
  Recommended model: claude-3-opus

Risk factors:
  - Similar tasks had 22% failure rate
  - Complex integration required

Recommendations:
  - Break into smaller subtasks
  - Ensure OAuth library docs are available
```

### Prediction Algorithm

```rust
struct TaskPrediction {
    success_probability: f64,
    token_range: (u64, u64),  // (min, max)
    duration_range: (Duration, Duration),
    recommended_model: String,
    risk_factors: Vec<String>,
    recommendations: Vec<String>,
}

fn predict(task: &str, history: &TaskHistory) -> TaskPrediction {
    let similar_tasks = history.find_similar(task);
    let success_rate = similar_tasks.iter().map(|t| t.success).mean();
    let token_p10 = similar_tasks.iter().map(|t| t.tokens).percentile(10);
    let token_p90 = similar_tasks.iter().map(|t| t.tokens).percentile(90);
    // ...
}
```

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 9]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
