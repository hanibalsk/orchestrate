//! Webhook event processor
//!
//! Polls the webhook_events queue and processes events asynchronously.

use orchestrate_core::{Database, WebhookConfig, WebhookEvent, WebhookEventStatus};
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
    webhook_config: Option<Arc<WebhookConfig>>,
}

impl WebhookProcessor {
    /// Create a new webhook processor
    pub fn new(database: Arc<Database>, config: WebhookProcessorConfig) -> Self {
        Self {
            database,
            config,
            webhook_config: None,
        }
    }

    /// Set webhook configuration
    pub fn with_config(mut self, webhook_config: WebhookConfig) -> Self {
        self.webhook_config = Some(Arc::new(webhook_config));
        self
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
    pub async fn process_batch(&self) -> orchestrate_core::Result<()> {
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
        // If config is set, check if event should be handled
        if let Some(config) = &self.webhook_config {
            // Build full event name with action
            let event_key = self.get_event_key(event);

            // Check if this event is configured
            if !config.should_handle_event(&event_key) {
                // Event not in config - skip silently
                debug!(
                    event_type = %event.event_type,
                    event_key = %event_key,
                    "Event not configured, skipping"
                );
                return Ok(());
            }

            // Check filter before processing
            if !self.should_process_event(event, &event_key, config).await? {
                debug!(
                    event_type = %event.event_type,
                    event_key = %event_key,
                    "Event filtered out by configuration"
                );
                return Ok(());
            }
        }

        match event.event_type.as_str() {
            "pull_request" => {
                crate::event_handlers::handle_pr_opened(self.database.clone(), event).await
            }
            "pull_request_review" => {
                crate::event_handlers::handle_pr_review_submitted(self.database.clone(), event)
                    .await
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

    /// Get the event key for configuration lookup (e.g., "pull_request.opened")
    fn get_event_key(&self, event: &WebhookEvent) -> String {
        // Parse payload to get action
        if let Ok(payload) = serde_json::from_str::<serde_json::Value>(&event.payload) {
            if let Some(action) = payload.get("action").and_then(|v| v.as_str()) {
                return format!("{}.{}", event.event_type, action);
            }
        }

        // For push events, there's no action field
        if event.event_type == "push" {
            return "push".to_string();
        }

        // Fallback to just event type
        event.event_type.clone()
    }

    /// Check if event should be processed based on filters
    async fn should_process_event(
        &self,
        event: &WebhookEvent,
        event_key: &str,
        config: &WebhookConfig,
    ) -> orchestrate_core::Result<bool> {
        let filter = match config.get_filter(event_key) {
            Some(f) => f,
            None => return Ok(true), // No filter = allow
        };

        // Parse payload for filtering
        let payload: serde_json::Value = serde_json::from_str(&event.payload)?;

        // Apply filters based on event type
        match event.event_type.as_str() {
            "pull_request" | "pull_request_review" => {
                let pr = payload.get("pull_request");

                // Check base branch filter
                if let Some(pr) = pr {
                    if let Some(base_ref) = pr
                        .get("base")
                        .and_then(|b| b.get("ref"))
                        .and_then(|r| r.as_str())
                    {
                        if !filter.allows_branch(base_ref) {
                            return Ok(false);
                        }
                    }

                    // Check fork filter
                    if let Some(is_fork) = pr
                        .get("head")
                        .and_then(|h| h.get("repo"))
                        .and_then(|r| r.get("fork"))
                        .and_then(|f| f.as_bool())
                    {
                        if !filter.allows_fork(is_fork) {
                            return Ok(false);
                        }
                    }
                }
            }
            "check_run" => {
                // Check conclusion filter
                if let Some(conclusion) = payload
                    .get("check_run")
                    .and_then(|cr| cr.get("conclusion"))
                    .and_then(|c| c.as_str())
                {
                    if !filter.allows_conclusion(conclusion) {
                        return Ok(false);
                    }
                }
            }
            "check_suite" => {
                // Check conclusion filter
                if let Some(conclusion) = payload
                    .get("check_suite")
                    .and_then(|cs| cs.get("conclusion"))
                    .and_then(|c| c.as_str())
                {
                    if !filter.allows_conclusion(conclusion) {
                        return Ok(false);
                    }
                }
            }
            "issues" => {
                // Check labels filter
                if let Some(labels) = payload
                    .get("issue")
                    .and_then(|i| i.get("labels"))
                    .and_then(|l| l.as_array())
                {
                    let label_names: Vec<String> = labels
                        .iter()
                        .filter_map(|l| l.get("name").and_then(|n| n.as_str()))
                        .map(|s| s.to_string())
                        .collect();

                    if !filter.allows_labels(&label_names) {
                        return Ok(false);
                    }
                }

                // Check author filter
                if let Some(author) = payload
                    .get("issue")
                    .and_then(|i| i.get("user"))
                    .and_then(|u| u.get("login"))
                    .and_then(|l| l.as_str())
                {
                    if !filter.allows_author(author) {
                        return Ok(false);
                    }
                }
            }
            _ => {}
        }

        Ok(true)
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
