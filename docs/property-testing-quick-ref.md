# Property-Based Testing Quick Reference

## Supported Properties

| Property | Description | Example Functions | Test Pattern |
|----------|-------------|-------------------|--------------|
| **Roundtrip** | f(g(x)) == x | serialize/deserialize, encode/decode | `assert_eq!(parse(&serialize(&x)), x)` |
| **Idempotency** | f(f(x)) == f(x) | normalize, trim, sort | `assert_eq!(f(&f(&x)), f(&x))` |
| **Commutativity** | f(a,b) == f(b,a) | add, multiply, max | `assert_eq!(f(a,b), f(b,a))` |
| **Inverse** | f(g(x, y), y) == x | add/subtract | `assert_eq!(subtract(add(x,y), y), x)` |

## Detection Keywords

### Roundtrip Pairs
- serialize ↔ deserialize
- encode ↔ decode
- parse ↔ format
- parse ↔ serialize
- to_string ↔ from_string
- to_json ↔ from_json
- compress ↔ decompress
- encrypt ↔ decrypt

### Idempotency
- normalize, sanitize, clean, trim
- sort, unique, dedupe, format

### Commutativity
- add, multiply, max, min
- gcd, lcm, union, intersection

### Inverse
- add ↔ subtract
- increment ↔ decrement
- push ↔ pop
- insert ↔ remove

## Framework Quick Start

### Rust (proptest)
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn property_name(input: Type) {
        // test assertion
    }
}
```

### TypeScript (fast-check)
```typescript
import fc from 'fast-check';

test('property_name', () => {
    fc.assert(
        fc.property(fc.type(), (input) => {
            // test assertion
        })
    );
});
```

### Python (hypothesis)
```python
from hypothesis import given
from hypothesis import strategies as st

@given(st.type())
def test_property_name(input):
    # test assertion
```

## Common Generators

| Type | Rust | TypeScript | Python |
|------|------|------------|--------|
| String | `String` | `fc.string()` | `st.text()` |
| Integer | `i32` | `fc.integer()` | `st.integers()` |
| Float | `f64` | `fc.float()` | `st.floats()` |
| Boolean | `bool` | `fc.boolean()` | `st.booleans()` |
| Array | `Vec<T>` | `fc.array(fc.type())` | `st.lists(st.type())` |

## API Usage

```rust
use orchestrate_core::TestGenerationService;

// Generate for all functions
let result = service.generate_property_tests(file_path, None).await?;

// Generate for specific function
let result = service.generate_property_tests(file_path, Some("serialize")).await?;

// Format output
let code = service.format_property_test_output(&result)?;
```

## Test File Locations

- **Rust**: `{file}_proptest.rs`
- **TypeScript**: `__tests__/{file}.property.test.ts`
- **Python**: `test_property_{file}`

## CLI (Story 9)

```bash
# Generate all property tests
orchestrate test generate --type property --target src/file.rs

# Generate for specific function
orchestrate test generate --type property --target serialize --file src/file.rs
```
