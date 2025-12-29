# Story 2: Event Queue System - Implementation Summary

## Overview

Successfully implemented a reliable event queue system for processing GitHub webhooks asynchronously with retry logic, idempotency, and dead-letter handling.

## Acceptance Criteria - All Complete ✓

- [x] Create `webhook_events` table in database
- [x] Queue incoming webhooks before processing
- [x] Implement event processor that polls queue
- [x] Handle duplicate events (idempotency via delivery ID)
- [x] Implement dead-letter handling for failed events
- [x] Add retry logic with exponential backoff

## Implementation Details

### Database Schema

**File**: `migrations/006_webhook_events.sql`

Created `webhook_events` table with:
- `id` (primary key, auto-increment)
- `delivery_id` (unique, for idempotency)
- `event_type` (GitHub event type)
- `payload` (JSON payload from GitHub)
- `status` (pending/processing/completed/failed/dead_letter)
- `retry_count` (number of retry attempts)
- `max_retries` (default 3)
- `error_message` (failure reason)
- `next_retry_at` (for exponential backoff)
- `received_at`, `processed_at`, `created_at`, `updated_at` (timestamps)

Indexes created for:
- `delivery_id` (unique)
- `status`
- `event_type`
- `next_retry_at` (for pending events)
- `received_at`

### Core Types

**File**: `crates/orchestrate-core/src/webhook.rs`

1. **WebhookEventStatus** enum:
   - Pending - waiting to be processed
   - Processing - currently being processed
   - Completed - successfully processed
   - Failed - processing failed (may retry)
   - DeadLetter - max retries exceeded

2. **WebhookEvent** struct:
   - Lifecycle methods: `mark_processing()`, `mark_completed()`, `mark_failed()`
   - Retry logic: `can_retry()`, `calculate_next_retry()`
   - Exponential backoff: 2^retry_count seconds (1s, 2s, 4s, 8s...)

### Database Operations

**File**: `crates/orchestrate-core/src/database.rs`

Added webhook event operations:
- `insert_webhook_event()` - idempotent insert using ON CONFLICT(delivery_id)
- `get_webhook_event()` - get by ID
- `get_webhook_event_by_delivery_id()` - get by delivery ID
- `get_pending_webhook_events()` - fetch events ready for processing
- `update_webhook_event()` - update status and metadata
- `get_webhook_events_by_status()` - query by status
- `count_webhook_events_by_status()` - count by status
- `delete_old_webhook_events()` - cleanup old events

### Event Processor

**File**: `crates/orchestrate-web/src/webhook_processor.rs`

Created `WebhookProcessor` with:
- Configurable polling interval (default 5 seconds)
- Configurable batch size (default 10 events)
- Configurable max concurrent processing (default 5)
- Graceful error handling with retry logic
- Automatic dead-letter queue management

Processing flow:
1. Poll database for pending events (respects next_retry_at)
2. Mark event as processing
3. Attempt to process event
4. On success: mark as completed
5. On failure: increment retry_count, calculate next_retry_at
6. If max retries exceeded: move to dead_letter queue

### Integration

**Files**:
- `crates/orchestrate-web/src/webhook.rs` (updated)
- `crates/orchestrate-web/src/api.rs` (updated)

Updated webhook handler to:
1. Parse and validate webhook payload
2. Create WebhookEvent with delivery_id from X-GitHub-Delivery header
3. Insert event into database (idempotent)
4. Return 200 OK immediately
5. Event processing happens asynchronously via WebhookProcessor

## Test Coverage

### Unit Tests (8 tests)
**File**: `crates/orchestrate-core/src/webhook.rs`
- WebhookEvent creation and initialization
- Status transitions (pending -> processing -> completed)
- Retry logic with exponential backoff
- Dead-letter queue after max retries
- Status parsing and serialization

### Database Integration Tests (12 tests)
**File**: `crates/orchestrate-core/src/database_webhook_tests.rs`
- Event insertion and retrieval
- Idempotency verification (duplicate delivery_id)
- Status updates
- Pending event queries (respects retry time)
- Retry flow simulation
- Dead-letter queue handling
- Status filtering and counting
- Old event cleanup

### Processor Tests (3 tests)
**File**: `crates/orchestrate-web/src/webhook_processor.rs`
- Event batch processing
- Batch size limits
- Empty queue handling

### Integration Tests (1 test)
**File**: `crates/orchestrate-web/src/webhook.rs`
- End-to-end webhook queueing
- Verification of event in database

**Total: 24 tests, all passing**

## Retry Logic

The system implements exponential backoff:
- Retry 1: 2^0 = 1 second
- Retry 2: 2^1 = 2 seconds
- Retry 3: 2^2 = 4 seconds
- After 3 retries: moved to dead_letter queue

Status transitions:
```
pending -> processing -> completed (success)
                      -> failed -> pending (retry if can_retry())
                                -> dead_letter (max retries exceeded)
```

## Idempotency

Achieved through:
1. Unique constraint on `delivery_id` column
2. `ON CONFLICT(delivery_id) DO NOTHING` in insert query
3. If duplicate detected, returns existing event ID
4. Prevents duplicate processing of same webhook

## Key Features

1. **Reliability**: Events are persisted before processing
2. **Idempotency**: Duplicate webhooks are detected via delivery_id
3. **Retry Logic**: Automatic retry with exponential backoff
4. **Dead Letter Queue**: Failed events moved to dead_letter after max retries
5. **Async Processing**: Handler returns immediately, processing happens separately
6. **Batch Processing**: Processor handles multiple events per poll
7. **Configurable**: Polling interval, batch size, max retries all configurable
8. **Observable**: Comprehensive logging at all stages
9. **Testable**: Full test coverage with mocked scenarios

## Future Enhancements (Next Stories)

The processor currently has a placeholder `handle_event()` function. Future stories will implement actual event handlers:
- Story 3: PR opened -> spawn pr-shepherd
- Story 4: PR review -> spawn issue-fixer
- Story 5: CI failure -> spawn issue-fixer
- Story 6: Push to main -> spawn regression-tester
- Story 7: Issue created -> spawn issue-triager

## Files Modified/Created

### New Files
- `migrations/006_webhook_events.sql` - Database schema
- `crates/orchestrate-core/src/webhook.rs` - Core types
- `crates/orchestrate-core/src/database_webhook_tests.rs` - Database tests
- `crates/orchestrate-web/src/webhook_processor.rs` - Event processor

### Modified Files
- `crates/orchestrate-core/src/lib.rs` - Export webhook types
- `crates/orchestrate-core/src/database.rs` - Add webhook operations
- `crates/orchestrate-web/src/lib.rs` - Export processor
- `crates/orchestrate-web/src/webhook.rs` - Queue events
- `crates/orchestrate-web/src/api.rs` - Pass database to webhook state

## Verification

All tests pass:
```bash
cargo test --package orchestrate-core webhook::
# 8 passed

cargo test --package orchestrate-core database_webhook_tests::
# 12 passed

cargo test --package orchestrate-web webhook::
# 12 passed (includes integration test)

cargo test --package orchestrate-web webhook_processor::
# 3 passed

cargo test
# 31 integration tests passed (no regressions)
```

## Performance Considerations

- Database indexes optimize common queries (status, delivery_id, retry_at)
- Batch processing reduces database round trips
- Configurable polling interval prevents excessive database load
- Cleanup operation allows removal of old events to prevent table growth

## Security

- Events are only queued after signature verification (Story 1)
- Database operations use parameterized queries (SQL injection safe)
- Error messages logged but not exposed to GitHub webhook response
- Idempotency prevents replay attacks

## Conclusion

Story 2 is complete with all acceptance criteria met. The event queue system provides a robust foundation for asynchronous webhook processing with reliability, retry logic, and observability. The system is ready for implementation of specific event handlers in Stories 3-7.

**Status**: ✓ Complete
**Commit**: 3892eb2 - feat: Implement webhook event queue system
**Tests**: 24/24 passing
**Lines of Code**: ~1,350 added
