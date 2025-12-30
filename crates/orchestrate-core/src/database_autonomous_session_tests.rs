//! Database tests for autonomous session operations

use crate::autonomous_session::{
    AutonomousSession, AutonomousSessionState, CompletedItem, SessionConfig, WorkItem,
    WorkItemType,
};
use crate::Database;
use chrono::Utc;

#[tokio::test]
async fn test_create_and_get_autonomous_session() {
    let db = Database::in_memory().await.unwrap();

    let session = AutonomousSession::with_id("test-session-1");

    db.create_autonomous_session(&session).await.unwrap();

    let retrieved = db.get_autonomous_session("test-session-1").await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.id, "test-session-1");
    assert_eq!(retrieved.state, AutonomousSessionState::Idle);
}

#[tokio::test]
async fn test_update_autonomous_session() {
    let db = Database::in_memory().await.unwrap();

    let mut session = AutonomousSession::with_id("test-session-2");
    db.create_autonomous_session(&session).await.unwrap();

    // Update session
    session.start().unwrap();
    session.set_current_epic("epic-016");
    session.set_current_story("story-1");
    db.update_autonomous_session(&session).await.unwrap();

    let retrieved = db
        .get_autonomous_session("test-session-2")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.state, AutonomousSessionState::Analyzing);
    assert_eq!(retrieved.current_epic_id, Some("epic-016".to_string()));
    assert_eq!(retrieved.current_story_id, Some("story-1".to_string()));
}

#[tokio::test]
async fn test_session_with_config() {
    let db = Database::in_memory().await.unwrap();

    let config = SessionConfig {
        max_agents: 5,
        epic_pattern: Some("epic-016-*".to_string()),
        model: Some("claude-opus-4".to_string()),
        dry_run: true,
        max_retries: 5,
        auto_merge: false,
    };

    let session = AutonomousSession::with_id("test-session-config").with_config(config);
    db.create_autonomous_session(&session).await.unwrap();

    let retrieved = db
        .get_autonomous_session("test-session-config")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.config.max_agents, 5);
    assert_eq!(
        retrieved.config.epic_pattern,
        Some("epic-016-*".to_string())
    );
    assert_eq!(retrieved.config.model, Some("claude-opus-4".to_string()));
    assert!(retrieved.config.dry_run);
    assert_eq!(retrieved.config.max_retries, 5);
    assert!(!retrieved.config.auto_merge);
}

#[tokio::test]
async fn test_session_with_work_queue() {
    let db = Database::in_memory().await.unwrap();

    let mut session = AutonomousSession::with_id("test-session-queue");

    session.add_work_item(WorkItem {
        id: "work-1".to_string(),
        work_type: WorkItemType::Story,
        epic_id: "epic-1".to_string(),
        story_id: Some("story-1".to_string()),
        priority: 1,
        dependencies: vec!["work-0".to_string()],
        metadata: serde_json::json!({"complexity": "medium"}),
    });

    session.add_work_item(WorkItem {
        id: "work-2".to_string(),
        work_type: WorkItemType::Review,
        epic_id: "epic-1".to_string(),
        story_id: None,
        priority: 2,
        dependencies: vec![],
        metadata: serde_json::Value::Null,
    });

    db.create_autonomous_session(&session).await.unwrap();

    let retrieved = db
        .get_autonomous_session("test-session-queue")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.work_queue.len(), 2);
    assert_eq!(retrieved.work_queue[0].id, "work-1");
    assert_eq!(retrieved.work_queue[0].work_type, WorkItemType::Story);
    assert_eq!(
        retrieved.work_queue[0].dependencies,
        vec!["work-0".to_string()]
    );
}

#[tokio::test]
async fn test_session_with_completed_items() {
    let db = Database::in_memory().await.unwrap();

    let mut session = AutonomousSession::with_id("test-session-completed");

    session.record_completed(CompletedItem {
        id: "work-1".to_string(),
        work_type: WorkItemType::Story,
        epic_id: "epic-1".to_string(),
        story_id: Some("story-1".to_string()),
        success: true,
        completed_at: Utc::now(),
        error: None,
        agent_id: Some("agent-123".to_string()),
    });

    session.record_completed(CompletedItem {
        id: "work-2".to_string(),
        work_type: WorkItemType::Story,
        epic_id: "epic-1".to_string(),
        story_id: Some("story-2".to_string()),
        success: false,
        completed_at: Utc::now(),
        error: Some("Test failure".to_string()),
        agent_id: Some("agent-456".to_string()),
    });

    db.create_autonomous_session(&session).await.unwrap();

    let retrieved = db
        .get_autonomous_session("test-session-completed")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.completed_items.len(), 2);
    assert!(retrieved.completed_items[0].success);
    assert!(!retrieved.completed_items[1].success);
    assert_eq!(
        retrieved.completed_items[1].error,
        Some("Test failure".to_string())
    );

    // Check metrics
    assert_eq!(retrieved.metrics.stories_completed, 1);
    assert_eq!(retrieved.metrics.stories_failed, 1);
}

#[tokio::test]
async fn test_session_with_metrics() {
    let db = Database::in_memory().await.unwrap();

    let mut session = AutonomousSession::with_id("test-session-metrics");
    session.metrics.stories_completed = 5;
    session.metrics.stories_failed = 2;
    session.metrics.reviews_passed = 4;
    session.metrics.reviews_failed = 1;
    session.metrics.total_iterations = 12;
    session.metrics.agents_spawned = 7;
    session.metrics.tokens_used = 150000;
    session
        .metrics
        .state_durations
        .insert("executing".to_string(), 3600);
    session
        .metrics
        .state_durations
        .insert("reviewing".to_string(), 1800);

    db.create_autonomous_session(&session).await.unwrap();

    let retrieved = db
        .get_autonomous_session("test-session-metrics")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.metrics.stories_completed, 5);
    assert_eq!(retrieved.metrics.stories_failed, 2);
    assert_eq!(retrieved.metrics.reviews_passed, 4);
    assert_eq!(retrieved.metrics.reviews_failed, 1);
    assert_eq!(retrieved.metrics.total_iterations, 12);
    assert_eq!(retrieved.metrics.agents_spawned, 7);
    assert_eq!(retrieved.metrics.tokens_used, 150000);
    assert_eq!(
        retrieved.metrics.state_durations.get("executing"),
        Some(&3600)
    );
    assert_eq!(
        retrieved.metrics.state_durations.get("reviewing"),
        Some(&1800)
    );
}

#[tokio::test]
async fn test_session_paused_blocked() {
    let db = Database::in_memory().await.unwrap();

    // Test paused session
    let mut session = AutonomousSession::with_id("test-session-paused");
    session.start().unwrap();
    session.pause("User requested pause").unwrap();

    db.create_autonomous_session(&session).await.unwrap();

    let retrieved = db
        .get_autonomous_session("test-session-paused")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved.state, AutonomousSessionState::Paused);
    assert_eq!(
        retrieved.pause_reason,
        Some("User requested pause".to_string())
    );

    // Test blocked session
    let mut session2 = AutonomousSession::with_id("test-session-blocked");
    session2.start().unwrap();
    session2.block("CI failure").unwrap();

    db.create_autonomous_session(&session2).await.unwrap();

    let retrieved2 = db
        .get_autonomous_session("test-session-blocked")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retrieved2.state, AutonomousSessionState::Blocked);
    assert_eq!(retrieved2.blocked_reason, Some("CI failure".to_string()));
    assert_eq!(retrieved2.error_message, Some("CI failure".to_string()));
}

#[tokio::test]
async fn test_list_autonomous_sessions() {
    let db = Database::in_memory().await.unwrap();

    // Create sessions with different states
    let session1 = AutonomousSession::with_id("session-idle");
    db.create_autonomous_session(&session1).await.unwrap();

    let mut session2 = AutonomousSession::with_id("session-analyzing");
    session2.start().unwrap();
    db.create_autonomous_session(&session2).await.unwrap();

    let mut session3 = AutonomousSession::with_id("session-paused");
    session3.start().unwrap();
    session3.pause("break").unwrap();
    db.create_autonomous_session(&session3).await.unwrap();

    // List all
    let all = db.list_autonomous_sessions(None, None).await.unwrap();
    assert_eq!(all.len(), 3);

    // List by state
    let idle = db
        .list_autonomous_sessions(Some("idle"), None)
        .await
        .unwrap();
    assert_eq!(idle.len(), 1);
    assert_eq!(idle[0].id, "session-idle");

    let analyzing = db
        .list_autonomous_sessions(Some("analyzing"), None)
        .await
        .unwrap();
    assert_eq!(analyzing.len(), 1);

    // List with limit
    let limited = db.list_autonomous_sessions(None, Some(2)).await.unwrap();
    assert_eq!(limited.len(), 2);
}

#[tokio::test]
async fn test_get_active_autonomous_session() {
    let db = Database::in_memory().await.unwrap();

    // Create sessions - one active, one done, one paused
    let mut session1 = AutonomousSession::with_id("session-done");
    session1.start().unwrap();
    session1
        .transition_to(AutonomousSessionState::Discovering)
        .unwrap();
    session1
        .transition_to(AutonomousSessionState::Planning)
        .unwrap();
    session1
        .transition_to(AutonomousSessionState::Executing)
        .unwrap();
    session1
        .transition_to(AutonomousSessionState::Reviewing)
        .unwrap();
    session1
        .transition_to(AutonomousSessionState::PrCreation)
        .unwrap();
    session1
        .transition_to(AutonomousSessionState::PrMonitoring)
        .unwrap();
    session1
        .transition_to(AutonomousSessionState::PrMerging)
        .unwrap();
    session1
        .transition_to(AutonomousSessionState::Completing)
        .unwrap();
    session1.complete().unwrap();
    db.create_autonomous_session(&session1).await.unwrap();

    let mut session2 = AutonomousSession::with_id("session-paused");
    session2.start().unwrap();
    session2.pause("break").unwrap();
    db.create_autonomous_session(&session2).await.unwrap();

    // No active session yet (done + paused)
    let active = db.get_active_autonomous_session().await.unwrap();
    assert!(active.is_none());

    // Create an active session
    let mut session3 = AutonomousSession::with_id("session-active");
    session3.start().unwrap();
    db.create_autonomous_session(&session3).await.unwrap();

    let active = db.get_active_autonomous_session().await.unwrap();
    assert!(active.is_some());
    assert_eq!(active.unwrap().id, "session-active");
}

#[tokio::test]
async fn test_delete_autonomous_session() {
    let db = Database::in_memory().await.unwrap();

    let session = AutonomousSession::with_id("session-to-delete");
    db.create_autonomous_session(&session).await.unwrap();

    // Verify it exists
    let exists = db
        .get_autonomous_session("session-to-delete")
        .await
        .unwrap();
    assert!(exists.is_some());

    // Delete it
    let deleted = db
        .delete_autonomous_session("session-to-delete")
        .await
        .unwrap();
    assert!(deleted);

    // Verify it's gone
    let gone = db
        .get_autonomous_session("session-to-delete")
        .await
        .unwrap();
    assert!(gone.is_none());

    // Try to delete non-existent
    let not_deleted = db.delete_autonomous_session("not-exists").await.unwrap();
    assert!(!not_deleted);
}

#[tokio::test]
async fn test_session_state_history() {
    let db = Database::in_memory().await.unwrap();

    let session = AutonomousSession::with_id("session-history");
    db.create_autonomous_session(&session).await.unwrap();

    // Record state transitions
    db.record_session_state_transition(
        "session-history",
        AutonomousSessionState::Idle,
        AutonomousSessionState::Analyzing,
        Some("Started processing"),
        None,
    )
    .await
    .unwrap();

    db.record_session_state_transition(
        "session-history",
        AutonomousSessionState::Analyzing,
        AutonomousSessionState::Discovering,
        Some("Analysis complete"),
        Some(serde_json::json!({"files_scanned": 150})),
    )
    .await
    .unwrap();

    db.record_session_state_transition(
        "session-history",
        AutonomousSessionState::Discovering,
        AutonomousSessionState::Blocked,
        Some("Epic file not found"),
        None,
    )
    .await
    .unwrap();

    // Get history
    let history = db
        .get_session_state_history("session-history")
        .await
        .unwrap();
    assert_eq!(history.len(), 3);

    assert_eq!(history[0].from_state, AutonomousSessionState::Idle);
    assert_eq!(history[0].to_state, AutonomousSessionState::Analyzing);
    assert_eq!(history[0].reason, Some("Started processing".to_string()));

    assert_eq!(history[1].from_state, AutonomousSessionState::Analyzing);
    assert_eq!(history[1].to_state, AutonomousSessionState::Discovering);
    assert!(history[1].metadata.get("files_scanned").is_some());

    assert_eq!(history[2].from_state, AutonomousSessionState::Discovering);
    assert_eq!(history[2].to_state, AutonomousSessionState::Blocked);
}

#[tokio::test]
async fn test_get_sessions_by_state() {
    let db = Database::in_memory().await.unwrap();

    // Create sessions with different states
    let session1 = AutonomousSession::with_id("idle-1");
    db.create_autonomous_session(&session1).await.unwrap();

    let session2 = AutonomousSession::with_id("idle-2");
    db.create_autonomous_session(&session2).await.unwrap();

    let mut session3 = AutonomousSession::with_id("analyzing-1");
    session3.start().unwrap();
    db.create_autonomous_session(&session3).await.unwrap();

    let idle_sessions = db
        .get_sessions_by_state(AutonomousSessionState::Idle)
        .await
        .unwrap();
    assert_eq!(idle_sessions.len(), 2);

    let analyzing_sessions = db
        .get_sessions_by_state(AutonomousSessionState::Analyzing)
        .await
        .unwrap();
    assert_eq!(analyzing_sessions.len(), 1);

    let done_sessions = db
        .get_sessions_by_state(AutonomousSessionState::Done)
        .await
        .unwrap();
    assert_eq!(done_sessions.len(), 0);
}

#[tokio::test]
async fn test_count_sessions_by_state() {
    let db = Database::in_memory().await.unwrap();

    // Create sessions with different states
    let session1 = AutonomousSession::with_id("idle-1");
    db.create_autonomous_session(&session1).await.unwrap();

    let session2 = AutonomousSession::with_id("idle-2");
    db.create_autonomous_session(&session2).await.unwrap();

    let mut session3 = AutonomousSession::with_id("analyzing-1");
    session3.start().unwrap();
    db.create_autonomous_session(&session3).await.unwrap();

    let mut session4 = AutonomousSession::with_id("blocked-1");
    session4.start().unwrap();
    session4.block("error").unwrap();
    db.create_autonomous_session(&session4).await.unwrap();

    let counts = db.count_sessions_by_state().await.unwrap();
    assert_eq!(counts.get("idle"), Some(&2));
    assert_eq!(counts.get("analyzing"), Some(&1));
    assert_eq!(counts.get("blocked"), Some(&1));
    assert_eq!(counts.get("done"), None); // No done sessions
}

#[tokio::test]
async fn test_session_not_found() {
    let db = Database::in_memory().await.unwrap();

    let result = db.get_autonomous_session("not-exists").await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_full_session_lifecycle_in_db() {
    let db = Database::in_memory().await.unwrap();

    // Create session
    let mut session = AutonomousSession::with_id("lifecycle-test").with_config(SessionConfig {
        max_agents: 3,
        epic_pattern: Some("epic-016-*".to_string()),
        ..Default::default()
    });

    db.create_autonomous_session(&session).await.unwrap();

    // Start processing
    session.start().unwrap();
    db.update_autonomous_session(&session).await.unwrap();
    db.record_session_state_transition(
        &session.id,
        AutonomousSessionState::Idle,
        AutonomousSessionState::Analyzing,
        Some("Started"),
        None,
    )
    .await
    .unwrap();

    // Add work
    session.add_work_item(WorkItem {
        id: "story-1".to_string(),
        work_type: WorkItemType::Story,
        epic_id: "epic-016".to_string(),
        story_id: Some("story-1".to_string()),
        priority: 1,
        dependencies: vec![],
        metadata: serde_json::Value::Null,
    });

    session
        .transition_to(AutonomousSessionState::Discovering)
        .unwrap();
    session
        .transition_to(AutonomousSessionState::Planning)
        .unwrap();
    session
        .transition_to(AutonomousSessionState::Executing)
        .unwrap();

    session.set_current_epic("epic-016");
    session.set_current_story("story-1");
    session.metrics.record_agent_spawned();

    db.update_autonomous_session(&session).await.unwrap();

    // Complete work
    let work = session.pop_work_item().unwrap();
    session.record_completed(CompletedItem {
        id: work.id,
        work_type: work.work_type,
        epic_id: work.epic_id,
        story_id: work.story_id,
        success: true,
        completed_at: Utc::now(),
        error: None,
        agent_id: Some("agent-1".to_string()),
    });

    session
        .transition_to(AutonomousSessionState::Reviewing)
        .unwrap();
    session.metrics.record_review_passed();

    session
        .transition_to(AutonomousSessionState::PrCreation)
        .unwrap();
    session
        .transition_to(AutonomousSessionState::PrMonitoring)
        .unwrap();
    session
        .transition_to(AutonomousSessionState::PrMerging)
        .unwrap();
    session
        .transition_to(AutonomousSessionState::Completing)
        .unwrap();
    session.complete().unwrap();

    db.update_autonomous_session(&session).await.unwrap();

    // Verify final state
    let final_session = db
        .get_autonomous_session("lifecycle-test")
        .await
        .unwrap()
        .unwrap();

    assert_eq!(final_session.state, AutonomousSessionState::Done);
    assert!(final_session.completed_at.is_some());
    assert_eq!(final_session.metrics.stories_completed, 1);
    assert_eq!(final_session.metrics.reviews_passed, 1);
    assert_eq!(final_session.metrics.agents_spawned, 1);
    assert_eq!(final_session.completed_items.len(), 1);
    assert!(final_session.work_queue.is_empty());
}
