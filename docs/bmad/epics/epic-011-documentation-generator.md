# Epic 011: Documentation Generator Agent

Implement automated documentation generation and maintenance.

**Priority:** Medium
**Effort:** Medium
**Use Cases:** UC-204

## Overview

Add a doc-generator agent that automatically creates and updates documentation from code, commits, and architectural decisions. This ensures documentation stays current with code changes and reduces manual documentation burden.

## Stories

### Story 1: Doc Generator Agent Type

Create new agent type for documentation.

**Acceptance Criteria:**
- [ ] Add `doc-generator` to AgentType enum
- [ ] Create agent prompt in `.claude/agents/doc-generator.md`
- [ ] Agent can analyze code and extract documentation
- [ ] Agent writes clear, concise documentation
- [ ] Agent follows project documentation style

### Story 2: API Documentation Generation

Generate OpenAPI/Swagger documentation from code.

**Acceptance Criteria:**
- [ ] Parse REST API endpoints from code
- [ ] Extract request/response schemas
- [ ] Generate OpenAPI 3.0 YAML
- [ ] Include examples for each endpoint
- [ ] `orchestrate docs generate --type api --output docs/api.yaml`

### Story 3: README Generation

Generate and update README files.

**Acceptance Criteria:**
- [ ] Generate README from project structure
- [ ] Include installation instructions
- [ ] Include usage examples
- [ ] Include configuration options
- [ ] Update existing README sections
- [ ] `orchestrate docs generate --type readme`

### Story 4: Changelog Automation

Generate changelog from commits and PRs.

**Acceptance Criteria:**
- [ ] Parse conventional commits
- [ ] Group by type (feat, fix, etc.)
- [ ] Link to PRs and issues
- [ ] Support Keep a Changelog format
- [ ] `orchestrate docs changelog --from <tag> --to <tag>`

**Output:**
```markdown
## [1.2.0] - 2024-01-15

### Added
- GitHub webhook triggers (#45)
- Scheduled agent execution (#46)

### Fixed
- Agent timeout handling (#47)
- PR queue race condition (#48)

### Changed
- Improved code reviewer prompts (#49)
```

### Story 5: Architecture Decision Records

Generate and manage ADRs.

**Acceptance Criteria:**
- [ ] ADR template with standard sections
- [ ] `orchestrate docs adr create --title <title>`
- [ ] `orchestrate docs adr list`
- [ ] Link ADRs to related code
- [ ] ADR status tracking (proposed, accepted, deprecated)

**ADR Template:**
```markdown
# ADR-001: Use SQLite for Agent State

## Status
Accepted

## Context
We need a database for storing agent state...

## Decision
We will use SQLite with WAL mode...

## Consequences
- Positive: Simple deployment, no external dependencies
- Negative: Single-node limitation
```

### Story 6: Inline Documentation Validation

Validate code documentation coverage.

**Acceptance Criteria:**
- [ ] Check public functions have docstrings
- [ ] Check complex logic has comments
- [ ] Check exported types are documented
- [ ] Report documentation coverage percentage
- [ ] `orchestrate docs validate --check-coverage`

### Story 7: Documentation CLI Commands

CLI commands for documentation.

**Acceptance Criteria:**
- [ ] `orchestrate docs generate --type <api|readme|changelog|adr>`
- [ ] `orchestrate docs validate [--check-coverage]`
- [ ] `orchestrate docs adr create/list/show`
- [ ] `orchestrate docs changelog --from <tag> --to <tag>`
- [ ] `orchestrate docs serve` - Serve docs locally

### Story 8: Documentation Dashboard UI

Add documentation pages to web dashboard.

**Acceptance Criteria:**
- [ ] Documentation overview
- [ ] API documentation viewer
- [ ] ADR browser
- [ ] Changelog viewer
- [ ] Generate documentation button

## Definition of Done

- [ ] All stories completed and tested
- [ ] API docs generation working
- [ ] Changelog automation operational
- [ ] Documentation for orchestrate itself generated
