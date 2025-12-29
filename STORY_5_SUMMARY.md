# Story 5: CI Status Event Handler - Implementation Summary

## Completed

### Acceptance Criteria Status

- [x] Detect `check_run.completed` or `check_suite.completed` event
  - Implemented in `webhook_processor.rs` to route check_run and check_suite events
  - Handler checks action field to ensure it's "completed"
  - Supports both event types with unified interface

- [x] Check if conclusion is `failure` or `timed_out`
  - Filters events to only process failure/timed_out conclusions
  - Skips success, cancelled, neutral, skipped, and action_required conclusions
  - Logs and skips non-failure events appropriately

- [x] Extract failed check details and logs URL
  - Extracts: check/suite ID, name, conclusion, head SHA, branch
  - Captures details_url and html_url for check_run events
  - Parses pull_requests array to find associated PRs
  - Stores all details in agent context for fixer to use

- [x] Spawn `issue-fixer` agent with CI failure context
  - Creates Agent with type `IssueFixer`
  - Sets context with PR number, branch name, and CI details
  - Persists agent to database
  - Links to pr-shepherd agent if one exists for the PR

- [x] Avoid spawning duplicate fixers for same failure
  - Implements `find_duplicate_ci_fixer` function
  - Checks for existing fixers with same check/suite ID, commit SHA, and PR
  - Different commit = different failure = new fixer (allows retry after fix)
  - Logs and skips duplicate events

## Implementation Details

### New Functions Created

1. **`handle_ci_status(database, event)`**
   - Entry point for CI status events
   - Routes to appropriate handler based on event type
   - Supports both check_run and check_suite

2. **`handle_check_run_completed(database, event, payload)`**
   - Parses check_run.completed payloads
   - Extracts check details and failure information
   - Creates issue-fixer agent with rich context
   - Links to pr-shepherd if available

3. **`handle_check_suite_completed(database, event, payload)`**
   - Parses check_suite.completed payloads
   - Similar logic to check_run handler
   - Handles suite-level failures

4. **`find_duplicate_ci_fixer(database, check_id, head_sha, pr_number)`**
   - Searches for existing issue-fixer agents
   - Matches on check/suite ID, commit SHA, and PR number
   - Returns agent ID if duplicate found
   - Allows new agent if commit is different (retry scenario)

### Modified Files

1. **`crates/orchestrate-web/src/event_handlers.rs`**
   - Added 3 new handler functions (460+ lines)
   - Added 1 duplicate detection helper function
   - Added 11 comprehensive unit tests (460+ lines)
   - Total new code: ~920 lines

2. **`crates/orchestrate-web/src/webhook_processor.rs`**
   - Updated `handle_event()` to route check_run and check_suite events
   - Added pattern matching for both event types

3. **`crates/orchestrate-web/tests/ci_status_integration_test.rs`**
   - New integration test file
   - 6 end-to-end tests covering all scenarios
   - Tests check_run and check_suite events
   - Tests duplicate prevention and shepherd linking

## Test Coverage

### Unit Tests (11 tests)
- `test_handle_ci_check_run_failure_creates_agent` - Basic check_run failure
- `test_handle_ci_check_run_timed_out_creates_agent` - Timeout handling
- `test_handle_ci_check_run_success_skipped` - Skip successful checks
- `test_handle_ci_check_run_without_pr` - CI on non-PR branches
- `test_handle_ci_check_run_links_to_shepherd` - Link to existing shepherd
- `test_handle_ci_check_run_skips_duplicate` - Duplicate prevention
- `test_handle_ci_check_run_different_commit_not_duplicate` - Retry after fix
- `test_handle_ci_check_suite_failure_creates_agent` - Suite failure handling
- `test_handle_ci_check_suite_success_skipped` - Skip successful suites
- `test_handle_ci_check_run_non_completed_action` - Skip non-completed
- `test_handle_ci_check_run_missing_fields` - Error handling

### Integration Tests (6 tests)
- `test_check_run_failure_processing` - End-to-end check_run processing
- `test_check_run_success_not_processed` - Verify success is skipped
- `test_check_suite_timeout_processing` - End-to-end suite timeout
- `test_ci_failure_links_to_shepherd` - Verify shepherd linking
- `test_ci_failure_duplicate_prevention` - Verify deduplication works
- `test_ci_failure_without_pr` - CI on main/other branches

All tests passing: ✅ (83 total tests in orchestrate-web)

## Agent Context Structure

When a CI failure is detected, the created issue-fixer agent has the following context:

```json
{
  "pr_number": 42,
  "branch_name": "feature/test",
  "custom": {
    "repository": "owner/repo",
    "event_delivery_id": "delivery-123",
    "ci_check_name": "Build",           // check_run only
    "ci_check_id": 12345,               // check_run only
    "ci_suite_id": 98765,               // check_suite only
    "ci_conclusion": "failure",         // or "timed_out"
    "ci_head_sha": "abc123def456",
    "ci_head_branch": "feature/test",
    "ci_details_url": "https://...",    // check_run only
    "ci_html_url": "https://...",       // check_run only
    "shepherd_agent_id": "uuid..."      // if shepherd exists
  }
}
```

## Deduplication Logic

The system prevents duplicate fixers by checking:

1. **Same check/suite ID** - Must match ci_check_id or ci_suite_id
2. **Same commit SHA** - Must match ci_head_sha
3. **Same PR** - Must match pr_number (or both None)

If all three match, the event is skipped as a duplicate.

**Important**: If the commit SHA is different, a new fixer is created. This allows the system to retry after a fix attempt fails.

## Example Scenarios

### Scenario 1: CI Fails on PR
```
1. PR #42 opened → pr-shepherd created
2. CI check fails on PR #42 → issue-fixer created, linked to shepherd
3. Same CI check fails again (webhook retry) → skipped as duplicate
4. Fixer pushes a fix, new commit
5. CI check fails on new commit → new issue-fixer created (different SHA)
```

### Scenario 2: CI Fails on Main Branch
```
1. Push to main branch
2. CI check fails → issue-fixer created without PR context
3. Agent has branch_name="main", pr_number=None
```

### Scenario 3: Check Suite Timeout
```
1. PR #99 has slow tests
2. Check suite times out → issue-fixer created with conclusion="timed_out"
3. Agent context includes ci_suite_id and timeout details
```

## How to Test

### Manual Testing

1. Start webhook server (if implemented)
2. Configure GitHub webhook for check_run and check_suite events
3. Create a PR and trigger CI failure
4. Check database for issue-fixer agent creation
5. Verify agent has correct context

### Unit Tests
```bash
cd /Users/martinjanci/projects/github.com/hanibalsk/orchestrate/.worktrees/epic-002-webhooks
cargo test --package orchestrate-web event_handlers::tests::test_handle_ci
```

### Integration Tests
```bash
cargo test --package orchestrate-web --test ci_status_integration_test
```

### All Tests
```bash
cargo test --package orchestrate-web
```

## Related Files

- `/Users/martinjanci/projects/github.com/hanibalsk/orchestrate/.worktrees/epic-002-webhooks/crates/orchestrate-web/src/event_handlers.rs`
- `/Users/martinjanci/projects/github.com/hanibalsk/orchestrate/.worktrees/epic-002-webhooks/crates/orchestrate-web/src/webhook_processor.rs`
- `/Users/martinjanci/projects/github.com/hanibalsk/orchestrate/.worktrees/epic-002-webhooks/crates/orchestrate-web/tests/ci_status_integration_test.rs`
- `/Users/martinjanci/projects/github.com/hanibalsk/orchestrate/docs/bmad/epics/epic-002-webhook-triggers.md`

## Next Steps

Story 5 is complete. The next stories in Epic 002 are:

- Story 6: Push to Main Event Handler
- Story 7: Issue Created Event Handler
- Story 8: Webhook CLI Commands
- Story 9: Webhook Configuration

## Technical Notes

### GitHub Webhook Payloads

The implementation follows GitHub's webhook payload structure:

**check_run.completed:**
```json
{
  "action": "completed",
  "check_run": {
    "id": 12345,
    "name": "Build",
    "conclusion": "failure",
    "head_sha": "abc123",
    "pull_requests": [{"number": 42}],
    "check_suite": {"head_branch": "feature/test"},
    "details_url": "...",
    "html_url": "..."
  },
  "repository": {"full_name": "owner/repo"}
}
```

**check_suite.completed:**
```json
{
  "action": "completed",
  "check_suite": {
    "id": 98765,
    "conclusion": "timed_out",
    "head_sha": "def456",
    "head_branch": "main",
    "pull_requests": []
  },
  "repository": {"full_name": "owner/repo"}
}
```

### Performance Considerations

- Duplicate detection queries all agents (could be optimized with index)
- For large repositories with many failures, consider:
  - Adding database index on (agent_type, custom->ci_check_id)
  - Limiting lookback window for duplicates
  - Cleaning up old failed agents

### Security Considerations

- CI failure context may contain sensitive information in URLs
- Ensure webhook signature verification is enabled (Story 1)
- Consider filtering/sanitizing URLs before storing
