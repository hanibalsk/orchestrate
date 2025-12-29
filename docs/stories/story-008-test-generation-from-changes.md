# Story 8: Test Generation from Code Changes

## Overview

Automatically analyze git diffs to identify changed functions and suggest tests for untested code. This feature integrates with the pr-shepherd workflow to help maintain test coverage for all code changes.

## Implementation

### Components

1. **ChangeTestAnalyzer** (`orchestrate-core/src/change_test_analyzer.rs`)
   - Parses git diffs to extract changed functions
   - Checks test coverage for each changed function
   - Generates test suggestions for untested code
   - Formats results as PR comments

2. **CLI Integration** (`orchestrate-cli`)
   - `orchestrate test analyze-pr` command
   - Supports text, JSON, and markdown output
   - Can post suggestions as PR comments via GitHub CLI

3. **Multi-Language Support**
   - Rust (`.rs`)
   - TypeScript/JavaScript (`.ts`, `.tsx`, `.js`, `.jsx`)
   - Python (`.py`)

### Key Features

#### 1. Diff Analysis

Analyzes git diffs to identify:
- New functions added
- Modified existing functions
- Public vs. private function visibility
- Function signatures and locations

```rust
let analyzer = ChangeTestAnalyzer::new(repo_path);
let result = analyzer.analyze_diff(&diff_content, "main", "feature-branch").await?;
```

#### 2. Test Coverage Detection

Automatically finds and checks test files:
- **Rust**: Inline `#[test]` modules and `tests/*.rs`
- **TypeScript**: `.test.ts`, `.spec.ts`, `__tests__/`
- **Python**: `test_*.py`, `*_test.py`, `tests/`

#### 3. Priority-Based Suggestions

Assigns priority levels to test suggestions:
- **High Priority**: Public functions (API surface)
- **Medium Priority**: Private/internal functions
- **Low Priority**: Helper functions and test utilities

#### 4. Test Case Generation

Suggests meaningful test cases for each function:
- Happy path tests
- Edge case tests
- Error handling tests (for public functions)

#### 5. PR Comment Formatting

Generates markdown comments suitable for GitHub PRs:

```markdown
## Test Coverage Analysis

**Coverage:** 66.7% of changed functions have tests

### Missing Tests

#### High Priority

**`calculate_sum`** in `src/calculator.rs`
- New public function 'calculate_sum' added without tests
- Suggested tests:
  - `test_calculate_sum_happy_path`
  - `test_calculate_sum_edge_cases`
  - `test_calculate_sum_error_handling`
```

## Usage

### CLI Command

```bash
# Analyze current branch against main
orchestrate test analyze-pr --base main

# Analyze specific PR
orchestrate test analyze-pr --pr 123 --base main --head feature-branch

# Post results as PR comment
orchestrate test analyze-pr --pr 123 --comment

# Output as JSON
orchestrate test analyze-pr --output json

# Output as markdown
orchestrate test analyze-pr --output markdown
```

### Programmatic Usage

```rust
use orchestrate_core::ChangeTestAnalyzer;
use std::path::PathBuf;

let analyzer = ChangeTestAnalyzer::new(PathBuf::from("."));

// Get git diff
let diff = get_git_diff("main", "feature-branch");

// Analyze changes
let result = analyzer.analyze_diff(&diff, "main", "feature-branch").await?;

// Check results
println!("Coverage: {:.1}%", result.coverage_percentage);
println!("Suggestions: {}", result.suggestions.len());

// Format as PR comment
let comment = analyzer.format_pr_comment(&result);
```

## Integration with PR Shepherd

This feature can be integrated into the pr-shepherd workflow to automatically:

1. **On PR Creation**: Analyze changes and post test coverage suggestions
2. **On Push**: Re-analyze and update suggestions
3. **Track Progress**: Monitor test additions over time

### Example Webhook Integration

```rust
// In webhook handler for pull_request.opened
async fn handle_pr_opened(pr_number: i32) -> Result<()> {
    let analyzer = ChangeTestAnalyzer::new(repo_path);

    // Get diff from GitHub API
    let diff = get_pr_diff(pr_number).await?;

    // Analyze
    let result = analyzer.analyze_diff(&diff, "main", &pr_branch).await?;

    // Post comment if there are suggestions
    if !result.suggestions.is_empty() {
        let comment = analyzer.format_pr_comment(&result);
        post_pr_comment(pr_number, &comment).await?;
    }

    Ok(())
}
```

## Test Coverage

### Unit Tests (16 tests)

Located in `orchestrate-core/src/change_test_analyzer.rs`:

- `test_extract_file_path` - Parse file paths from diff headers
- `test_extract_line_number` - Parse line numbers from hunks
- `test_extract_rust_function` - Extract Rust function signatures
- `test_extract_rust_private_function` - Detect public/private visibility
- `test_extract_typescript_function` - Extract TypeScript functions
- `test_extract_typescript_arrow_function` - Handle arrow functions
- `test_extract_python_function` - Extract Python functions
- `test_extract_python_private_function` - Detect Python visibility
- `test_calculate_priority_public_function` - Priority calculation
- `test_calculate_priority_private_function` - Priority calculation
- `test_calculate_coverage_percentage` - Coverage percentage calculation
- `test_generate_test_cases_rust` - Test case generation
- `test_extract_ts_test_name` - Parse TypeScript test names
- `test_parse_diff_rust` - Full diff parsing
- `test_format_pr_comment_with_suggestions` - Comment formatting
- `test_format_pr_comment_all_covered` - Comment for full coverage

### Integration Tests (5 tests)

Located in `orchestrate-core/tests/change_analyzer_integration_test.rs`:

- `test_analyze_rust_diff_with_new_function` - End-to-end Rust analysis
- `test_analyze_typescript_diff` - End-to-end TypeScript analysis
- `test_generate_suggestions_for_untested_functions` - Suggestion generation
- `test_format_pr_comment` - PR comment formatting
- `test_coverage_percentage_in_result` - Empty diff handling

## Example Output

### Text Format

```
═══════════════════════════════════════════════════════════════
  Test Coverage Analysis Results
═══════════════════════════════════════════════════════════════

  Coverage: 50.0% of changed functions have tests
  Changed functions: 4
  Functions with tests: 2
  Functions without tests: 2

🔴 High Priority (1 functions)
───────────────────────────────────────────────────────────────
  📝 process_payment (src/payment.rs:42)
     New public function 'process_payment' added without tests
     Suggested tests:
       • test_process_payment_happy_path
       • test_process_payment_edge_cases
       • test_process_payment_error_handling

🟡 Medium Priority (1 functions)
───────────────────────────────────────────────────────────────
  📝 validate_amount (src/payment.rs:67)
     New function 'validate_amount' added without tests
     Suggested tests:
       • test_validate_amount_happy_path
       • test_validate_amount_edge_cases
```

### JSON Format

```json
{
  "changed_functions": [
    {
      "name": "process_payment",
      "file_path": "src/payment.rs",
      "line_number": 42,
      "change_type": "added",
      "signature": "pub fn process_payment(amount: f64) -> Result<()>",
      "is_public": true
    }
  ],
  "coverage": [
    {
      "function": { /* ... */ },
      "has_tests": false,
      "test_files": [],
      "test_names": []
    }
  ],
  "suggestions": [
    {
      "function": { /* ... */ },
      "suggested_tests": [
        "test_process_payment_happy_path",
        "test_process_payment_edge_cases",
        "test_process_payment_error_handling"
      ],
      "priority": "high",
      "reason": "New public function 'process_payment' added without tests"
    }
  ],
  "coverage_percentage": 50.0
}
```

## Acceptance Criteria Status

- [x] Analyze git diff for changed functions
- [x] Identify functions lacking tests
- [x] Generate test suggestions
- [x] Post test suggestions as PR comment
- [x] Integrate with pr-shepherd workflow (via CLI and programmatic API)
- [x] Track test additions over time (via coverage percentage in results)

## Future Enhancements

1. **Database Storage**: Store analysis results for historical tracking
2. **Automatic Test Generation**: Generate actual test code, not just suggestions
3. **Coverage Metrics Integration**: Combine with actual coverage data
4. **Smart Prioritization**: Use ML to prioritize based on code complexity
5. **Test Template Library**: Provide language-specific test templates
6. **CI/CD Integration**: Fail builds if coverage drops below threshold

## Related Files

- `crates/orchestrate-core/src/change_test_analyzer.rs` - Main implementation
- `crates/orchestrate-core/src/lib.rs` - Module exports
- `crates/orchestrate-cli/src/main.rs` - CLI integration
- `crates/orchestrate-core/tests/change_analyzer_integration_test.rs` - Integration tests
- `docs/bmad/epics/epic-005-test-generation.md` - Epic documentation
