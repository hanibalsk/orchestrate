# Epic 005: Test Generation Agent

Implement automated test generation and validation capabilities.

**Priority:** Critical
**Effort:** Large
**Use Cases:** UC-203

## Overview

Currently, orchestrate has minimal test coverage (~10%). This epic introduces a test-generator agent that can automatically create unit tests, integration tests, and e2e tests for new and existing code. The agent understands code context and generates meaningful tests with proper assertions.

## Stories

### Story 1: Test Generator Agent Type

Create new agent type for test generation.

**Acceptance Criteria:**
- [ ] Add `test-generator` to AgentType enum
- [ ] Create agent prompt in `.claude/agents/test-generator.md`
- [ ] Agent can analyze code and identify testable units
- [ ] Agent generates tests matching project's test framework
- [ ] Agent follows existing test patterns in codebase
- [ ] Support for Rust (cargo test), TypeScript (vitest/jest), Python (pytest)

**Agent Prompt Focus:**
- Analyze function signatures and behavior
- Identify edge cases and error conditions
- Generate descriptive test names
- Follow existing test conventions
- Include setup/teardown when needed

### Story 2: Unit Test Generation

Generate unit tests for individual functions/methods.

**Acceptance Criteria:**
- [ ] `orchestrate test generate --type unit --target <file>` command
- [ ] Analyze function parameters and return types
- [ ] Generate tests for happy path
- [ ] Generate tests for edge cases (null, empty, boundary values)
- [ ] Generate tests for error conditions
- [ ] Mock external dependencies
- [ ] Place tests in appropriate location (same file, `tests/`, `__tests__/`)

**Output Format:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name_happy_path() {
        // Arrange
        let input = ...;
        // Act
        let result = function_name(input);
        // Assert
        assert_eq!(result, expected);
    }

    #[test]
    fn test_function_name_edge_case_empty() {
        // ...
    }
}
```

### Story 3: Integration Test Generation

Generate integration tests for module interactions.

**Acceptance Criteria:**
- [ ] `orchestrate test generate --type integration --target <module>` command
- [ ] Identify module boundaries and interfaces
- [ ] Generate tests for cross-module interactions
- [ ] Set up test fixtures and databases
- [ ] Clean up after tests
- [ ] Support async test patterns

### Story 4: E2E Test Generation from Stories

Generate end-to-end tests from user story acceptance criteria.

**Acceptance Criteria:**
- [ ] `orchestrate test generate --type e2e --story <story-id>` command
- [ ] Parse acceptance criteria from story
- [ ] Generate Playwright/Cypress tests for web UI
- [ ] Generate API tests for backend endpoints
- [ ] Generate CLI tests for command-line tools
- [ ] Include test data setup

**Example:**
```typescript
// Generated from Story: User can login with valid credentials
test('user can login with valid credentials', async ({ page }) => {
  // Arrange
  await page.goto('/login');

  // Act
  await page.fill('[data-testid="email"]', 'user@example.com');
  await page.fill('[data-testid="password"]', 'validpassword');
  await page.click('[data-testid="login-button"]');

  // Assert
  await expect(page).toHaveURL('/dashboard');
  await expect(page.locator('[data-testid="welcome"]')).toBeVisible();
});
```

### Story 5: Test Coverage Analysis

Track and report test coverage metrics.

**Acceptance Criteria:**
- [ ] `orchestrate test coverage` command
- [ ] Run tests with coverage instrumentation
- [ ] Parse coverage reports (lcov, cobertura)
- [ ] Store coverage metrics in database
- [ ] Track coverage trends over time
- [ ] Identify untested code paths
- [ ] Set coverage thresholds per module

**Output:**
```
Coverage Report:
  orchestrate-core: 45% (target: 80%)
    - src/agent.rs: 62%
    - src/database.rs: 38% ⚠️
    - src/learning.rs: 22% ⚠️
  orchestrate-web: 28% (target: 70%)
  Overall: 38%
```

### Story 6: Test Quality Validation

Validate generated tests are meaningful.

**Acceptance Criteria:**
- [ ] Run mutation testing on generated tests
- [ ] Identify tests with weak assertions
- [ ] Detect tests that always pass
- [ ] Detect tests that test implementation not behavior
- [ ] Suggest test improvements
- [ ] `orchestrate test validate --mutation` command

### Story 7: Property-Based Test Generation

Generate property-based tests for edge case discovery.

**Acceptance Criteria:**
- [ ] Identify functions suitable for property testing
- [ ] Generate property definitions from function contracts
- [ ] Use proptest (Rust), fast-check (TS), hypothesis (Python)
- [ ] Generate shrinking for minimal failure cases
- [ ] `orchestrate test generate --type property --target <fn>`

**Example:**
```rust
proptest! {
    #[test]
    fn parse_then_serialize_roundtrip(input: String) {
        let parsed = parse(&input);
        if let Ok(value) = parsed {
            let serialized = serialize(&value);
            let reparsed = parse(&serialized).unwrap();
            assert_eq!(value, reparsed);
        }
    }
}
```

### Story 8: Test Generation from Code Changes

Automatically suggest tests for changed code in PRs.

**Acceptance Criteria:**
- [ ] Analyze git diff for changed functions
- [ ] Identify functions lacking tests
- [ ] Generate test suggestions
- [ ] Post test suggestions as PR comment
- [ ] Integrate with pr-shepherd workflow
- [ ] Track test additions over time

### Story 9: Test CLI Commands

Comprehensive CLI for test operations.

**Acceptance Criteria:**
- [ ] `orchestrate test generate --type <unit|integration|e2e|property> --target <path>`
- [ ] `orchestrate test coverage [--threshold <percent>]`
- [ ] `orchestrate test coverage --diff` - Coverage for changed files only
- [ ] `orchestrate test validate` - Validate test quality
- [ ] `orchestrate test run` - Run all tests
- [ ] `orchestrate test run --changed` - Run tests for changed code
- [ ] `orchestrate test report` - Generate test report

### Story 10: Test REST API

Add REST endpoints for test operations.

**Acceptance Criteria:**
- [ ] `POST /api/tests/generate` - Generate tests for target
- [ ] `GET /api/tests/coverage` - Get coverage report
- [ ] `GET /api/tests/coverage/history` - Coverage trends
- [ ] `POST /api/tests/run` - Trigger test run
- [ ] `GET /api/tests/runs/:id` - Get test run results
- [ ] `GET /api/tests/suggestions` - Get test suggestions for PR

### Story 11: Test Dashboard UI

Add test metrics to web dashboard.

**Acceptance Criteria:**
- [ ] Coverage overview widget on dashboard
- [ ] Coverage trend chart
- [ ] Module-level coverage breakdown
- [ ] Untested code highlighting
- [ ] Test run history
- [ ] Generate test button for files

## Definition of Done

- [ ] All stories completed and tested
- [ ] Test generator produces valid, runnable tests
- [ ] Coverage tracking operational
- [ ] Integration with BMAD workflow
- [ ] Documentation with examples
- [ ] Self-test: orchestrate's own coverage at 50%+
