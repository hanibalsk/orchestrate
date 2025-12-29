# Story 15.8: Incident CLI Commands

Status: ready-for-dev

## Story

As a **SRE**, I want **comprehensive CLI commands for incident management**, so that **incidents can be managed from command line**.

## Acceptance Criteria

1. **AC1**: `orchestrate incident list [--status <status>]`
2. **AC2**: `orchestrate incident show <id>`
3. **AC3**: `orchestrate incident create --title <title> --severity <sev>`
4. **AC4**: `orchestrate incident investigate <id>`
5. **AC5**: `orchestrate incident mitigate <id> --playbook <name>`
6. **AC6**: `orchestrate incident resolve <id>`
7. **AC7**: `orchestrate incident postmortem <id>`
8. **AC8**: `orchestrate playbook create/list/run`

## Tasks / Subtasks

- [ ] Task 1: Implement all incident subcommands
- [ ] Task 2: Implement playbook subcommands
- [ ] Task 3: Write tests

## Dev Notes

### References
- [Source: docs/bmad/epics/epic-015-incident-response.md#Story 8]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
