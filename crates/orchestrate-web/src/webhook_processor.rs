//! Webhook event processor
//!
//! Polls the webhook_events queue and processes events asynchronously.

use orchestrate_core::{Database, WebhookEvent, WebhookEventStatus};
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{debug, error, info, warn};

/// Webhook event processor configuration
#[derive(Clone, Debug)]
pub struct WebhookProcessorConfig {
    /// Number of events to poll per batch
    pub batch_size: i64,
    /// Polling interval in seconds
    pub poll_interval_secs: u64,
    /// Maximum concurrent event processing
    pub max_concurrent: usize,
}

impl Default for WebhookProcessorConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            poll_interval_secs: 5,
            max_concurrent: 5,
        }
    }
}

/// Webhook event processor
pub struct WebhookProcessor {
    database: Arc<Database>,
    config: WebhookProcessorConfig,
}

impl WebhookProcessor {
    /// Create a new webhook processor
    pub fn new(database: Arc<Database>, config: WebhookProcessorConfig) -> Self {
        Self { database, config }
    }

    /// Run the processor loop (blocking)
    pub async fn run(&self) {
        info!(
            poll_interval_secs = self.config.poll_interval_secs,
            batch_size = self.config.batch_size,
            "Starting webhook event processor"
        );

        loop {
            if let Err(e) = self.process_batch().await {
                error!(error = %e, "Error processing webhook event batch");
            }

            sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }

    /// Process a batch of events
    async fn process_batch(&self) -> orchestrate_core::Result<()> {
        let events = self
            .database
            .get_pending_webhook_events(self.config.batch_size)
            .await?;

        if events.is_empty() {
            debug!("No pending webhook events to process");
            return Ok(());
        }

        info!(count = events.len(), "Processing webhook events");

        for event in events {
            if let Err(e) = self.process_event(event).await {
                error!(error = %e, "Failed to process webhook event");
            }
        }

        Ok(())
    }

    /// Process a single event
    async fn process_event(&self, mut event: WebhookEvent) -> orchestrate_core::Result<()> {
        let event_id = event.id.unwrap_or(0);

        info!(
            event_id = event_id,
            delivery_id = %event.delivery_id,
            event_type = %event.event_type,
            retry_count = event.retry_count,
            "Processing webhook event"
        );

        // Mark as processing
        event.mark_processing();
        self.database.update_webhook_event(&event).await?;

        // TODO: Actual event processing logic will be added in future stories
        // For now, just simulate processing and mark as completed
        match Self::handle_event(&event).await {
            Ok(()) => {
                event.mark_completed();
                self.database.update_webhook_event(&event).await?;
                info!(
                    event_id = event_id,
                    delivery_id = %event.delivery_id,
                    "Webhook event processed successfully"
                );
            }
            Err(e) => {
                warn!(
                    event_id = event_id,
                    delivery_id = %event.delivery_id,
                    error = %e,
                    retry_count = event.retry_count,
                    "Webhook event processing failed"
                );
                event.mark_failed(e.to_string());
                self.database.update_webhook_event(&event).await?;

                if event.status == WebhookEventStatus::DeadLetter {
                    error!(
                        event_id = event_id,
                        delivery_id = %event.delivery_id,
                        "Webhook event moved to dead letter queue after max retries"
                    );
                }
            }
        }

        Ok(())
    }

    /// Handle event processing (placeholder for actual logic)
    async fn handle_event(_event: &WebhookEvent) -> orchestrate_core::Result<()> {
        // TODO: This is a placeholder. Future stories will implement actual event handling:
        // - Story 3: PR opened -> spawn pr-shepherd
        // - Story 4: PR review -> spawn issue-fixer
        // - Story 5: CI failure -> spawn issue-fixer
        // - etc.

        // For now, just succeed
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_processor_processes_events() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Insert test events
        for i in 1..=3 {
            let event = WebhookEvent::new(
                format!("delivery-{}", i),
                "pull_request".to_string(),
                r#"{"action":"opened"}"#.to_string(),
            );
            database.insert_webhook_event(&event).await.unwrap();
        }

        // Process one batch
        let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default());
        processor.process_batch().await.unwrap();

        // All events should be completed
        let completed = database
            .get_webhook_events_by_status(WebhookEventStatus::Completed, 10)
            .await
            .unwrap();
        assert_eq!(completed.len(), 3);

        let pending = database
            .get_webhook_events_by_status(WebhookEventStatus::Pending, 10)
            .await
            .unwrap();
        assert_eq!(pending.len(), 0);
    }

    #[tokio::test]
    async fn test_processor_respects_batch_size() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Insert more events than batch size
        for i in 1..=15 {
            let event = WebhookEvent::new(
                format!("delivery-batch-{}", i),
                "pull_request".to_string(),
                r#"{"action":"opened"}"#.to_string(),
            );
            database.insert_webhook_event(&event).await.unwrap();
        }

        // Process with smaller batch size
        let config = WebhookProcessorConfig {
            batch_size: 5,
            ..Default::default()
        };
        let processor = WebhookProcessor::new(database.clone(), config);

        // First batch
        processor.process_batch().await.unwrap();
        let completed = database
            .count_webhook_events_by_status(WebhookEventStatus::Completed)
            .await
            .unwrap();
        assert_eq!(completed, 5);

        // Second batch
        processor.process_batch().await.unwrap();
        let completed = database
            .count_webhook_events_by_status(WebhookEventStatus::Completed)
            .await
            .unwrap();
        assert_eq!(completed, 10);

        // Third batch
        processor.process_batch().await.unwrap();
        let completed = database
            .count_webhook_events_by_status(WebhookEventStatus::Completed)
            .await
            .unwrap();
        assert_eq!(completed, 15);
    }

    #[tokio::test]
    async fn test_processor_handles_empty_queue() {
        let database = Arc::new(Database::in_memory().await.unwrap());
        let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default());

        // Should not error on empty queue
        processor.process_batch().await.unwrap();
    }
}
