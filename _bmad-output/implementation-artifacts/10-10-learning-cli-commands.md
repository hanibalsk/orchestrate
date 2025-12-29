# Story 10.10: Learning CLI Commands

Status: ready-for-dev

## Story

As a **system administrator**,
I want **comprehensive CLI commands for all learning operations**,
so that **I can manage the learning system from the command line**.

## Acceptance Criteria

1. **AC1**: `orchestrate learn analyze` - Run learning analysis manually
2. **AC2**: `orchestrate learn successes` - Analyze successful patterns (from 10.1)
3. **AC3**: `orchestrate learn effectiveness` - Show instruction effectiveness scores
4. **AC4**: `orchestrate learn suggest` - Get improvement suggestions
5. **AC5**: `orchestrate learn export` / `import` - Pattern sharing (from 10.7)
6. **AC6**: `orchestrate learn auto --enable/--disable` - Toggle automation (from 10.8)
7. **AC7**: `orchestrate experiment create/list/results/promote` - A/B testing commands
8. **AC8**: `orchestrate feedback add/list/stats` - Feedback commands (from 10.2)

## Tasks / Subtasks

- [ ] Task 1: Core learn subcommands (AC: 1, 2, 3, 4)
  - [ ] 1.1: Restructure CLI to have `learn` subcommand group
  - [ ] 1.2: Implement `learn analyze` - trigger pattern analysis
  - [ ] 1.3: Implement `learn successes` - success pattern analysis
  - [ ] 1.4: Implement `learn effectiveness` - show effectiveness table
  - [ ] 1.5: Implement `learn suggest` - show suggestions

- [ ] Task 2: Export/Import commands (AC: 5)
  - [ ] 2.1: Implement `learn export --output <file>`
  - [ ] 2.2: Implement `learn import --file <file>`
  - [ ] 2.3: Add filters: --agent-type, --min-confidence

- [ ] Task 3: Automation commands (AC: 6)
  - [ ] 3.1: Implement `learn auto --enable`
  - [ ] 3.2: Implement `learn auto --disable`
  - [ ] 3.3: Implement `learn auto --status`

- [ ] Task 4: Experiment commands (AC: 7)
  - [ ] 4.1: Implement `experiment create --name <n> --variants <file>`
  - [ ] 4.2: Implement `experiment list`
  - [ ] 4.3: Implement `experiment results --name <n>`
  - [ ] 4.4: Implement `experiment promote --name <n> --variant <v>`

- [ ] Task 5: Feedback commands (AC: 8)
  - [ ] 5.1: Implement `feedback add --agent <id> --rating <r>`
  - [ ] 5.2: Implement `feedback list --agent <id>`
  - [ ] 5.3: Implement `feedback stats`

- [ ] Task 6: Write tests
  - [ ] 6.1: Tests for each subcommand
  - [ ] 6.2: Tests for output formatting
  - [ ] 6.3: Integration tests

## Dev Notes

### CLI Structure

```
orchestrate learn
├── analyze       # Run pattern analysis
├── successes     # Analyze success patterns
├── effectiveness # Show effectiveness scores
├── suggest       # Get improvement suggestions
├── export        # Export patterns
├── import        # Import patterns
└── auto          # Automation toggle

orchestrate experiment
├── create        # Create new experiment
├── list          # List experiments
├── results       # Show experiment results
└── promote       # Promote winning variant

orchestrate feedback
├── add           # Add feedback
├── list          # List feedback
└── stats         # Show statistics
```

### Output Formatting

Use consistent table formatting for list commands:
```
Name             | Effectiveness | Usage  | Status
-----------------|---------------|--------|--------
learned_error_01 | 85.2%         | 234    | enabled
learned_tool_03  | 42.1%         | 89     | disabled
```

### References

- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 10]
- [Source: crates/orchestrate-cli/src/main.rs] - Existing CLI structure

## Dev Agent Record

### Agent Model Used
### Debug Log References
### Completion Notes List
### File List
