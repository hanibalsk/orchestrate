# Test Generation Guide

## Quick Start

Generate unit tests for a source file:

```bash
orchestrate test generate --target path/to/file.rs
```

## Command Reference

### Basic Usage

```bash
# Print tests to stdout (default)
orchestrate test generate --target src/lib.rs

# Save tests to file
orchestrate test generate --target src/lib.rs --write

# Custom output location
orchestrate test generate --target src/lib.rs --output tests/custom_test.rs --write
```

### Options

- `--target <PATH>` (required): Source file to analyze
- `--type <TYPE>`: Test type (default: unit)
  - Currently supported: `unit`
  - Future: `integration`, `e2e`, `property`
- `--output <PATH>`: Custom output file path
- `--write`: Write tests to file instead of stdout

## Supported Languages

### Rust (.rs)

**Features:**
- Function parameter and return type detection
- Async function support with `#[tokio::test]`
- Inline test module with `#[cfg(test)]`
- Filters out existing test functions

**Example:**
```rust
// Input: src/math.rs
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

// Generated output (inline in same file):
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_happy_path() {
        // Arrange
        let input = todo!("Setup test input");

        // Act
        let result = add(input);

        // Assert
        assert!(result.is_ok(), "Function should succeed with valid input");
    }
    // ... more tests
}
```

### TypeScript (.ts, .tsx)

**Features:**
- Function and arrow function detection
- Async function support
- Vitest/Jest compatible output
- Placed in `__tests__/` directory

**Example:**
```typescript
// Input: src/utils.ts
function calculateSum(a: number, b: number): number {
    return a + b;
}

// Generated: src/__tests__/utils.test.ts
import { describe, test, expect } from 'vitest';

test('test_calculateSum_happy_path', () => {
  const input = null; // TODO: Setup test input
  const result = calculateSum(input);
  expect(result).toBeDefined();
});
```

### Python (.py)

**Features:**
- Function and async function detection
- pytest compatible output
- Placed in `tests/` directory

**Example:**
```python
# Input: src/calculator.py
def add(a, b):
    return a + b

# Generated: src/tests/calculator_test.py
import pytest

def test_add_happy_path():
    # Arrange
    input_data = None  # TODO: Setup test input

    # Act
    result = add(input_data)

    # Assert
    assert result is not None
```

## Test Categories

Each function gets three test cases:

1. **Happy Path**: Tests with valid inputs
2. **Edge Cases**: Empty inputs, boundary values, null checks
3. **Error Conditions**: Invalid inputs, exception handling

## Best Practices

### 1. Review and Customize Generated Tests

Generated tests are scaffolding. Always:
- Replace `todo!()` markers with actual test data
- Add specific assertions based on business logic
- Include additional edge cases specific to your function

### 2. Run Tests After Generation

```bash
# Generate tests
orchestrate test generate --target src/lib.rs --write

# Run tests (will fail on todo!() markers)
cargo test
```

### 3. Iterate on Test Quality

- Start with happy path tests
- Add edge cases gradually
- Use property-based testing for complex functions (Story 7)

### 4. Use Version Control

```bash
# Generate tests
orchestrate test generate --target src/lib.rs --write

# Review changes
git diff

# Commit if satisfied
git add tests/
git commit -m "Add generated tests for lib.rs"
```

## Examples

### Complete Workflow

```bash
# 1. Write your function
cat > src/calculator.rs << EOF
pub fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err("Division by zero".to_string())
    } else {
        Ok(a / b)
    }
}
EOF

# 2. Generate tests
orchestrate test generate --target src/calculator.rs --write

# 3. Customize tests
# Edit the generated test file to add real test data

# 4. Run tests
cargo test
```

### Async Function Example

```rust
// Input
pub async fn fetch_data(url: String) -> Result<String, Error> {
    reqwest::get(&url).await?.text().await
}

// Generated
#[tokio::test]
async fn test_fetch_data_happy_path() {
    // Arrange
    let input = todo!("Setup test input");

    // Act
    let result = fetch_data(input).await;

    // Assert
    assert!(result.is_ok(), "Function should succeed with valid input");
}
```

## Troubleshooting

### No Functions Found

If no functions are detected:
- Check file extension is supported (.rs, .ts, .py)
- Ensure functions use standard syntax
- Functions starting with `test_` are filtered out

### Type Detection Issues

Current limitations:
- Complex generic types may not be fully parsed
- TypeScript/Python type hints are limited
- Use AST parsers for production (future enhancement)

### Test Framework Compatibility

- Rust: Standard `#[test]` and `#[tokio::test]`
- TypeScript: Vitest and Jest compatible
- Python: pytest compatible

## Future Features

See Epic 005 for upcoming enhancements:
- Integration test generation (Story 3)
- E2E test generation from stories (Story 4)
- Test coverage analysis (Story 5)
- Property-based test generation (Story 7)
- Mutation testing (Story 6)

## API Reference

### TestGenerationService

```rust
use orchestrate_core::TestGenerationService;

let service = TestGenerationService::new();
let result = service.generate_tests(Path::new("src/lib.rs")).await?;

// Access generated data
println!("Functions: {}", result.functions.len());
println!("Test cases: {}", result.test_cases.len());

// Format output
let test_code = service.format_test_output(&result)?;
```

### Language

```rust
use orchestrate_core::Language;

let lang = Language::from_path(Path::new("src/lib.rs"))?;
assert_eq!(lang, Language::Rust);

println!("Test directory: {}", lang.test_directory());
println!("Test extension: {}", lang.test_extension());
```

## Contributing

When adding language support:
1. Add language to `Language` enum
2. Implement function extraction method
3. Implement test code generation
4. Add test file location logic
5. Write comprehensive unit tests

See `crates/orchestrate-core/src/test_generation.rs` for reference.
