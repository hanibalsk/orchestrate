# Story 10.3: Effectiveness Scoring

Status: ready-for-dev

## Story

As a **system administrator**,
I want **the system to calculate effectiveness scores for instructions and patterns**,
so that **I can identify and remove ineffective instructions while promoting effective ones**.

## Acceptance Criteria

1. **AC1**: Track instruction usage across all agent runs
2. **AC2**: Correlate instructions with agent outcomes (success/failure)
3. **AC3**: Calculate effectiveness = successes / (successes + failures)
4. **AC4**: Weight scores by recency (recent outcomes matter more)
5. **AC5**: Automatically identify ineffective instructions (effectiveness < threshold)
6. **AC6**: Generate suggestions for instruction improvements

## Tasks / Subtasks

- [ ] Task 1: Enhance instruction tracking (AC: 1, 2)
  - [ ] 1.1: Add instruction_usage tracking to agent runs
  - [ ] 1.2: Store which instructions were active for each run
  - [ ] 1.3: Link agent outcomes to active instructions

- [ ] Task 2: Implement effectiveness calculation (AC: 3, 4)
  - [ ] 2.1: Add time-weighted effectiveness algorithm
  - [ ] 2.2: Implement recency decay (exponential decay for older data)
  - [ ] 2.3: Add user feedback score integration
  - [ ] 2.4: Update InstructionEffectiveness struct with new fields

- [ ] Task 3: Auto-identify ineffective instructions (AC: 5)
  - [ ] 3.1: Add configurable effectiveness threshold
  - [ ] 3.2: Create query for low-effectiveness instructions
  - [ ] 3.3: Add CLI command to list ineffective instructions

- [ ] Task 4: Generate improvement suggestions (AC: 6)
  - [ ] 4.1: Analyze patterns in instruction failures
  - [ ] 4.2: Generate natural language suggestions
  - [ ] 4.3: Add CLI command for suggestions

- [ ] Task 5: Write tests
  - [ ] 5.1: Unit tests for effectiveness calculation
  - [ ] 5.2: Tests for recency weighting
  - [ ] 5.3: Tests for suggestion generation

## Dev Notes

### Effectiveness Algorithm (from Epic)

```
effectiveness = (
    0.7 * recent_success_rate +  // Last 7 days
    0.2 * historical_success_rate +  // All time
    0.1 * user_feedback_score
)
```

### Key Files

- `crates/orchestrate-core/src/instruction.rs` - InstructionEffectiveness already exists
- `crates/orchestrate-core/src/database.rs` - Add tracking queries
- `crates/orchestrate-core/src/learning.rs` - Integrate with learning engine

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 3]
- [Source: crates/orchestrate-core/src/instruction.rs#InstructionEffectiveness]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
