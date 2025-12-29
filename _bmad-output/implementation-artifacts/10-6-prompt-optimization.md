# Story 10.6: Prompt Optimization

Status: ready-for-dev

## Story

As a **system administrator**,
I want **the system to automatically suggest and test prompt improvements**,
so that **prompts continuously improve based on observed outcomes**.

## Acceptance Criteria

1. **AC1**: Track prompt effectiveness metrics (success rate, completion time, token usage)
2. **AC2**: Identify prompt sections that correlate with failures
3. **AC3**: Generate natural language suggestions for prompt improvements
4. **AC4**: Test improvements via the A/B testing framework (Story 10.4)
5. **AC5**: Version prompts with effectiveness history
6. **AC6**: CLI command `orchestrate prompt optimize --agent-type <type>`

## Tasks / Subtasks

- [ ] Task 1: Prompt metrics tracking (AC: 1)
  - [ ] 1.1: Create PromptVersion struct with content hash, version, metrics
  - [ ] 1.2: Track effectiveness per prompt version
  - [ ] 1.3: Store prompt history in database

- [ ] Task 2: Failure correlation analysis (AC: 2)
  - [ ] 2.1: Parse prompts into sections (instructions, examples, constraints)
  - [ ] 2.2: Correlate sections with failure patterns
  - [ ] 2.3: Identify statistically significant correlations

- [ ] Task 3: Suggestion generation (AC: 3)
  - [ ] 3.1: Generate improvement suggestions based on analysis
  - [ ] 3.2: Suggest instruction clarity improvements
  - [ ] 3.3: Suggest example quality improvements
  - [ ] 3.4: Suggest constraint effectiveness improvements

- [ ] Task 4: A/B testing integration (AC: 4)
  - [ ] 4.1: Auto-create experiments for suggested improvements
  - [ ] 4.2: Link optimizations to experiment results
  - [ ] 4.3: Track optimization lineage

- [ ] Task 5: Prompt versioning (AC: 5)
  - [ ] 5.1: Implement prompt version control
  - [ ] 5.2: Store version history with effectiveness
  - [ ] 5.3: Support rollback to previous versions

- [ ] Task 6: CLI command (AC: 6)
  - [ ] 6.1: Add `prompt optimize` command
  - [ ] 6.2: Output suggestions with confidence scores
  - [ ] 6.3: Add --apply flag to create experiment

- [ ] Task 7: Write tests
  - [ ] 7.1: Tests for prompt section parsing
  - [ ] 7.2: Tests for correlation analysis
  - [ ] 7.3: Tests for suggestion generation

## Dev Notes

### Optimization Areas (from Epic)

- Instruction clarity
- Example quality
- Constraint effectiveness
- Context relevance
- Output format specifications

### Database Schema

```sql
CREATE TABLE prompt_versions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_type TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    version INTEGER NOT NULL,
    content TEXT NOT NULL,
    success_rate REAL,
    avg_completion_time_ms INTEGER,
    avg_tokens INTEGER,
    created_at TEXT NOT NULL,
    UNIQUE(agent_type, version)
);

CREATE TABLE prompt_sections (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    prompt_version_id INTEGER NOT NULL,
    section_type TEXT NOT NULL,
    content_hash TEXT NOT NULL,
    failure_correlation REAL,
    FOREIGN KEY (prompt_version_id) REFERENCES prompt_versions(id)
);
```

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 6]
- Depends on: Story 10.4 (A/B Testing Framework)

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
