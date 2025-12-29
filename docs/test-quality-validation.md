# Test Quality Validation

Orchestrate can analyze your test files to identify quality issues and suggest improvements.

## Quick Start

```bash
# Validate a test file
orchestrate test validate --file tests/my_test.rs

# Validate and store the report
orchestrate test validate --file tests/my_test.rs --store

# Get JSON output for CI integration
orchestrate test validate --file tests/my_test.rs --output json
```

## What Gets Detected

### 1. Weak Assertions

Tests that don't verify anything:

```rust
#[test]
fn test_something() {
    let result = do_something();
    // Missing assertion!
}
```

**Issue:** No way to know if the test actually validates the code.

**Suggestion:** Add assertions like `assert_eq!`, `assert!`, or `expect()`.

### 2. Always-Passing Tests

Tests with tautological assertions:

```rust
#[test]
fn test_always_passes() {
    assert!(true); // Always passes!
}
```

**Issue:** Provides false confidence - the test can never fail.

**Suggestion:** Replace with meaningful assertions that verify actual behavior.

### 3. Implementation-Focused Tests

Tests that check internal details instead of behavior:

```rust
#[test]
fn test_internal_state() {
    let obj = MyStruct::new();
    assert_eq!(obj.internal_counter, 0); // Checking internals
}
```

**Issue:** Brittle tests that break when refactoring internal implementation.

**Suggestion:** Test public behavior and observable outcomes instead.

## Quality Score

Each test file receives a quality score from 0-100:

- **90-100** ✅ Excellent
- **70-89** ⚠️ Good, but has some issues
- **0-69** ❌ Needs improvement

### Scoring Formula

```
Base Score: 100

Issue Penalties:
- 1-2 issues: -10 points
- 3-5 issues: -20 points
- 6+ issues: -30 points

If mutation testing is performed:
Final Score = (Base Score * 0.5) + (Mutation Score * 0.5)
```

## Command Options

```bash
orchestrate test validate [OPTIONS]

Options:
  -f, --file <FILE>           Test file to validate [required]
      --mutation              Run mutation testing
      --source <FILE>         Source file for mutation testing
      --store                 Store report in database
  -o, --output <FORMAT>       Output format [default: text] [possible: text, json]
```

## Examples

### Basic Validation

```bash
orchestrate test validate --file tests/user_test.rs
```

Output:
```
╔══════════════════════════════════════════════════════════════╗
║              Test Quality Validation Report                  ║
╚══════════════════════════════════════════════════════════════╝

  File: tests/user_test.rs

  Quality Score: 90.0% ✅

  Issues Found: 1

  ❌ Weak Assertions (1):
     • test_user_creation (line 15)
       Test 'test_user_creation' has no assertions
       💡 Add assertions to verify expected behavior
```

### JSON Output for CI

```bash
orchestrate test validate --file tests/api_test.rs --output json | jq
```

```json
{
  "file_path": "tests/api_test.rs",
  "quality_score": 85.0,
  "issues": [
    {
      "file_path": "tests/api_test.rs",
      "test_name": "test_endpoint",
      "issue_type": "weak_assertion",
      "description": "Test has no assertions",
      "suggestion": "Add assertions to verify expected behavior",
      "line_number": 42
    }
  ],
  "mutation_result": null
}
```

### Store Report for Tracking

```bash
orchestrate test validate --file tests/lib_test.rs --store
```

This stores the report in the database for historical tracking and trend analysis.

## Mutation Testing (Coming Soon)

Mutation testing will verify that your tests actually catch bugs by introducing deliberate mutations in your code:

```bash
orchestrate test validate \
  --file tests/calculator_test.rs \
  --mutation \
  --source src/calculator.rs
```

This will:
1. Create mutations in `src/calculator.rs` (e.g., changing `+` to `-`)
2. Run your tests against each mutation
3. Report which mutations survived (tests didn't catch them)
4. Calculate a mutation score

**Mutation Score:** Percentage of mutations caught by your tests.

## Integration with CI/CD

### GitHub Actions

```yaml
- name: Validate Test Quality
  run: |
    orchestrate test validate \
      --file tests/main_test.rs \
      --output json \
      --store > quality-report.json

    # Fail if quality score is below threshold
    score=$(jq '.quality_score' quality-report.json)
    if (( $(echo "$score < 80.0" | bc -l) )); then
      echo "Quality score $score is below threshold 80.0"
      exit 1
    fi
```

### Pre-commit Hook

```bash
#!/bin/bash
# .git/hooks/pre-commit

# Find modified test files
TEST_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep '_test\.rs$')

for file in $TEST_FILES; do
  echo "Validating $file..."

  # Run validation
  orchestrate test validate --file "$file" --output json > /tmp/quality.json

  # Check score
  score=$(jq '.quality_score' /tmp/quality.json)
  if (( $(echo "$score < 70.0" | bc -l) )); then
    echo "❌ Test quality too low ($score): $file"
    echo "Please address the issues:"
    jq -r '.issues[] | "  - \(.test_name): \(.description)"' /tmp/quality.json
    exit 1
  fi
done
```

## Best Practices

### 1. Run Regularly

Validate tests as part of your normal development workflow:
- Before committing changes
- In CI/CD pipelines
- When reviewing PRs

### 2. Set Quality Thresholds

Establish minimum quality scores for your project:
- New code: 90%+
- Existing code: 70%+
- Critical paths: 95%+

### 3. Address Issues Promptly

Don't let quality debt accumulate:
- Fix weak assertions immediately
- Refactor implementation-focused tests
- Use mutation testing to verify fixes

### 4. Track Trends

Store reports over time to:
- Monitor quality improvements
- Identify patterns
- Celebrate progress

## Database Schema

Quality reports are stored in these tables:

- `test_quality_reports` - Overall reports with scores
- `test_quality_issues` - Individual issues found
- `mutation_test_results` - Mutation testing results
- `mutation_details` - Specific mutations that survived

Query historical data:
```sql
SELECT
  file_path,
  quality_score,
  created_at
FROM test_quality_reports
WHERE file_path = 'tests/my_test.rs'
ORDER BY created_at DESC
LIMIT 10;
```

## FAQ

**Q: What languages are supported?**

A: Currently Rust, with patterns that detect TypeScript/JavaScript and Python assertions. Full multi-language support coming soon.

**Q: How does the regex detection work?**

A: The service uses carefully crafted regex patterns to identify test functions and analyze their contents for common anti-patterns.

**Q: Can I customize the detection rules?**

A: Not yet, but this is planned. Custom rules and configurable thresholds are on the roadmap.

**Q: Does this slow down my tests?**

A: No! Validation is separate from test execution. It only analyzes the test code, not runs the tests.

**Q: How accurate is the detection?**

A: Very accurate for common patterns. It may produce false positives for unusual test structures. Report any issues you find!

## Related Commands

- `orchestrate test generate` - Generate tests for your code
- `orchestrate test coverage` - Analyze test coverage
- `orchestrate story` - Manage development stories

## Support

For issues or feature requests, please file an issue in the orchestrate repository.
