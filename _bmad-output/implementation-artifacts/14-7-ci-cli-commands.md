# Story 14.7: CI CLI Commands

Status: ready-for-dev

## Story

As a **developer**, I want **comprehensive CLI commands for CI operations**, so that **CI can be managed from command line**.

## Acceptance Criteria

1. **AC1**: `orchestrate ci trigger --workflow <name> [--branch <branch>]`
2. **AC2**: `orchestrate ci status --run <id>`
3. **AC3**: `orchestrate ci wait --run <id>` - Block until complete
4. **AC4**: `orchestrate ci logs --run <id> --job <job>`
5. **AC5**: `orchestrate ci retry --run <id> [--job <job>]`
6. **AC6**: `orchestrate ci cancel --run <id>`
7. **AC7**: `orchestrate ci artifacts --run <id> --output <dir>`

## Tasks / Subtasks

- [ ] Task 1: Implement all ci subcommands
- [ ] Task 2: Write tests

## Dev Notes

### References
- [Source: docs/bmad/epics/epic-014-ci-integration.md#Story 7]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
