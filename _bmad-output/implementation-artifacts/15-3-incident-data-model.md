# Story 15.3: Incident Data Model

Status: ready-for-dev

## Story

As a **SRE**, I want **to track incidents through their lifecycle**, so that **incident management is organized**.

## Acceptance Criteria

1. **AC1**: Create `incidents` table with id, title, severity, status, detected_at, resolved_at
2. **AC2**: Create `incident_timeline` table for events
3. **AC3**: Status: Detected, Investigating, Mitigating, Resolved, PostMortem
4. **AC4**: Severity: Critical, High, Medium, Low
5. **AC5**: Link incidents to agents and actions

## Tasks / Subtasks

- [ ] Task 1: Database schema
- [ ] Task 2: Incident model
- [ ] Task 3: Timeline model
- [ ] Task 4: Status/severity enums
- [ ] Task 5: Write tests

## Dev Notes

### References
- [Source: docs/bmad/epics/epic-015-incident-response.md#Story 3]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
