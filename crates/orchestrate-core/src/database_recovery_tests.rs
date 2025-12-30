//! Database tests for recovery attempt operations

use crate::recovery::{RecoveryActionType, RecoveryAttempt, RecoveryOutcome};
use crate::Database;

#[tokio::test]
async fn test_create_and_get_recovery_attempt() {
    let db = Database::in_memory().await.unwrap();

    let attempt = RecoveryAttempt::new("agent-123", RecoveryActionType::Retry)
        .with_details(serde_json::json!({"reason": "test"}));

    let id = db.create_recovery_attempt(&attempt).await.unwrap();
    assert!(id > 0);

    let retrieved = db.get_recovery_attempt(id).await.unwrap();
    assert!(retrieved.is_some());

    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.agent_id, "agent-123");
    assert_eq!(retrieved.action_type, RecoveryActionType::Retry);
    assert_eq!(retrieved.outcome, RecoveryOutcome::InProgress);
}

#[tokio::test]
async fn test_recovery_attempt_with_session() {
    let db = Database::in_memory().await.unwrap();

    let attempt = RecoveryAttempt::new("agent-456", RecoveryActionType::ModelEscalation)
        .with_session("session-abc");

    let id = db.create_recovery_attempt(&attempt).await.unwrap();

    let retrieved = db.get_recovery_attempt(id).await.unwrap().unwrap();
    assert_eq!(retrieved.session_id, Some("session-abc".to_string()));
}

#[tokio::test]
async fn test_recovery_attempt_with_detection_id() {
    let db = Database::in_memory().await.unwrap();

    // First create a stuck detection
    let detection = crate::stuck_detection::StuckDetection::new(
        "agent-789",
        crate::stuck_detection::StuckType::TurnLimit,
        crate::stuck_detection::StuckSeverity::High,
    );
    let detection_id = db.create_stuck_detection(&detection).await.unwrap();

    // Create recovery attempt with detection reference
    let attempt = RecoveryAttempt::new("agent-789", RecoveryActionType::ModelEscalation)
        .with_detection(detection_id);

    let id = db.create_recovery_attempt(&attempt).await.unwrap();

    let retrieved = db.get_recovery_attempt(id).await.unwrap().unwrap();
    assert_eq!(retrieved.stuck_detection_id, Some(detection_id));
}

#[tokio::test]
async fn test_update_recovery_attempt_success() {
    let db = Database::in_memory().await.unwrap();

    let mut attempt = RecoveryAttempt::new("agent-update", RecoveryActionType::Retry);
    let id = db.create_recovery_attempt(&attempt).await.unwrap();
    attempt.id = id;

    // Mark as successful
    attempt.succeed();
    db.update_recovery_attempt(&attempt).await.unwrap();

    let retrieved = db.get_recovery_attempt(id).await.unwrap().unwrap();
    assert_eq!(retrieved.outcome, RecoveryOutcome::Success);
    assert!(retrieved.completed_at.is_some());
}

#[tokio::test]
async fn test_update_recovery_attempt_failure() {
    let db = Database::in_memory().await.unwrap();

    let mut attempt = RecoveryAttempt::new("agent-fail", RecoveryActionType::SpawnFixer);
    let id = db.create_recovery_attempt(&attempt).await.unwrap();
    attempt.id = id;

    // Mark as failed
    attempt.fail("Fixer agent also got stuck");
    db.update_recovery_attempt(&attempt).await.unwrap();

    let retrieved = db.get_recovery_attempt(id).await.unwrap().unwrap();
    assert_eq!(retrieved.outcome, RecoveryOutcome::Failed);
    assert_eq!(
        retrieved.error_message,
        Some("Fixer agent also got stuck".to_string())
    );
}

#[tokio::test]
async fn test_get_recovery_attempts_for_agent() {
    let db = Database::in_memory().await.unwrap();

    // Create multiple attempts for same agent
    let attempt1 = RecoveryAttempt::new("agent-multi", RecoveryActionType::Retry);
    db.create_recovery_attempt(&attempt1).await.unwrap();

    let attempt2 = RecoveryAttempt::new("agent-multi", RecoveryActionType::ModelEscalation);
    db.create_recovery_attempt(&attempt2).await.unwrap();

    // Create one for different agent
    let attempt3 = RecoveryAttempt::new("agent-other", RecoveryActionType::Retry);
    db.create_recovery_attempt(&attempt3).await.unwrap();

    let attempts = db.get_recovery_attempts_for_agent("agent-multi").await.unwrap();
    assert_eq!(attempts.len(), 2);
}

#[tokio::test]
async fn test_get_in_progress_recovery_attempts() {
    let db = Database::in_memory().await.unwrap();

    // Create in-progress attempt
    let attempt1 = RecoveryAttempt::new("agent-progress", RecoveryActionType::Retry);
    db.create_recovery_attempt(&attempt1).await.unwrap();

    // Create completed attempt
    let mut attempt2 = RecoveryAttempt::new("agent-progress", RecoveryActionType::ModelEscalation);
    attempt2.succeed();
    db.create_recovery_attempt(&attempt2).await.unwrap();

    // Should only get in-progress ones
    let in_progress = db
        .get_in_progress_recovery_attempts("agent-progress")
        .await
        .unwrap();
    assert_eq!(in_progress.len(), 1);
    assert_eq!(in_progress[0].action_type, RecoveryActionType::Retry);
}

#[tokio::test]
async fn test_count_recovery_attempts_by_type() {
    let db = Database::in_memory().await.unwrap();

    // Create attempts of various types
    let attempt1 = RecoveryAttempt::new("agent-count", RecoveryActionType::Retry);
    db.create_recovery_attempt(&attempt1).await.unwrap();

    let attempt2 = RecoveryAttempt::new("agent-count", RecoveryActionType::Retry);
    db.create_recovery_attempt(&attempt2).await.unwrap();

    let attempt3 = RecoveryAttempt::new("agent-count", RecoveryActionType::ModelEscalation);
    db.create_recovery_attempt(&attempt3).await.unwrap();

    let counts = db
        .count_recovery_attempts_by_type("agent-count")
        .await
        .unwrap();
    assert_eq!(counts.get("retry"), Some(&2));
    assert_eq!(counts.get("model_escalation"), Some(&1));
}

#[tokio::test]
async fn test_get_recovery_stats() {
    let db = Database::in_memory().await.unwrap();

    // Create attempts with various outcomes
    let mut attempt1 = RecoveryAttempt::new("agent-stats", RecoveryActionType::Retry);
    attempt1.succeed();
    db.create_recovery_attempt(&attempt1).await.unwrap();

    let mut attempt2 = RecoveryAttempt::new("agent-stats", RecoveryActionType::Retry);
    attempt2.succeed();
    db.create_recovery_attempt(&attempt2).await.unwrap();

    let mut attempt3 = RecoveryAttempt::new("agent-stats", RecoveryActionType::ModelEscalation);
    attempt3.fail("Failed to escalate");
    db.create_recovery_attempt(&attempt3).await.unwrap();

    let attempt4 = RecoveryAttempt::new("agent-stats", RecoveryActionType::Wait);
    db.create_recovery_attempt(&attempt4).await.unwrap();

    let stats = db.get_recovery_stats("agent-stats").await.unwrap();
    assert_eq!(stats.total_attempts, 4);
    assert_eq!(stats.successful, 2);
    assert_eq!(stats.failed, 1);
    assert!((stats.success_rate - 0.5).abs() < 0.01); // 2/4 = 0.5
}

#[tokio::test]
async fn test_get_recovery_stats_empty() {
    let db = Database::in_memory().await.unwrap();

    let stats = db.get_recovery_stats("agent-no-attempts").await.unwrap();
    assert_eq!(stats.total_attempts, 0);
    assert_eq!(stats.successful, 0);
    assert_eq!(stats.failed, 0);
    assert_eq!(stats.success_rate, 0.0);
}

#[tokio::test]
async fn test_recovery_attempt_all_action_types() {
    let db = Database::in_memory().await.unwrap();

    let types = vec![
        RecoveryActionType::PauseAndAlert,
        RecoveryActionType::ModelEscalation,
        RecoveryActionType::SpawnFixer,
        RecoveryActionType::FreshRetry,
        RecoveryActionType::EscalateToParent,
        RecoveryActionType::Retry,
        RecoveryActionType::Wait,
        RecoveryActionType::Abort,
    ];

    for action_type in types {
        let attempt = RecoveryAttempt::new("agent-types", action_type);
        let id = db.create_recovery_attempt(&attempt).await.unwrap();
        let retrieved = db.get_recovery_attempt(id).await.unwrap().unwrap();
        assert_eq!(retrieved.action_type, action_type);
    }
}

#[tokio::test]
async fn test_recovery_attempt_all_outcomes() {
    let db = Database::in_memory().await.unwrap();

    // InProgress (default)
    let attempt1 = RecoveryAttempt::new("agent-outcomes", RecoveryActionType::Retry);
    let id1 = db.create_recovery_attempt(&attempt1).await.unwrap();
    let retrieved1 = db.get_recovery_attempt(id1).await.unwrap().unwrap();
    assert_eq!(retrieved1.outcome, RecoveryOutcome::InProgress);

    // Success
    let mut attempt2 = RecoveryAttempt::new("agent-outcomes", RecoveryActionType::Retry);
    attempt2.succeed();
    let id2 = db.create_recovery_attempt(&attempt2).await.unwrap();
    let retrieved2 = db.get_recovery_attempt(id2).await.unwrap().unwrap();
    assert_eq!(retrieved2.outcome, RecoveryOutcome::Success);

    // Failed
    let mut attempt3 = RecoveryAttempt::new("agent-outcomes", RecoveryActionType::Retry);
    attempt3.fail("error");
    let id3 = db.create_recovery_attempt(&attempt3).await.unwrap();
    let retrieved3 = db.get_recovery_attempt(id3).await.unwrap().unwrap();
    assert_eq!(retrieved3.outcome, RecoveryOutcome::Failed);

    // Cancelled
    let mut attempt4 = RecoveryAttempt::new("agent-outcomes", RecoveryActionType::Retry);
    attempt4.cancel();
    let id4 = db.create_recovery_attempt(&attempt4).await.unwrap();
    let retrieved4 = db.get_recovery_attempt(id4).await.unwrap().unwrap();
    assert_eq!(retrieved4.outcome, RecoveryOutcome::Cancelled);

    // Skipped
    let mut attempt5 = RecoveryAttempt::new("agent-outcomes", RecoveryActionType::Retry);
    attempt5.skip("not applicable");
    let id5 = db.create_recovery_attempt(&attempt5).await.unwrap();
    let retrieved5 = db.get_recovery_attempt(id5).await.unwrap().unwrap();
    assert_eq!(retrieved5.outcome, RecoveryOutcome::Skipped);
}

#[tokio::test]
async fn test_recovery_attempt_with_attempt_number() {
    let db = Database::in_memory().await.unwrap();

    let attempt = RecoveryAttempt::new("agent-num", RecoveryActionType::Retry)
        .with_attempt_number(3);

    let id = db.create_recovery_attempt(&attempt).await.unwrap();

    let retrieved = db.get_recovery_attempt(id).await.unwrap().unwrap();
    assert_eq!(retrieved.attempt_number, 3);
}

#[tokio::test]
async fn test_recovery_attempt_not_found() {
    let db = Database::in_memory().await.unwrap();

    let result = db.get_recovery_attempt(99999).await.unwrap();
    assert!(result.is_none());
}
