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

        // Process the event with the appropriate handler
        match self.handle_event(&event).await {
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

    /// Handle event processing
    async fn handle_event(&self, event: &WebhookEvent) -> orchestrate_core::Result<()> {
        match event.event_type.as_str() {
            "pull_request" => {
                crate::event_handlers::handle_pr_opened(self.database.clone(), event).await
            }
            "pull_request_review" => {
                crate::event_handlers::handle_pr_review_submitted(self.database.clone(), event).await
            }
            "check_run" | "check_suite" => {
                crate::event_handlers::handle_ci_status(self.database.clone(), event).await
            }
            "push" => {
                crate::event_handlers::handle_push_to_main(self.database.clone(), event).await
            }
            "issues" => {
                crate::event_handlers::handle_issue_opened(self.database.clone(), event).await
            }
            _ => {
                // Unknown event type - not an error, just skip
                debug!(event_type = %event.event_type, "No handler for event type");
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_processor_processes_events() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Insert test events with a valid PR opened payload
        for i in 1..=3 {
            let payload = serde_json::json!({
                "action": "opened",
                "number": i,
                "pull_request": {
                    "number": i,
                    "head": {
                        "ref": format!("feature/test-{}", i),
                        "repo": {
                            "fork": false
                        }
                    }
                },
                "repository": {
                    "full_name": "owner/repo"
                }
            }).to_string();

            let event = WebhookEvent::new(
                format!("delivery-{}", i),
                "pull_request".to_string(),
                payload,
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

        // Verify agents were created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 3);
    }

    #[tokio::test]
    async fn test_processor_respects_batch_size() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Insert more events than batch size
        for i in 1..=15 {
            let payload = serde_json::json!({
                "action": "opened",
                "number": i,
                "pull_request": {
                    "number": i,
                    "head": {
                        "ref": format!("feature/batch-{}", i),
                        "repo": {
                            "fork": false
                        }
                    }
                },
                "repository": {
                    "full_name": "owner/repo"
                }
            }).to_string();

            let event = WebhookEvent::new(
                format!("delivery-batch-{}", i),
                "pull_request".to_string(),
                payload,
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

        // Verify all agents were created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 15);
    }

    #[tokio::test]
    async fn test_processor_handles_empty_queue() {
        let database = Arc::new(Database::in_memory().await.unwrap());
        let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default());

        // Should not error on empty queue
        processor.process_batch().await.unwrap();
    }

    #[tokio::test]
    async fn test_processor_handles_push_events() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Insert push event to main branch
        let payload = serde_json::json!({
            "ref": "refs/heads/main",
            "before": "abc123",
            "after": "def456",
            "repository": {
                "full_name": "owner/repo"
            },
            "commits": [
                {
                    "added": ["file1.rs"],
                    "modified": ["file2.rs"],
                    "removed": []
                }
            ]
        }).to_string();

        let event = WebhookEvent::new(
            "delivery-push-test".to_string(),
            "push".to_string(),
            payload,
        );
        database.insert_webhook_event(&event).await.unwrap();

        // Process event
        let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default());
        processor.process_batch().await.unwrap();

        // Event should be completed
        let completed = database
            .get_webhook_events_by_status(WebhookEventStatus::Completed, 10)
            .await
            .unwrap();
        assert_eq!(completed.len(), 1);

        // Verify regression-tester agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_type, orchestrate_core::AgentType::RegressionTester);
    }

    #[tokio::test]
    async fn test_processor_skips_push_to_feature_branch() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Insert push event to feature branch
        let payload = serde_json::json!({
            "ref": "refs/heads/feature/test",
            "before": "abc123",
            "after": "def456",
            "repository": {
                "full_name": "owner/repo"
            },
            "commits": [
                {
                    "added": ["file1.rs"],
                    "modified": [],
                    "removed": []
                }
            ]
        }).to_string();

        let event = WebhookEvent::new(
            "delivery-push-feature".to_string(),
            "push".to_string(),
            payload,
        );
        database.insert_webhook_event(&event).await.unwrap();

        // Process event
        let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default());
        processor.process_batch().await.unwrap();

        // Event should be completed (successfully skipped)
        let completed = database
            .get_webhook_events_by_status(WebhookEventStatus::Completed, 10)
            .await
            .unwrap();
        assert_eq!(completed.len(), 1);

        // No agent should be created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_processor_handles_issue_opened_events() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Insert issue opened event
        let payload = serde_json::json!({
            "action": "opened",
            "issue": {
                "number": 999,
                "title": "Test issue",
                "body": "This is a test issue",
                "state": "open",
            },
            "repository": {
                "full_name": "owner/repo"
            }
        }).to_string();

        let event = WebhookEvent::new(
            "delivery-issue-test".to_string(),
            "issues".to_string(),
            payload,
        );
        database.insert_webhook_event(&event).await.unwrap();

        // Process event
        let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default());
        processor.process_batch().await.unwrap();

        // Event should be completed
        let completed = database
            .get_webhook_events_by_status(WebhookEventStatus::Completed, 10)
            .await
            .unwrap();
        assert_eq!(completed.len(), 1);

        // Verify issue-triager agent was created
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].agent_type, orchestrate_core::AgentType::IssueTriager);
    }

    #[tokio::test]
    async fn test_processor_skips_issue_closed() {
        let database = Arc::new(Database::in_memory().await.unwrap());

        // Insert issue closed event
        let payload = serde_json::json!({
            "action": "closed",
            "issue": {
                "number": 1000,
                "title": "Closed issue",
                "body": "This issue was closed",
                "state": "closed",
            },
            "repository": {
                "full_name": "owner/repo"
            }
        }).to_string();

        let event = WebhookEvent::new(
            "delivery-issue-closed".to_string(),
            "issues".to_string(),
            payload,
        );
        database.insert_webhook_event(&event).await.unwrap();

        // Process event
        let processor = WebhookProcessor::new(database.clone(), WebhookProcessorConfig::default());
        processor.process_batch().await.unwrap();

        // Event should be completed (successfully skipped)
        let completed = database
            .get_webhook_events_by_status(WebhookEventStatus::Completed, 10)
            .await
            .unwrap();
        assert_eq!(completed.len(), 1);

        // No agent should be created for closed issue
        let agents = database.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }
}
