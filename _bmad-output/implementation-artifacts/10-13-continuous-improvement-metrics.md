# Story 10.13: Continuous Improvement Metrics

Status: ready-for-dev

## Story

As a **system administrator**,
I want **to track overall system improvement over time**,
so that **I can measure the effectiveness of the learning system**.

## Acceptance Criteria

1. **AC1**: Track success rate over time (daily/weekly/monthly)
2. **AC2**: Track cost efficiency over time
3. **AC3**: Track completion time trends
4. **AC4**: Track instruction count and quality metrics
5. **AC5**: Track feedback sentiment (positive/negative ratio)
6. **AC6**: Generate weekly/monthly improvement reports

## Tasks / Subtasks

- [ ] Task 1: Time series data collection (AC: 1, 2, 3)
  - [ ] 1.1: Create daily_metrics table for aggregated stats
  - [ ] 1.2: Implement daily aggregation job
  - [ ] 1.3: Track: success rate, total cost, avg completion time, agent count

- [ ] Task 2: Instruction quality metrics (AC: 4)
  - [ ] 2.1: Track active instruction count over time
  - [ ] 2.2: Track instruction churn (added/removed)
  - [ ] 2.3: Track average instruction effectiveness

- [ ] Task 3: Feedback sentiment tracking (AC: 5)
  - [ ] 3.1: Calculate positive/negative/neutral ratios
  - [ ] 3.2: Track sentiment trend over time
  - [ ] 3.3: Identify feedback patterns

- [ ] Task 4: Improvement reports (AC: 6)
  - [ ] 4.1: Create ImprovementReport struct
  - [ ] 4.2: Implement weekly report generation
  - [ ] 4.3: Implement monthly report generation
  - [ ] 4.4: Compare current period vs previous
  - [ ] 4.5: Identify top improvements and areas needing work

- [ ] Task 5: CLI and API for reports
  - [ ] 5.1: Add `orchestrate report weekly` command
  - [ ] 5.2: Add `orchestrate report monthly` command
  - [ ] 5.3: Add GET /api/reports/:period endpoint

- [ ] Task 6: Write tests
  - [ ] 6.1: Tests for aggregation logic
  - [ ] 6.2: Tests for report generation
  - [ ] 6.3: Tests for trend calculation

## Dev Notes

### Weekly Improvement Report Format (from Epic)

```
Weekly Improvement Report

Success Rate: 78% → 82% (+4%)
Avg Completion Time: 32m → 28m (-12%)
Cost per Task: $0.45 → $0.38 (-15%)
Active Instructions: 45 (+3 new, -1 deprecated)

Top Improvements:
  1. New instruction "validate API responses" +5% success
  2. Prompt v2 for code-reviewer +8% fix rate
  3. Model selection optimization -20% cost

Areas for Improvement:
  1. Complex refactoring tasks (62% success)
  2. Database migration tasks (68% success)
```

### Database Schema

```sql
CREATE TABLE daily_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    date TEXT NOT NULL UNIQUE,
    agent_count INTEGER DEFAULT 0,
    success_count INTEGER DEFAULT 0,
    failure_count INTEGER DEFAULT 0,
    total_tokens INTEGER DEFAULT 0,
    total_cost_cents INTEGER DEFAULT 0,
    avg_completion_time_ms INTEGER,
    active_instructions INTEGER DEFAULT 0,
    positive_feedback INTEGER DEFAULT 0,
    negative_feedback INTEGER DEFAULT 0,
    created_at TEXT NOT NULL
);

CREATE TABLE improvement_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_type TEXT NOT NULL, -- 'weekly' or 'monthly'
    period_start TEXT NOT NULL,
    period_end TEXT NOT NULL,
    report_data TEXT NOT NULL, -- JSON
    created_at TEXT NOT NULL
);
```

### Metrics to Track

| Metric | Description | Calculation |
|--------|-------------|-------------|
| Success Rate | % of agents completing successfully | success_count / (success + failure) |
| Cost Efficiency | Cost per successful task | total_cost / success_count |
| Completion Time | Average time to complete | avg(completion_time) |
| Instruction Churn | New - Removed instructions | count(new) - count(removed) |
| Feedback Sentiment | Positive feedback ratio | positive / (positive + negative) |

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 13]
- [Source: crates/orchestrate-core/src/database.rs] - DailyTokenUsage existing pattern

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
