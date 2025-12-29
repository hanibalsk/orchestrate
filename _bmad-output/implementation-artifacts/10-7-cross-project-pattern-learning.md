# Story 10.7: Cross-Project Pattern Learning

Status: ready-for-dev

## Story

As a **system administrator**,
I want **to export and import learned patterns between projects**,
so that **effective patterns can be shared and reused**.

## Acceptance Criteria

1. **AC1**: Export learned instructions to portable YAML format
2. **AC2**: Import instructions from other projects with confidence adjustment
3. **AC3**: Pattern matching for similar projects (language, framework detection)
4. **AC4**: Confidence adjustment for imported patterns based on project similarity
5. **AC5**: CLI commands for export/import operations
6. **AC6**: Include effectiveness metrics in exported patterns

## Tasks / Subtasks

- [ ] Task 1: Pattern export functionality (AC: 1, 6)
  - [ ] 1.1: Create PatternExport struct with pattern data and metadata
  - [ ] 1.2: Export instructions with effectiveness metrics
  - [ ] 1.3: Export tool sequences with success rates
  - [ ] 1.4: Generate YAML output format
  - [ ] 1.5: Add `orchestrate learn export --output patterns.yaml`

- [ ] Task 2: Pattern import functionality (AC: 2)
  - [ ] 2.1: Parse YAML pattern files
  - [ ] 2.2: Validate pattern format and data
  - [ ] 2.3: Import as new instructions with adjusted confidence
  - [ ] 2.4: Add `orchestrate learn import --file patterns.yaml`

- [ ] Task 3: Project similarity detection (AC: 3)
  - [ ] 3.1: Detect project language from files (Cargo.toml, package.json, etc.)
  - [ ] 3.2: Detect frameworks and libraries
  - [ ] 3.3: Calculate similarity score between projects

- [ ] Task 4: Confidence adjustment (AC: 4)
  - [ ] 4.1: Reduce confidence based on project dissimilarity
  - [ ] 4.2: Reduce confidence based on pattern age
  - [ ] 4.3: Apply minimum confidence threshold

- [ ] Task 5: CLI commands (AC: 5)
  - [ ] 5.1: Implement `learn export` command
  - [ ] 5.2: Implement `learn import` command
  - [ ] 5.3: Add --filter options for selective export

- [ ] Task 6: Write tests
  - [ ] 6.1: Tests for export format
  - [ ] 6.2: Tests for import validation
  - [ ] 6.3: Tests for similarity detection
  - [ ] 6.4: Tests for confidence adjustment

## Dev Notes

### Pattern Export Format (from Epic)

```yaml
patterns:
  - id: pat-001
    type: instruction
    content: "Always check for null before accessing properties"
    context:
      language: typescript
      agent_types: [story-developer, issue-fixer]
    effectiveness:
      success_rate: 0.87
      sample_size: 234

  - id: pat-002
    type: tool_sequence
    content: ["read_file", "grep", "edit_file", "run_tests"]
    context:
      task_type: bug_fix
    effectiveness:
      success_rate: 0.91
      sample_size: 156
```

### Confidence Adjustment Algorithm

```rust
fn adjust_confidence(base_confidence: f64, project_similarity: f64, pattern_age_days: u32) -> f64 {
    let similarity_factor = 0.5 + (0.5 * project_similarity);  // 0.5-1.0
    let age_factor = 1.0 - (pattern_age_days as f64 / 365.0).min(0.5);  // 0.5-1.0
    (base_confidence * similarity_factor * age_factor).max(0.3)  // Min 0.3
}
```

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 7]
- [Source: crates/orchestrate-core/src/instruction.rs] - InstructionSource::Imported

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
