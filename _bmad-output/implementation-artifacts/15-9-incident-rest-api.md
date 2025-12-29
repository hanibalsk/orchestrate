# Story 15.9: Incident REST API

Status: ready-for-dev

## Story

As a **developer**, I want **REST endpoints for incident management**, so that **incidents can be managed via API**.

## Acceptance Criteria

1. **AC1**: `GET /api/incidents` - List incidents
2. **AC2**: `GET /api/incidents/:id` - Get details
3. **AC3**: `POST /api/incidents` - Create incident
4. **AC4**: `POST /api/incidents/:id/investigate` - Start investigation
5. **AC5**: `POST /api/incidents/:id/mitigate` - Run playbook
6. **AC6**: `POST /api/incidents/:id/resolve` - Resolve
7. **AC7**: `GET /api/incidents/:id/postmortem` - Get post-mortem
8. **AC8**: `GET /api/playbooks` - List playbooks
9. **AC9**: `POST /api/playbooks/:id/run` - Run playbook

## Tasks / Subtasks

- [ ] Task 1: Implement all REST endpoints
- [ ] Task 2: Write tests

## Dev Notes

### References
- [Source: docs/bmad/epics/epic-015-incident-response.md#Story 9]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
