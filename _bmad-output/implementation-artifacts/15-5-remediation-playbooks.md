# Story 15.5: Remediation Playbooks

Status: ready-for-dev

## Story

As a **SRE**, I want **to define and execute remediation playbooks**, so that **common issues are fixed automatically**.

## Acceptance Criteria

1. **AC1**: Create `playbooks` table with id, name, triggers, actions
2. **AC2**: Define playbooks in YAML
3. **AC3**: Execute playbook actions automatically
4. **AC4**: Support manual approval for dangerous actions
5. **AC5**: Track playbook execution history
6. **AC6**: `orchestrate playbook create/list/run`

## Tasks / Subtasks

- [ ] Task 1: Playbook model
- [ ] Task 2: YAML parser
- [ ] Task 3: Action execution
- [ ] Task 4: Approval flow
- [ ] Task 5: History tracking
- [ ] Task 6: CLI commands
- [ ] Task 7: Write tests

## Dev Notes

### Playbook Example
```yaml
playbook:
  name: db-connection-exhaustion
  triggers:
    - condition: "db_connection_errors > 10/min"
  actions:
    - name: Increase connection pool
      command: "kubectl set env deployment/app DB_POOL_SIZE=50"
      requires_approval: false
```

### References
- [Source: docs/bmad/epics/epic-015-incident-response.md#Story 5]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
