# Story 11.4: Changelog Automation

Status: ready-for-dev

## Story

As a **developer**, I want **to generate changelogs from commits and PRs**, so that **release notes are automatically created**.

## Acceptance Criteria

1. **AC1**: Parse conventional commits
2. **AC2**: Group by type (feat, fix, etc.)
3. **AC3**: Link to PRs and issues
4. **AC4**: Support Keep a Changelog format
5. **AC5**: CLI command `orchestrate docs changelog --from <tag> --to <tag>`

## Tasks / Subtasks

- [ ] Task 1: Commit parsing (AC: 1, 2)
- [ ] Task 2: PR/Issue linking (AC: 3)
- [ ] Task 3: Changelog formatting (AC: 4)
- [ ] Task 4: CLI command (AC: 5)
- [ ] Task 5: Write tests

## Dev Notes

### Output Format
```markdown
## [1.2.0] - 2024-01-15

### Added
- GitHub webhook triggers (#45)

### Fixed
- Agent timeout handling (#47)
```

### References
- [Source: docs/bmad/epics/epic-011-documentation-generator.md#Story 4]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
