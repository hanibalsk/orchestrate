# Story 14.6: CI Failure Response

Status: ready-for-dev

## Story

As a **developer**, I want **the system to automatically respond to CI failures**, so that **issues are fixed quickly**.

## Acceptance Criteria

1. **AC1**: Spawn issue-fixer agent on CI failure
2. **AC2**: Provide failure context to agent
3. **AC3**: Auto-retry flaky tests
4. **AC4**: Track flaky test patterns
5. **AC5**: `orchestrate ci fix --run <id>`

## Tasks / Subtasks

- [ ] Task 1: Failure detection trigger
- [ ] Task 2: Agent spawning
- [ ] Task 3: Flaky test detection
- [ ] Task 4: CLI command
- [ ] Task 5: Write tests

## Dev Notes

### References
- [Source: docs/bmad/epics/epic-014-ci-integration.md#Story 6]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
