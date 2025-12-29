# Story 11.1: Doc Generator Agent Type

Status: ready-for-dev

## Story

As a **developer**, I want **a doc-generator agent type that can analyze code and generate documentation**, so that **documentation stays current with code changes**.

## Acceptance Criteria

1. **AC1**: Add `doc-generator` to AgentType enum
2. **AC2**: Create agent prompt in `.claude/agents/doc-generator.md`
3. **AC3**: Agent can analyze code and extract documentation
4. **AC4**: Agent writes clear, concise documentation
5. **AC5**: Agent follows project documentation style

## Tasks / Subtasks

- [ ] Task 1: Add DocGenerator to AgentType enum (AC: 1)
  - [ ] 1.1: Update AgentType enum in agent.rs
  - [ ] 1.2: Add agent type string conversion
  - [ ] 1.3: Update CLI agent type parsing

- [ ] Task 2: Create agent prompt (AC: 2)
  - [ ] 2.1: Create .claude/agents/doc-generator.md prompt file
  - [ ] 2.2: Define documentation analysis capabilities
  - [ ] 2.3: Define output format guidelines

- [ ] Task 3: Implement documentation extraction (AC: 3, 4, 5)
  - [ ] 3.1: Implement code parsing for docstrings
  - [ ] 3.2: Implement type/function signature extraction
  - [ ] 3.3: Generate markdown documentation

- [ ] Task 4: Write tests
  - [ ] 4.1: Unit tests for agent type
  - [ ] 4.2: Tests for documentation extraction

## Dev Notes

### Key Files
- `crates/orchestrate-core/src/agent.rs` - AgentType enum
- `.claude/agents/doc-generator.md` - New prompt file

### References
- [Source: docs/bmad/epics/epic-011-documentation-generator.md#Story 1]

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
