//! Database tests for edge case handling (Epic 016 - Story 14)

use crate::edge_case_handler::{EdgeCaseEvent, EdgeCaseLearning, EdgeCaseResolution, EdgeCaseType};
use crate::Database;

// ==================== EdgeCaseEvent Tests ====================

#[tokio::test]
async fn test_create_and_get_edge_case_event() {
    let db = Database::in_memory().await.unwrap();

    let event = EdgeCaseEvent::new(EdgeCaseType::FlakyTest)
        .with_session("session-123")
        .with_agent("agent-456")
        .with_story("story-789")
        .with_error("Test failed intermittently");

    let id = db.create_edge_case_event(&event).await.unwrap();
    assert!(id > 0);

    let retrieved = db.get_edge_case_event(id).await.unwrap().unwrap();
    assert_eq!(retrieved.id, id);
    assert_eq!(retrieved.session_id, Some("session-123".to_string()));
    assert_eq!(retrieved.agent_id, Some("agent-456".to_string()));
    assert_eq!(retrieved.story_id, Some("story-789".to_string()));
    assert_eq!(retrieved.edge_case_type, EdgeCaseType::FlakyTest);
    assert_eq!(retrieved.resolution, EdgeCaseResolution::Pending);
    assert_eq!(
        retrieved.error_message,
        Some("Test failed intermittently".to_string())
    );
}

#[tokio::test]
async fn test_update_edge_case_event() {
    let db = Database::in_memory().await.unwrap();

    let mut event = EdgeCaseEvent::new(EdgeCaseType::MergeConflict)
        .with_session("session-1")
        .with_agent("agent-1");

    let id = db.create_edge_case_event(&event).await.unwrap();
    event.id = id;

    // Update resolution
    event.resolve(
        EdgeCaseResolution::AutoResolved,
        Some("Conflict resolved automatically".to_string()),
    );
    event.action_taken = Some("spawn_resolver".to_string());
    event.retry_count = 1;

    db.update_edge_case_event(&event).await.unwrap();

    let retrieved = db.get_edge_case_event(id).await.unwrap().unwrap();
    assert_eq!(retrieved.resolution, EdgeCaseResolution::AutoResolved);
    assert_eq!(
        retrieved.action_taken,
        Some("spawn_resolver".to_string())
    );
    assert_eq!(retrieved.retry_count, 1);
    assert!(retrieved.resolved_at.is_some());
    assert_eq!(
        retrieved.resolution_notes,
        Some("Conflict resolved automatically".to_string())
    );
}

#[tokio::test]
async fn test_get_edge_case_events_for_session() {
    let db = Database::in_memory().await.unwrap();

    // Create events for different sessions
    let event1 = EdgeCaseEvent::new(EdgeCaseType::FlakyTest).with_session("session-A");
    let event2 = EdgeCaseEvent::new(EdgeCaseType::RateLimit).with_session("session-A");
    let event3 = EdgeCaseEvent::new(EdgeCaseType::Timeout).with_session("session-B");

    db.create_edge_case_event(&event1).await.unwrap();
    db.create_edge_case_event(&event2).await.unwrap();
    db.create_edge_case_event(&event3).await.unwrap();

    let events = db
        .get_edge_case_events_for_session("session-A")
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
    assert!(events.iter().all(|e| e.session_id == Some("session-A".to_string())));
}

#[tokio::test]
async fn test_get_edge_case_events_for_agent() {
    let db = Database::in_memory().await.unwrap();

    let event1 = EdgeCaseEvent::new(EdgeCaseType::FlakyTest).with_agent("agent-X");
    let event2 = EdgeCaseEvent::new(EdgeCaseType::RateLimit).with_agent("agent-X");
    let event3 = EdgeCaseEvent::new(EdgeCaseType::Timeout).with_agent("agent-Y");

    db.create_edge_case_event(&event1).await.unwrap();
    db.create_edge_case_event(&event2).await.unwrap();
    db.create_edge_case_event(&event3).await.unwrap();

    let events = db
        .get_edge_case_events_for_agent("agent-X")
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn test_get_unresolved_edge_case_events() {
    let db = Database::in_memory().await.unwrap();

    // Create events with different resolutions
    let event1 = EdgeCaseEvent::new(EdgeCaseType::FlakyTest);
    let mut event2 = EdgeCaseEvent::new(EdgeCaseType::RateLimit);
    event2.resolution = EdgeCaseResolution::AutoResolved;
    let mut event3 = EdgeCaseEvent::new(EdgeCaseType::Timeout);
    event3.resolution = EdgeCaseResolution::Retrying;

    db.create_edge_case_event(&event1).await.unwrap();
    db.create_edge_case_event(&event2).await.unwrap();
    db.create_edge_case_event(&event3).await.unwrap();

    let unresolved = db.get_unresolved_edge_case_events().await.unwrap();
    assert_eq!(unresolved.len(), 2); // Pending and Retrying
}

#[tokio::test]
async fn test_get_edge_case_events_by_type() {
    let db = Database::in_memory().await.unwrap();

    db.create_edge_case_event(&EdgeCaseEvent::new(EdgeCaseType::FlakyTest))
        .await
        .unwrap();
    db.create_edge_case_event(&EdgeCaseEvent::new(EdgeCaseType::FlakyTest))
        .await
        .unwrap();
    db.create_edge_case_event(&EdgeCaseEvent::new(EdgeCaseType::RateLimit))
        .await
        .unwrap();

    let events = db
        .get_edge_case_events_by_type("flaky_test", Some(10))
        .await
        .unwrap();
    assert_eq!(events.len(), 2);
}

#[tokio::test]
async fn test_get_edge_case_stats() {
    let db = Database::in_memory().await.unwrap();

    // Create events with various types and resolutions
    let mut event1 = EdgeCaseEvent::new(EdgeCaseType::FlakyTest);
    event1.resolution = EdgeCaseResolution::AutoResolved;
    event1.resolved_at = Some(chrono::Utc::now());

    let mut event2 = EdgeCaseEvent::new(EdgeCaseType::FlakyTest);
    event2.resolution = EdgeCaseResolution::Failed;

    let event3 = EdgeCaseEvent::new(EdgeCaseType::RateLimit);
    // Pending

    db.create_edge_case_event(&event1).await.unwrap();
    db.create_edge_case_event(&event2).await.unwrap();
    db.create_edge_case_event(&event3).await.unwrap();

    let stats = db.get_edge_case_stats().await.unwrap();
    assert_eq!(stats.total, 3);
    assert_eq!(stats.resolved, 1);
    assert_eq!(stats.failed, 1);
    assert_eq!(stats.pending, 1);
    assert_eq!(stats.by_type.get("flaky_test"), Some(&2));
    assert_eq!(stats.by_type.get("rate_limit"), Some(&1));
}

// ==================== EdgeCaseLearning Tests ====================

#[tokio::test]
async fn test_upsert_edge_case_learning_insert() {
    let db = Database::in_memory().await.unwrap();

    let learning = EdgeCaseLearning::new(EdgeCaseType::FlakyTest, "test.*timeout");

    let id = db.upsert_edge_case_learning(&learning).await.unwrap();
    assert!(id > 0);

    let retrieved = db
        .get_edge_case_learning("flaky_test", "test.*timeout")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.edge_case_type, EdgeCaseType::FlakyTest);
    assert_eq!(retrieved.pattern, "test.*timeout");
    assert_eq!(retrieved.occurrence_count, 1);
}

#[tokio::test]
async fn test_upsert_edge_case_learning_update() {
    let db = Database::in_memory().await.unwrap();

    let mut learning = EdgeCaseLearning::new(EdgeCaseType::FlakyTest, "test.*timeout");
    db.upsert_edge_case_learning(&learning).await.unwrap();

    // Update with new data
    learning.record_occurrence(true, Some(30.0));
    learning.record_occurrence(true, Some(20.0));

    let id = db.upsert_edge_case_learning(&learning).await.unwrap();

    let retrieved = db
        .get_edge_case_learning("flaky_test", "test.*timeout")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.id, id);
    assert_eq!(retrieved.occurrence_count, 3);
    assert!(retrieved.success_rate > 0.5);
}

#[tokio::test]
async fn test_get_edge_case_learnings_by_type() {
    let db = Database::in_memory().await.unwrap();

    let learning1 = EdgeCaseLearning::new(EdgeCaseType::FlakyTest, "pattern-1");
    let learning2 = EdgeCaseLearning::new(EdgeCaseType::FlakyTest, "pattern-2");
    let learning3 = EdgeCaseLearning::new(EdgeCaseType::RateLimit, "pattern-3");

    db.upsert_edge_case_learning(&learning1).await.unwrap();
    db.upsert_edge_case_learning(&learning2).await.unwrap();
    db.upsert_edge_case_learning(&learning3).await.unwrap();

    let learnings = db
        .get_edge_case_learnings_by_type("flaky_test")
        .await
        .unwrap();
    assert_eq!(learnings.len(), 2);
}

#[tokio::test]
async fn test_get_top_edge_case_learnings() {
    let db = Database::in_memory().await.unwrap();

    // Create learnings with different success rates and occurrence counts
    let mut learning1 = EdgeCaseLearning::new(EdgeCaseType::FlakyTest, "pattern-high");
    learning1.success_rate = 0.9;
    learning1.occurrence_count = 10;

    let mut learning2 = EdgeCaseLearning::new(EdgeCaseType::FlakyTest, "pattern-low");
    learning2.success_rate = 0.5;
    learning2.occurrence_count = 5;

    let mut learning3 = EdgeCaseLearning::new(EdgeCaseType::RateLimit, "pattern-few");
    learning3.success_rate = 1.0;
    learning3.occurrence_count = 2; // Below threshold of 3

    db.upsert_edge_case_learning(&learning1).await.unwrap();
    db.upsert_edge_case_learning(&learning2).await.unwrap();
    db.upsert_edge_case_learning(&learning3).await.unwrap();

    let top = db.get_top_edge_case_learnings(10).await.unwrap();
    // Should only include learnings with occurrence_count >= 3
    assert_eq!(top.len(), 2);
    // First should be highest success rate
    assert_eq!(top[0].pattern, "pattern-high");
}

// ==================== All Edge Case Types ====================

#[tokio::test]
async fn test_all_edge_case_types() {
    let db = Database::in_memory().await.unwrap();

    let types = vec![
        EdgeCaseType::DelayedCiReview,
        EdgeCaseType::MergeConflict,
        EdgeCaseType::FlakyTest,
        EdgeCaseType::ServiceDowntime,
        EdgeCaseType::DependencyFailure,
        EdgeCaseType::ReviewPingPong,
        EdgeCaseType::ContextOverflow,
        EdgeCaseType::RateLimit,
        EdgeCaseType::Timeout,
        EdgeCaseType::AuthError,
        EdgeCaseType::NetworkError,
        EdgeCaseType::Unknown,
    ];

    for edge_type in types {
        let event = EdgeCaseEvent::new(edge_type);
        let id = db.create_edge_case_event(&event).await.unwrap();
        let retrieved = db.get_edge_case_event(id).await.unwrap().unwrap();
        assert_eq!(retrieved.edge_case_type, edge_type);
    }
}

// ==================== All Resolution Types ====================

#[tokio::test]
async fn test_all_resolution_types() {
    let db = Database::in_memory().await.unwrap();

    let resolutions = vec![
        EdgeCaseResolution::Pending,
        EdgeCaseResolution::AutoResolved,
        EdgeCaseResolution::ManualResolved,
        EdgeCaseResolution::Bypassed,
        EdgeCaseResolution::Failed,
        EdgeCaseResolution::Retrying,
        EdgeCaseResolution::Waiting,
    ];

    for resolution in resolutions {
        let mut event = EdgeCaseEvent::new(EdgeCaseType::Unknown);
        event.resolution = resolution;

        let id = db.create_edge_case_event(&event).await.unwrap();
        let retrieved = db.get_edge_case_event(id).await.unwrap().unwrap();
        assert_eq!(retrieved.resolution, resolution);
    }
}
