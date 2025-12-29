# Story 13.1: Repository Registry

Status: ready-for-dev

## Story

As a **developer**, I want **to manage multiple repository configurations**, so that **I can work across multiple projects**.

## Acceptance Criteria

1. **AC1**: Create `repositories` table with id, name, url, path, config
2. **AC2**: `orchestrate repo add --url <url> --path <local-path>`
3. **AC3**: `orchestrate repo list`
4. **AC4**: `orchestrate repo remove <name>`
5. **AC5**: Support GitHub, GitLab, Bitbucket URLs
6. **AC6**: Clone repository on add

## Tasks / Subtasks

- [ ] Task 1: Create repositories table
- [ ] Task 2: Implement add/list/remove commands
- [ ] Task 3: URL parsing for different providers
- [ ] Task 4: Repository cloning
- [ ] Task 5: Write tests

## Dev Notes

### References
- [Source: docs/bmad/epics/epic-013-multi-repo.md#Story 1]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
