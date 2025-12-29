# Story 8: Quick Reference Guide

## Test Generation from Code Changes - Cheat Sheet

### CLI Commands

```bash
# Basic analysis
orchestrate test analyze-pr

# Analyze specific PR
orchestrate test analyze-pr --pr 123

# With custom base/head
orchestrate test analyze-pr --base main --head feature-branch

# Post as PR comment
orchestrate test analyze-pr --pr 123 --comment

# Different output formats
orchestrate test analyze-pr --output text      # Human-readable (default)
orchestrate test analyze-pr --output json      # Machine-readable
orchestrate test analyze-pr --output markdown  # For copying to PR
```

### Programmatic API

```rust
use orchestrate_core::ChangeTestAnalyzer;
use std::path::PathBuf;

// Initialize
let analyzer = ChangeTestAnalyzer::new(PathBuf::from("."));

// Analyze diff
let result = analyzer.analyze_diff(
    &diff_content,
    "main",           // base ref
    "feature-branch"  // head ref
).await?;

// Access results
println!("Coverage: {:.1}%", result.coverage_percentage);
println!("Changed: {}", result.changed_functions.len());
println!("Missing tests: {}", result.suggestions.len());

// Get suggestions by priority
use orchestrate_core::Priority;
let high_priority: Vec<_> = result.suggestions
    .iter()
    .filter(|s| s.priority == Priority::High)
    .collect();

// Format as PR comment
let comment = analyzer.format_pr_comment(&result);
```

### Data Structures

```rust
// Main result
struct ChangeAnalysisResult {
    changed_functions: Vec<ChangedFunction>,
    coverage: Vec<TestCoverage>,
    suggestions: Vec<TestSuggestion>,
    coverage_percentage: f64,
}

// Changed function
struct ChangedFunction {
    name: String,
    file_path: PathBuf,
    line_number: usize,
    change_type: ChangeType,    // Added, Modified, Deleted
    signature: String,
    is_public: bool,
}

// Test suggestion
struct TestSuggestion {
    function: ChangedFunction,
    suggested_tests: Vec<String>,
    priority: Priority,         // High, Medium, Low
    reason: String,
}
```

### Language Support

| Language | File Extensions | Test Patterns | Function Detection |
|----------|----------------|---------------|-------------------|
| Rust | `.rs` | `#[test]`, `tests/*.rs` | `pub fn`, `fn`, `async fn` |
| TypeScript | `.ts`, `.tsx` | `*.test.ts`, `__tests__/` | `export function`, `const x =` |
| JavaScript | `.js`, `.jsx` | `*.test.js`, `__tests__/` | `export function`, `const x =` |
| Python | `.py` | `test_*.py`, `tests/` | `def func()`, `def _private()` |

### Priority Levels

| Priority | Criteria | Example |
|----------|----------|---------|
| High | Public functions | `pub fn api_endpoint()` |
| Medium | Private functions | `fn internal_helper()` |
| Low | Test helpers, utilities | `fn test_setup_helper()` |

### Output Formats

#### Text (Console)
```
═══════════════════════════════════════════════════════════════
  Test Coverage Analysis Results
═══════════════════════════════════════════════════════════════
  Coverage: 66.7% of changed functions have tests
  Changed functions: 3
  Functions with tests: 2
  Functions without tests: 1
```

#### JSON (Machine-readable)
```json
{
  "coverage_percentage": 66.7,
  "changed_functions": [...],
  "suggestions": [...]
}
```

#### Markdown (PR Comments)
```markdown
## Test Coverage Analysis
**Coverage:** 66.7% of changed functions have tests

### Missing Tests
#### High Priority
**`process_payment`** in `src/payment.rs`
- New public function added without tests
- Suggested tests:
  - `test_process_payment_happy_path`
  - `test_process_payment_edge_cases`
```

### Integration Examples

#### Webhook Handler
```rust
async fn on_pr_opened(pr: PullRequest) -> Result<()> {
    let diff = fetch_pr_diff(&pr).await?;
    let analyzer = ChangeTestAnalyzer::new(repo_path());
    let result = analyzer.analyze_diff(&diff, "main", &pr.head).await?;

    if result.coverage_percentage < 80.0 {
        let comment = analyzer.format_pr_comment(&result);
        post_comment(pr.number, &comment).await?;
    }

    Ok(())
}
```

#### CI/CD Check
```rust
async fn ci_test_coverage_check() -> Result<()> {
    let diff = get_diff_from_env()?;
    let analyzer = ChangeTestAnalyzer::new(PathBuf::from("."));
    let result = analyzer.analyze_diff(&diff, "main", "HEAD").await?;

    if result.coverage_percentage < 100.0 {
        eprintln!("Error: {} functions lack tests", result.suggestions.len());
        std::process::exit(1);
    }

    Ok(())
}
```

#### Pre-commit Hook
```bash
#!/bin/bash
# .git/hooks/pre-commit

# Get staged changes
DIFF=$(git diff --cached)

# Analyze
RESULT=$(orchestrate test analyze-pr --output json <<< "$DIFF")

# Check coverage
COVERAGE=$(echo "$RESULT" | jq '.coverage_percentage')

if (( $(echo "$COVERAGE < 80" | bc -l) )); then
    echo "⚠️  Test coverage below 80%"
    orchestrate test analyze-pr
    exit 1
fi
```

### Common Patterns

#### Filter by File Type
```rust
let rust_functions: Vec<_> = result.changed_functions
    .iter()
    .filter(|f| f.file_path.extension().unwrap() == "rs")
    .collect();
```

#### Group Suggestions by File
```rust
use std::collections::HashMap;

let by_file: HashMap<&PathBuf, Vec<&TestSuggestion>> =
    result.suggestions
        .iter()
        .fold(HashMap::new(), |mut acc, s| {
            acc.entry(&s.function.file_path).or_default().push(s);
            acc
        });
```

#### Calculate Statistics
```rust
let stats = (
    result.changed_functions.len(),
    result.suggestions.len(),
    result.coverage_percentage,
);

println!("Changed: {}, Missing: {}, Coverage: {:.1}%",
    stats.0, stats.1, stats.2);
```

### Troubleshooting

**No functions detected?**
- Check if file extensions are supported
- Verify diff format is correct
- Ensure functions have proper syntax

**Wrong priority assigned?**
- Check function visibility (pub/private)
- Verify naming conventions (underscores in Python)

**Tests not found?**
- Check test file naming matches language conventions
- Verify test files exist in expected locations
- Ensure test names reference the function

**Comment not posting?**
- Install GitHub CLI: `gh auth login`
- Verify PR number is correct
- Check you have write permissions

### Tips & Best Practices

1. **Run before committing**: Catch missing tests early
2. **Use in CI/CD**: Enforce coverage standards
3. **Review suggestions**: Not all functions need tests
4. **Customize thresholds**: Set team-specific coverage goals
5. **Track trends**: Monitor coverage over time
6. **Educate team**: Share suggestions to improve test quality

### Related Documentation

- Full Documentation: `docs/stories/story-008-test-generation-from-changes.md`
- Implementation Summary: `docs/stories/STORY_8_SUMMARY.md`
- Epic Overview: `docs/bmad/epics/epic-005-test-generation.md`
