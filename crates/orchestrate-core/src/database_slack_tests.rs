//! Database tests for Slack integration

use crate::approval::{ApprovalRequest as CoreApprovalRequest, ApprovalStatus};
use crate::slack::{ApprovalDecision, PrThread, SlackApprovalRequest, SlackConnection};
use crate::Database;
use sqlx::Row;

async fn setup_test_connection(db: &Database) -> String {
    let conn = SlackConnection::new("T123", "Test Team", "xoxb-test-token");

    sqlx::query(
        r#"
        INSERT INTO slack_connections (
            id, team_id, team_name, bot_token, bot_user_id, app_id,
            connected_at, connected_by, is_active, scopes
        )
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
    )
    .bind(&conn.id)
    .bind(&conn.team_id)
    .bind(&conn.team_name)
    .bind(&conn.bot_token)
    .bind("U123")
    .bind("A123")
    .bind(conn.connected_at.to_rfc3339())
    .bind("admin")
    .bind(1)
    .bind("[]")
    .execute(db.pool())
    .await
    .unwrap();

    conn.id
}

async fn setup_test_approval(db: &Database) -> i64 {
    // Insert a simple approval request directly
    sqlx::query(
        r#"
        INSERT INTO approval_requests (
            stage_id, run_id, status, required_approvers, required_count,
            approval_count, rejection_count, timeout_seconds, timeout_action,
            timeout_at, resolved_at, created_at
        )
        VALUES (1, 1, 'pending', 'user1@example.com', 1, 0, 0, NULL, NULL, NULL, NULL, datetime('now'))
        "#,
    )
    .execute(db.pool())
    .await
    .unwrap();

    // Get the last inserted row ID
    let row = sqlx::query("SELECT last_insert_rowid() as id")
        .fetch_one(db.pool())
        .await
        .unwrap();

    row.get::<i64, _>("id")
}

#[tokio::test]
async fn test_insert_and_get_slack_approval_request() {
    let db = Database::in_memory().await.unwrap();
    let conn_id = setup_test_connection(&db).await;
    let approval_id = setup_test_approval(&db).await;

    let mut request = SlackApprovalRequest::new(
        &approval_id.to_string(),
        "deployment",
        "production",
        "Deploy version 1.2.0 to production",
    );
    request.channel_id = "C123".to_string();
    request.message_ts = "1234.5678".to_string();
    request.requester_slack_id = "U999".to_string();

    // Insert
    db.insert_slack_approval_request(&request, &conn_id)
        .await
        .unwrap();

    // Get
    let retrieved = db
        .get_slack_approval_request(&request.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.id, request.id);
    assert_eq!(retrieved.approval_id, request.approval_id);
    assert_eq!(retrieved.resource_type, request.resource_type);
    assert_eq!(retrieved.resource_id, request.resource_id);
    assert_eq!(retrieved.description, request.description);
    assert!(retrieved.decision.is_none());
}

#[tokio::test]
async fn test_update_slack_approval_request_response() {
    let db = Database::in_memory().await.unwrap();
    let conn_id = setup_test_connection(&db).await;
    let approval_id = setup_test_approval(&db).await;

    let mut request = SlackApprovalRequest::new(
        &approval_id.to_string(),
        "deployment",
        "production",
        "Deploy version 1.2.0 to production",
    );
    request.channel_id = "C123".to_string();
    request.message_ts = "1234.5678".to_string();
    request.requester_slack_id = "U999".to_string();

    db.insert_slack_approval_request(&request, &conn_id)
        .await
        .unwrap();

    // Update with response
    db.update_slack_approval_request_response(
        &request.id,
        "U456",
        &ApprovalDecision::Approved,
        Some("LGTM"),
    )
    .await
    .unwrap();

    // Verify
    let updated = db
        .get_slack_approval_request(&request.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(updated.responder_slack_id, Some("U456".to_string()));
    assert_eq!(updated.decision, Some(ApprovalDecision::Approved));
    assert!(updated.responded_at.is_some());
}

#[tokio::test]
async fn test_upsert_pr_thread() {
    let db = Database::in_memory().await.unwrap();
    let conn_id = setup_test_connection(&db).await;

    let thread = PrThread::new(123, "C123", "1234.5678");

    // Insert
    db.upsert_pr_thread(&thread, &conn_id).await.unwrap();

    // Get
    let retrieved = db
        .get_pr_thread(123, &conn_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.pr_number, 123);
    assert_eq!(retrieved.channel_id, "C123");
    assert_eq!(retrieved.thread_ts, "1234.5678");
    assert!(!retrieved.is_archived);

    // Update
    let mut updated_thread = thread.clone();
    updated_thread.thread_ts = "9876.5432".to_string();

    db.upsert_pr_thread(&updated_thread, &conn_id)
        .await
        .unwrap();

    let retrieved = db
        .get_pr_thread(123, &conn_id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.thread_ts, "9876.5432");
}

#[tokio::test]
async fn test_archive_pr_thread() {
    let db = Database::in_memory().await.unwrap();
    let conn_id = setup_test_connection(&db).await;

    let thread = PrThread::new(123, "C123", "1234.5678");

    db.upsert_pr_thread(&thread, &conn_id).await.unwrap();

    // Archive
    db.archive_pr_thread(123, &conn_id).await.unwrap();

    // Verify
    let retrieved = db
        .get_pr_thread(123, &conn_id)
        .await
        .unwrap()
        .unwrap();

    assert!(retrieved.is_archived);
}

#[tokio::test]
async fn test_insert_slack_command_audit() {
    let db = Database::in_memory().await.unwrap();
    let conn_id = setup_test_connection(&db).await;

    db.insert_slack_command_audit(
        &conn_id,
        "/orchestrate",
        "U123",
        "testuser",
        "C123",
        Some("status"),
        "ephemeral",
        true,
        None,
    )
    .await
    .unwrap();

    // Get recent
    let audits = db
        .get_recent_slack_command_audit(&conn_id, 10)
        .await
        .unwrap();

    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0].command, "/orchestrate");
    assert_eq!(audits[0].user_id, "U123");
    assert_eq!(audits[0].user_name, "testuser");
    assert_eq!(audits[0].text, Some("status".to_string()));
    assert!(audits[0].success);
}

#[tokio::test]
async fn test_get_recent_slack_command_audit_limit() {
    let db = Database::in_memory().await.unwrap();
    let conn_id = setup_test_connection(&db).await;

    // Insert 5 audit entries
    for i in 0..5 {
        db.insert_slack_command_audit(
            &conn_id,
            "/orchestrate",
            "U123",
            "testuser",
            "C123",
            Some(&format!("command{}", i)),
            "ephemeral",
            true,
            None,
        )
        .await
        .unwrap();
    }

    // Get only 3
    let audits = db
        .get_recent_slack_command_audit(&conn_id, 3)
        .await
        .unwrap();

    assert_eq!(audits.len(), 3);
}
