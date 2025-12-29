//! Database tests for webhook event operations

#[cfg(test)]
mod tests {
    use crate::{Database, WebhookEvent, WebhookEventStatus};

    #[tokio::test]
    async fn test_insert_webhook_event() {
        let db = Database::in_memory().await.unwrap();

        let event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            r#"{"action":"opened"}"#.to_string(),
        );

        let id = db.insert_webhook_event(&event).await.unwrap();
        assert!(id > 0);
    }

    #[tokio::test]
    async fn test_insert_webhook_event_idempotency() {
        let db = Database::in_memory().await.unwrap();

        let event = WebhookEvent::new(
            "delivery-456".to_string(),
            "pull_request".to_string(),
            r#"{"action":"opened"}"#.to_string(),
        );

        // Insert same event twice
        let id1 = db.insert_webhook_event(&event).await.unwrap();
        let id2 = db.insert_webhook_event(&event).await.unwrap();

        // Should return same ID (idempotent)
        assert_eq!(id1, id2);
    }

    #[tokio::test]
    async fn test_get_webhook_event() {
        let db = Database::in_memory().await.unwrap();

        let event = WebhookEvent::new(
            "delivery-789".to_string(),
            "check_run".to_string(),
            r#"{"conclusion":"failure"}"#.to_string(),
        );

        let id = db.insert_webhook_event(&event).await.unwrap();
        let retrieved = db.get_webhook_event(id).await.unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.delivery_id, "delivery-789");
        assert_eq!(retrieved.event_type, "check_run");
        assert_eq!(retrieved.status, WebhookEventStatus::Pending);
    }

    #[tokio::test]
    async fn test_get_webhook_event_by_delivery_id() {
        let db = Database::in_memory().await.unwrap();

        let event = WebhookEvent::new(
            "delivery-abc".to_string(),
            "push".to_string(),
            r#"{"ref":"refs/heads/main"}"#.to_string(),
        );

        db.insert_webhook_event(&event).await.unwrap();
        let retrieved = db
            .get_webhook_event_by_delivery_id("delivery-abc")
            .await
            .unwrap();

        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.delivery_id, "delivery-abc");
        assert_eq!(retrieved.event_type, "push");
    }

    #[tokio::test]
    async fn test_update_webhook_event() {
        let db = Database::in_memory().await.unwrap();

        let event = WebhookEvent::new(
            "delivery-update".to_string(),
            "pull_request".to_string(),
            r#"{"action":"opened"}"#.to_string(),
        );

        let id = db.insert_webhook_event(&event).await.unwrap();
        let mut event = db.get_webhook_event(id).await.unwrap().unwrap();

        // Mark as processing
        event.mark_processing();
        db.update_webhook_event(&event).await.unwrap();

        let updated = db.get_webhook_event(id).await.unwrap().unwrap();
        assert_eq!(updated.status, WebhookEventStatus::Processing);
    }

    #[tokio::test]
    async fn test_get_pending_webhook_events() {
        let db = Database::in_memory().await.unwrap();

        // Insert multiple events
        for i in 1..=5 {
            let event = WebhookEvent::new(
                format!("delivery-{}", i),
                "pull_request".to_string(),
                r#"{"action":"opened"}"#.to_string(),
            );
            db.insert_webhook_event(&event).await.unwrap();
        }

        let pending = db.get_pending_webhook_events(10).await.unwrap();
        assert_eq!(pending.len(), 5);
        assert!(pending.iter().all(|e| e.status == WebhookEventStatus::Pending));
    }

    #[tokio::test]
    async fn test_get_pending_webhook_events_respects_retry_time() {
        let db = Database::in_memory().await.unwrap();

        // Create event with future retry time
        let mut event = WebhookEvent::new(
            "delivery-future".to_string(),
            "pull_request".to_string(),
            r#"{"action":"opened"}"#.to_string(),
        );
        event.next_retry_at = Some(chrono::Utc::now() + chrono::Duration::hours(1));

        let id = db.insert_webhook_event(&event).await.unwrap();

        // Should not appear in pending events
        let pending = db.get_pending_webhook_events(10).await.unwrap();
        assert_eq!(pending.len(), 0);

        // Update to have past retry time
        let mut event = db.get_webhook_event(id).await.unwrap().unwrap();
        event.next_retry_at = Some(chrono::Utc::now() - chrono::Duration::seconds(1));
        db.update_webhook_event(&event).await.unwrap();

        // Now should appear
        let pending = db.get_pending_webhook_events(10).await.unwrap();
        assert_eq!(pending.len(), 1);
    }

    #[tokio::test]
    async fn test_webhook_event_retry_flow() {
        let db = Database::in_memory().await.unwrap();

        let event = WebhookEvent::new(
            "delivery-retry".to_string(),
            "pull_request".to_string(),
            r#"{"action":"opened"}"#.to_string(),
        );

        let id = db.insert_webhook_event(&event).await.unwrap();
        let mut event = db.get_webhook_event(id).await.unwrap().unwrap();

        // Simulate processing failure
        event.mark_processing();
        db.update_webhook_event(&event).await.unwrap();

        event.mark_failed("test error".to_string());
        db.update_webhook_event(&event).await.unwrap();

        let updated = db.get_webhook_event(id).await.unwrap().unwrap();
        assert_eq!(updated.status, WebhookEventStatus::Pending);
        assert_eq!(updated.retry_count, 1);
        assert!(updated.next_retry_at.is_some());
        assert_eq!(updated.error_message, Some("test error".to_string()));
    }

    #[tokio::test]
    async fn test_webhook_event_dead_letter() {
        let db = Database::in_memory().await.unwrap();

        let event = WebhookEvent::new(
            "delivery-dead".to_string(),
            "pull_request".to_string(),
            r#"{"action":"opened"}"#.to_string(),
        );

        let id = db.insert_webhook_event(&event).await.unwrap();
        let mut event = db.get_webhook_event(id).await.unwrap().unwrap();

        // Exhaust retries
        for i in 1..=4 {
            event.mark_failed(format!("error {}", i));
            db.update_webhook_event(&event).await.unwrap();
        }

        let updated = db.get_webhook_event(id).await.unwrap().unwrap();
        assert_eq!(updated.status, WebhookEventStatus::DeadLetter);
        assert_eq!(updated.retry_count, 4);
        assert!(updated.next_retry_at.is_none());
    }

    #[tokio::test]
    async fn test_get_webhook_events_by_status() {
        let db = Database::in_memory().await.unwrap();

        // Create events with different statuses
        let event1 = WebhookEvent::new(
            "delivery-1".to_string(),
            "pull_request".to_string(),
            "{}".to_string(),
        );
        db.insert_webhook_event(&event1).await.unwrap();

        let mut event2 = WebhookEvent::new(
            "delivery-2".to_string(),
            "pull_request".to_string(),
            "{}".to_string(),
        );
        let id2 = db.insert_webhook_event(&event2).await.unwrap();
        event2 = db.get_webhook_event(id2).await.unwrap().unwrap();
        event2.mark_completed();
        db.update_webhook_event(&event2).await.unwrap();

        let pending = db
            .get_webhook_events_by_status(WebhookEventStatus::Pending, 10)
            .await
            .unwrap();
        assert_eq!(pending.len(), 1);

        let completed = db
            .get_webhook_events_by_status(WebhookEventStatus::Completed, 10)
            .await
            .unwrap();
        assert_eq!(completed.len(), 1);
    }

    #[tokio::test]
    async fn test_count_webhook_events_by_status() {
        let db = Database::in_memory().await.unwrap();

        // Create multiple pending events
        for i in 1..=3 {
            let event = WebhookEvent::new(
                format!("delivery-count-{}", i),
                "pull_request".to_string(),
                "{}".to_string(),
            );
            db.insert_webhook_event(&event).await.unwrap();
        }

        let count = db
            .count_webhook_events_by_status(WebhookEventStatus::Pending)
            .await
            .unwrap();
        assert_eq!(count, 3);
    }

    #[tokio::test]
    async fn test_delete_old_webhook_events() {
        let db = Database::in_memory().await.unwrap();

        // Create a completed event
        let mut event = WebhookEvent::new(
            "delivery-old".to_string(),
            "pull_request".to_string(),
            "{}".to_string(),
        );
        let id = db.insert_webhook_event(&event).await.unwrap();
        event = db.get_webhook_event(id).await.unwrap().unwrap();
        event.mark_completed();
        db.update_webhook_event(&event).await.unwrap();

        // Manually update received_at to be in the past (SQLite doesn't have easy date math in tests)
        sqlx::query("UPDATE webhook_events SET received_at = datetime('now', '-8 days') WHERE id = ?")
            .bind(id)
            .execute(&db.pool)
            .await
            .unwrap();

        // Delete events older than 7 days
        let deleted = db.delete_old_webhook_events(7).await.unwrap();
        assert_eq!(deleted, 1);

        // Verify it's gone
        let retrieved = db.get_webhook_event(id).await.unwrap();
        assert!(retrieved.is_none());
    }
}
