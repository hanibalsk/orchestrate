# Story 11.2: API Documentation Generation

Status: ready-for-dev

## Story

As a **developer**, I want **to generate OpenAPI/Swagger documentation from code**, so that **API documentation is accurate and up-to-date**.

## Acceptance Criteria

1. **AC1**: Parse REST API endpoints from Rust code
2. **AC2**: Extract request/response schemas from types
3. **AC3**: Generate valid OpenAPI 3.0 YAML
4. **AC4**: Include examples for each endpoint
5. **AC5**: CLI command `orchestrate docs generate --type api --output docs/api.yaml`

## Tasks / Subtasks

- [ ] Task 1: API endpoint parsing (AC: 1)
  - [ ] 1.1: Parse Axum route definitions
  - [ ] 1.2: Extract HTTP methods and paths
  - [ ] 1.3: Extract handler function signatures

- [ ] Task 2: Schema extraction (AC: 2)
  - [ ] 2.1: Parse struct definitions for request/response
  - [ ] 2.2: Convert Rust types to OpenAPI types
  - [ ] 2.3: Handle nested types and enums

- [ ] Task 3: OpenAPI generation (AC: 3, 4)
  - [ ] 3.1: Generate OpenAPI 3.0 structure
  - [ ] 3.2: Add endpoint documentation
  - [ ] 3.3: Add example values

- [ ] Task 4: CLI command (AC: 5)
  - [ ] 4.1: Add `docs generate --type api` command
  - [ ] 4.2: Output to specified file

- [ ] Task 5: Write tests

## Dev Notes

### References
- [Source: docs/bmad/epics/epic-011-documentation-generator.md#Story 2]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
