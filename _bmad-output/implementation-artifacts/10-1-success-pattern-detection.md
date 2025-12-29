# Story 10.1: Success Pattern Detection

Status: ready-for-dev

## Story

As a **system administrator**,
I want **the learning system to analyze successful agent completions and identify common success patterns**,
so that **the system can apply these patterns to improve future agent performance**.

## Acceptance Criteria

1. **AC1**: The system analyzes successful task completions and extracts patterns from tool usage, prompt structure, and context
2. **AC2**: Success patterns are identified including effective tool sequences, optimal context sizes, and prompt structures
3. **AC3**: Patterns are stored in the database with metadata (agent type, task type, success metrics)
4. **AC4**: A `orchestrate learn successes` CLI command triggers success pattern analysis
5. **AC5**: Success patterns include: prompt structure, tool call sequences, context size, model choice, timing information
6. **AC6**: The system tracks and stores at least 5 success factors as defined in the epic

## Tasks / Subtasks

- [ ] Task 1: Extend LearningEngine to analyze successful runs (AC: 1)
  - [ ] 1.1: Add `analyze_successful_run` method to LearningEngine
  - [ ] 1.2: Create SuccessPattern struct with fields for tool sequence, prompt hash, context size, model, duration
  - [ ] 1.3: Extract tool call sequences from successful message history
  - [ ] 1.4: Calculate and store context size metrics

- [ ] Task 2: Create success pattern data model (AC: 3, 5)
  - [ ] 2.1: Add SuccessPatternType enum (ToolSequence, PromptStructure, ContextSize, ModelChoice, TimingPattern)
  - [ ] 2.2: Create SuccessPattern struct in instruction.rs or new success_pattern.rs
  - [ ] 2.3: Add database migration for success_patterns table
  - [ ] 2.4: Implement Database methods: insert_success_pattern, get_success_patterns, get_patterns_by_type

- [ ] Task 3: Implement success factor tracking (AC: 5, 6)
  - [ ] 3.1: Track prompt structure patterns (length, sections, format)
  - [ ] 3.2: Track tool call sequence efficiency (order, count, success rate)
  - [ ] 3.3: Track optimal context sizes per task type
  - [ ] 3.4: Track model choice effectiveness per task type
  - [ ] 3.5: Track time-of-day correlation with success (API performance patterns)

- [ ] Task 4: Add CLI command for success analysis (AC: 4)
  - [ ] 4.1: Add `learn successes` subcommand to CLI
  - [ ] 4.2: Implement analysis runner that processes recent successful agents
  - [ ] 4.3: Add output formatting showing discovered patterns
  - [ ] 4.4: Add --since and --agent-type filter options

- [ ] Task 5: Write comprehensive tests (AC: 1-6)
  - [ ] 5.1: Unit tests for success pattern extraction
  - [ ] 5.2: Unit tests for pattern storage and retrieval
  - [ ] 5.3: Integration tests for CLI command
  - [ ] 5.4: Test pattern deduplication logic

## Dev Notes

### Architecture Context

The project already has a robust learning infrastructure:

- **LearningEngine** (`crates/orchestrate-core/src/learning.rs`): Currently only analyzes failures. This story extends it to analyze successes.
- **Instruction types** (`crates/orchestrate-core/src/instruction.rs`): Contains PatternType, LearningPattern, LearningConfig. Extend with success-specific types.
- **Database layer** (`crates/orchestrate-core/src/database.rs`): Has pattern storage methods. Add success pattern methods.
- **CLI** (`crates/orchestrate-cli/`): Add new subcommand under `learn`.

### Key Files to Modify

1. `crates/orchestrate-core/src/learning.rs` - Add success analysis methods
2. `crates/orchestrate-core/src/instruction.rs` - Add SuccessPatternType, SuccessPattern structs
3. `crates/orchestrate-core/src/database.rs` - Add success pattern CRUD methods
4. `crates/orchestrate-cli/src/main.rs` or commands module - Add `learn successes` command
5. `migrations/` - Add new migration for success_patterns table

### Existing Patterns to Follow

From `learning.rs`:
```rust
// Pattern analysis uses signature hashing for deduplication
fn create_signature(&self, content: &str, prefix: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    format!("{}_{}", prefix, hex::encode(&hash[..8]))
}

// Patterns are stored via Database methods
db.upsert_learning_pattern(pattern).await?;
```

From `instruction.rs`:
```rust
// PatternType enum pattern - extend similarly for SuccessPatternType
pub enum PatternType {
    ErrorPattern,
    ToolUsagePattern,
    BehaviorPattern,
}
```

### Database Schema for Success Patterns

```sql
CREATE TABLE success_patterns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    pattern_type TEXT NOT NULL,  -- 'tool_sequence', 'prompt_structure', 'context_size', 'model_choice', 'timing'
    agent_type TEXT,
    task_type TEXT,
    pattern_signature TEXT NOT NULL UNIQUE,
    pattern_data TEXT NOT NULL,  -- JSON blob
    occurrence_count INTEGER DEFAULT 1,
    avg_completion_time_ms INTEGER,
    avg_token_usage INTEGER,
    success_rate REAL DEFAULT 1.0,
    first_seen_at TEXT NOT NULL,
    last_seen_at TEXT NOT NULL
);

CREATE INDEX idx_success_patterns_type ON success_patterns(pattern_type);
CREATE INDEX idx_success_patterns_agent ON success_patterns(agent_type);
```

### Success Factors to Track (from Epic)

1. **Prompt structure that works** - Hash and categorize effective prompt patterns
2. **Tool call sequences that are efficient** - Track order and combinations
3. **Context size that's optimal** - Correlate context token count with success
4. **Model choice for task type** - Track which model works best for what
5. **Time of day (API performance)** - Note timing correlations

### Testing Strategy

- Use existing test patterns from `learning.rs` tests
- Mock database interactions for unit tests
- Create test fixtures with sample successful agent runs
- Test CLI with integration tests using temp database

### Project Structure Notes

- Rust workspace with multiple crates
- SQLite database with sqlx
- Tokio async runtime
- Tracing for logging
- Clap for CLI

### References

- [Source: crates/orchestrate-core/src/learning.rs] - Existing learning engine
- [Source: crates/orchestrate-core/src/instruction.rs] - Pattern type definitions
- [Source: docs/bmad/epics/epic-010-closed-loop-learning.md#Story 1] - Original requirements

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

(To be filled during implementation)

### Completion Notes List

(To be filled during implementation)

### File List

(To be filled during implementation - list all new/modified/deleted files)
