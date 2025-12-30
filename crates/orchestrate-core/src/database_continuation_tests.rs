//! Database tests for agent continuation operations

use crate::agent_continuation::{
    AgentContinuation, ContinuationBuilder, ContinuationReason, ContinuationResult,
    ContinuationStatus,
};
use crate::Database;

#[tokio::test]
async fn test_create_and_get_continuation() {
    let db = Database::in_memory().await.unwrap();

    let continuation = AgentContinuation::new(
        "agent-123",
        ContinuationReason::ReviewFeedback,
        "Please fix the formatting",
    );

    let id = db.create_continuation(&continuation).await.unwrap();
    assert!(id > 0);

    let retrieved = db.get_continuation(id).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.agent_id, "agent-123");
    assert_eq!(retrieved.reason, ContinuationReason::ReviewFeedback);
    assert_eq!(retrieved.message, "Please fix the formatting");
    assert_eq!(retrieved.status, ContinuationStatus::Pending);
}

#[tokio::test]
async fn test_update_continuation() {
    let db = Database::in_memory().await.unwrap();

    let mut continuation = AgentContinuation::new(
        "agent-456",
        ContinuationReason::TestFailures,
        "Fix the failing tests",
    );

    let id = db.create_continuation(&continuation).await.unwrap();
    continuation.id = id;

    // Start execution
    continuation.start_execution();
    db.update_continuation(&continuation).await.unwrap();

    let retrieved = db.get_continuation(id).await.unwrap().unwrap();
    assert_eq!(retrieved.status, ContinuationStatus::Executing);
    assert!(retrieved.started_at.is_some());

    // Complete
    continuation.complete(ContinuationResult {
        success: true,
        summary: Some("Fixed all tests".to_string()),
        new_agent_state: Some("completed".to_string()),
        files_changed: vec!["src/lib.rs".to_string()],
        tests_affected: vec!["test_feature".to_string()],
    });
    db.update_continuation(&continuation).await.unwrap();

    let retrieved = db.get_continuation(id).await.unwrap().unwrap();
    assert_eq!(retrieved.status, ContinuationStatus::Completed);
    assert!(retrieved.completed_at.is_some());
    assert!(retrieved.result.is_some());
    assert!(retrieved.result.unwrap().success);
}

#[tokio::test]
async fn test_continuation_with_session() {
    let db = Database::in_memory().await.unwrap();

    let continuation = AgentContinuation::new(
        "agent-789",
        ContinuationReason::AdditionalTask,
        "Add export feature",
    )
    .with_session("session-abc");

    let id = db.create_continuation(&continuation).await.unwrap();

    let retrieved = db.get_continuation(id).await.unwrap().unwrap();
    assert_eq!(retrieved.session_id, Some("session-abc".to_string()));
}

#[tokio::test]
async fn test_continuation_with_context() {
    let db = Database::in_memory().await.unwrap();

    let context = serde_json::json!({
        "pr_number": 123,
        "files": ["src/lib.rs", "src/main.rs"]
    });

    let continuation = AgentContinuation::new(
        "agent-abc",
        ContinuationReason::FixRequest,
        "Fix CI failure",
    )
    .with_context(context.clone());

    let id = db.create_continuation(&continuation).await.unwrap();

    let retrieved = db.get_continuation(id).await.unwrap().unwrap();
    assert_eq!(retrieved.context, context);
}

#[tokio::test]
async fn test_get_pending_continuations() {
    let db = Database::in_memory().await.unwrap();

    // Create multiple continuations for same agent
    let cont1 = AgentContinuation::new(
        "agent-pending",
        ContinuationReason::ReviewFeedback,
        "Feedback 1",
    );
    db.create_continuation(&cont1).await.unwrap();

    let cont2 = AgentContinuation::new(
        "agent-pending",
        ContinuationReason::TestFailures,
        "Test failures",
    );
    db.create_continuation(&cont2).await.unwrap();

    // Create one for different agent
    let cont3 = AgentContinuation::new(
        "agent-other",
        ContinuationReason::Retry,
        "Retry",
    );
    db.create_continuation(&cont3).await.unwrap();

    // Get pending for agent-pending
    let pending = db.get_pending_continuations("agent-pending").await.unwrap();
    assert_eq!(pending.len(), 2);
    assert_eq!(pending[0].message, "Feedback 1"); // Oldest first
    assert_eq!(pending[1].message, "Test failures");

    // Get pending for agent-other
    let pending = db.get_pending_continuations("agent-other").await.unwrap();
    assert_eq!(pending.len(), 1);
}

#[tokio::test]
async fn test_get_continuations_for_agent() {
    let db = Database::in_memory().await.unwrap();

    // Create continuations with different statuses
    let cont1 = AgentContinuation::new(
        "agent-all",
        ContinuationReason::ReviewFeedback,
        "Pending",
    );
    db.create_continuation(&cont1).await.unwrap();

    let mut cont2 = AgentContinuation::new(
        "agent-all",
        ContinuationReason::TestFailures,
        "Completed",
    );
    cont2.start_execution();
    cont2.complete(ContinuationResult {
        success: true,
        summary: None,
        new_agent_state: None,
        files_changed: vec![],
        tests_affected: vec![],
    });
    db.create_continuation(&cont2).await.unwrap();

    let all = db.get_continuations_for_agent("agent-all").await.unwrap();
    assert_eq!(all.len(), 2);

    // Most recent first
    let statuses: Vec<_> = all.iter().map(|c| c.status).collect();
    assert!(statuses.contains(&ContinuationStatus::Pending));
    assert!(statuses.contains(&ContinuationStatus::Completed));
}

#[tokio::test]
async fn test_get_next_pending_continuation() {
    let db = Database::in_memory().await.unwrap();

    // No pending continuations
    let next = db.get_next_pending_continuation().await.unwrap();
    assert!(next.is_none());

    // Add some
    let cont1 = AgentContinuation::new("agent-1", ContinuationReason::Retry, "First");
    db.create_continuation(&cont1).await.unwrap();

    std::thread::sleep(std::time::Duration::from_millis(10));

    let cont2 = AgentContinuation::new("agent-2", ContinuationReason::Retry, "Second");
    db.create_continuation(&cont2).await.unwrap();

    // Should get oldest
    let next = db.get_next_pending_continuation().await.unwrap();
    assert!(next.is_some());
    assert_eq!(next.unwrap().message, "First");
}

#[tokio::test]
async fn test_cancel_pending_continuations() {
    let db = Database::in_memory().await.unwrap();

    // Create pending continuations
    let cont1 = AgentContinuation::new(
        "agent-cancel",
        ContinuationReason::ReviewFeedback,
        "Pending 1",
    );
    db.create_continuation(&cont1).await.unwrap();

    let cont2 = AgentContinuation::new(
        "agent-cancel",
        ContinuationReason::TestFailures,
        "Pending 2",
    );
    db.create_continuation(&cont2).await.unwrap();

    // Create one that's already executing
    let mut cont3 = AgentContinuation::new(
        "agent-cancel",
        ContinuationReason::Retry,
        "Executing",
    );
    cont3.start_execution();
    let id3 = db.create_continuation(&cont3).await.unwrap();
    // Update it manually to executing status
    cont3.id = id3;
    db.update_continuation(&cont3).await.unwrap();

    // Cancel pending
    let cancelled = db.cancel_pending_continuations("agent-cancel").await.unwrap();
    assert_eq!(cancelled, 2);

    // Verify statuses
    let all = db.get_continuations_for_agent("agent-cancel").await.unwrap();
    let cancelled_count = all
        .iter()
        .filter(|c| c.status == ContinuationStatus::Cancelled)
        .count();
    let executing_count = all
        .iter()
        .filter(|c| c.status == ContinuationStatus::Executing)
        .count();

    assert_eq!(cancelled_count, 2);
    assert_eq!(executing_count, 1);
}

#[tokio::test]
async fn test_count_continuations_by_status() {
    let db = Database::in_memory().await.unwrap();

    // Create continuations with different statuses
    let cont1 = AgentContinuation::new("agent-1", ContinuationReason::Retry, "Pending 1");
    db.create_continuation(&cont1).await.unwrap();

    let cont2 = AgentContinuation::new("agent-2", ContinuationReason::Retry, "Pending 2");
    db.create_continuation(&cont2).await.unwrap();

    let mut cont3 = AgentContinuation::new("agent-3", ContinuationReason::Retry, "Completed");
    cont3.start_execution();
    cont3.complete(ContinuationResult {
        success: true,
        summary: None,
        new_agent_state: None,
        files_changed: vec![],
        tests_affected: vec![],
    });
    db.create_continuation(&cont3).await.unwrap();

    let counts = db.count_continuations_by_status().await.unwrap();
    assert_eq!(counts.get("pending"), Some(&2));
    assert_eq!(counts.get("completed"), Some(&1));
}

#[tokio::test]
async fn test_continuation_builder_database() {
    let db = Database::in_memory().await.unwrap();

    let continuation = ContinuationBuilder::review_feedback("agent-builder")
        .session("session-123")
        .add_comment("src/lib.rs", Some(42), "Use snake_case")
        .add_comment("src/main.rs", None, "Add docs")
        .context(serde_json::json!({"reviewer": "human"}))
        .build();

    let id = db.create_continuation(&continuation).await.unwrap();

    let retrieved = db.get_continuation(id).await.unwrap().unwrap();
    assert_eq!(retrieved.reason, ContinuationReason::ReviewFeedback);
    assert_eq!(retrieved.session_id, Some("session-123".to_string()));
    assert!(retrieved.message.contains("src/lib.rs:42"));
    assert!(retrieved.message.contains("snake_case"));
    assert!(retrieved.context.get("reviewer").is_some());
}

#[tokio::test]
async fn test_continuation_failed_status() {
    let db = Database::in_memory().await.unwrap();

    let mut continuation = AgentContinuation::new(
        "agent-fail",
        ContinuationReason::FixRequest,
        "Fix bug",
    );

    let id = db.create_continuation(&continuation).await.unwrap();
    continuation.id = id;

    // Start and fail
    continuation.start_execution();
    continuation.fail("Agent crashed unexpectedly");
    db.update_continuation(&continuation).await.unwrap();

    let retrieved = db.get_continuation(id).await.unwrap().unwrap();
    assert_eq!(retrieved.status, ContinuationStatus::Failed);
    assert_eq!(
        retrieved.error_message,
        Some("Agent crashed unexpectedly".to_string())
    );
}

#[tokio::test]
async fn test_continuation_not_found() {
    let db = Database::in_memory().await.unwrap();

    let result = db.get_continuation(99999).await.unwrap();
    assert!(result.is_none());
}
