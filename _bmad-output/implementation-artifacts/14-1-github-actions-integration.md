# Story 14.1: GitHub Actions Integration

Status: ready-for-dev

## Story

As a **developer**, I want **to integrate with GitHub Actions workflows**, so that **CI/CD is automated**.

## Acceptance Criteria

1. **AC1**: Trigger workflow runs via GitHub API
2. **AC2**: Monitor workflow status
3. **AC3**: Parse workflow results
4. **AC4**: Download workflow artifacts
5. **AC5**: Cancel running workflows
6. **AC6**: `orchestrate ci trigger --workflow <name>`
7. **AC7**: `orchestrate ci status --run <id>`

## Tasks / Subtasks

- [ ] Task 1: GitHub Actions API integration
- [ ] Task 2: Workflow triggering
- [ ] Task 3: Status monitoring
- [ ] Task 4: Result parsing
- [ ] Task 5: Artifact downloading
- [ ] Task 6: CLI commands
- [ ] Task 7: Write tests

## Dev Notes

### References
- [Source: docs/bmad/epics/epic-014-ci-integration.md#Story 1]
- [Source: crates/orchestrate-github/src/lib.rs] - Existing GitHub integration

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
