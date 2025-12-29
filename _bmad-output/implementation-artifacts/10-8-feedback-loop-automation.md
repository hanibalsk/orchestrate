# Story 10.8: Feedback Loop Automation

Status: ready-for-dev

## Story

As a **system administrator**,
I want **the learning system to run automated daily analysis and improvements**,
so that **the system continuously improves without manual intervention**.

## Acceptance Criteria

1. **AC1**: Scheduled daily learning analysis runs automatically
2. **AC2**: Auto-generate instruction suggestions based on patterns
3. **AC3**: Auto-disable ineffective instructions (below threshold)
4. **AC4**: Auto-promote successful A/B test experiments
5. **AC5**: Generate daily learning summary reports
6. **AC6**: CLI command `orchestrate learn auto --enable` to toggle automation

## Tasks / Subtasks

- [ ] Task 1: Scheduled analysis (AC: 1)
  - [ ] 1.1: Create LearningScheduler component
  - [ ] 1.2: Implement daily analysis trigger (cron-like)
  - [ ] 1.3: Analyze previous day's agent outcomes
  - [ ] 1.4: Store analysis results

- [ ] Task 2: Auto-generate suggestions (AC: 2)
  - [ ] 2.1: Process observed patterns meeting threshold
  - [ ] 2.2: Generate instruction suggestions from patterns
  - [ ] 2.3: Store suggestions for review or auto-apply

- [ ] Task 3: Auto-disable ineffective (AC: 3)
  - [ ] 3.1: Query instructions with penalty > threshold
  - [ ] 3.2: Auto-disable with logged reason
  - [ ] 3.3: Notify via learning report

- [ ] Task 4: Auto-promote experiments (AC: 4)
  - [ ] 4.1: Check experiments for statistical significance
  - [ ] 4.2: Promote winning variants automatically
  - [ ] 4.3: Close completed experiments

- [ ] Task 5: Learning reports (AC: 5)
  - [ ] 5.1: Create DailyLearningReport struct
  - [ ] 5.2: Include: patterns found, suggestions made, disabled instructions, experiment results
  - [ ] 5.3: Store reports in database
  - [ ] 5.4: Add CLI command to view reports

- [ ] Task 6: Automation toggle (AC: 6)
  - [ ] 6.1: Add `learn auto --enable/--disable` CLI command
  - [ ] 6.2: Store automation preference in config
  - [ ] 6.3: Respect automation setting in scheduler

- [ ] Task 7: Write tests
  - [ ] 7.1: Tests for scheduler logic
  - [ ] 7.2: Tests for auto-disable
  - [ ] 7.3: Tests for report generation

## Dev Notes

### Daily Learning Cycle (from Epic)

1. Analyze previous day's outcomes
2. Update effectiveness scores
3. Identify new patterns
4. Suggest new instructions
5. Review experiment results
6. Generate learning report

### Database Schema

```sql
CREATE TABLE learning_reports (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    report_date TEXT NOT NULL UNIQUE,
    patterns_found INTEGER DEFAULT 0,
    suggestions_made INTEGER DEFAULT 0,
    instructions_disabled INTEGER DEFAULT 0,
    experiments_promoted INTEGER DEFAULT 0,
    report_data TEXT NOT NULL, -- JSON
    created_at TEXT NOT NULL
);

CREATE TABLE automation_config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
```

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 8]
- Depends on: Stories 10.3, 10.4

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
