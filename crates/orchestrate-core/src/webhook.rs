//! Webhook event queue and processing
//!
//! This module handles GitHub webhook event queueing, processing, and retry logic.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use crate::{Error, Result};

/// Status of a webhook event in the queue
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEventStatus {
    /// Event is pending processing
    Pending,
    /// Event is currently being processed
    Processing,
    /// Event was processed successfully
    Completed,
    /// Event processing failed (may retry)
    Failed,
    /// Event moved to dead letter queue (max retries exceeded)
    DeadLetter,
}

impl WebhookEventStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::DeadLetter => "dead_letter",
        }
    }
}

impl FromStr for WebhookEventStatus {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "pending" => Ok(Self::Pending),
            "processing" => Ok(Self::Processing),
            "completed" => Ok(Self::Completed),
            "failed" => Ok(Self::Failed),
            "dead_letter" => Ok(Self::DeadLetter),
            _ => Err(Error::Other(format!("Invalid webhook event status: {}", s))),
        }
    }
}

/// A webhook event in the queue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    /// Database ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// GitHub delivery ID (for idempotency)
    pub delivery_id: String,
    /// Event type (e.g., "pull_request", "check_run")
    pub event_type: String,
    /// Raw JSON payload from GitHub
    pub payload: String,
    /// Current status
    pub status: WebhookEventStatus,
    /// Number of retry attempts
    pub retry_count: i32,
    /// Maximum retry attempts before dead-letter
    pub max_retries: i32,
    /// Error message if processing failed
    pub error_message: Option<String>,
    /// When to retry next (for exponential backoff)
    pub next_retry_at: Option<DateTime<Utc>>,
    /// When the webhook was received
    pub received_at: DateTime<Utc>,
    /// When processing completed
    pub processed_at: Option<DateTime<Utc>>,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Updated timestamp
    pub updated_at: DateTime<Utc>,
}

impl WebhookEvent {
    /// Create a new webhook event
    pub fn new(delivery_id: String, event_type: String, payload: String) -> Self {
        let now = Utc::now();
        Self {
            id: None,
            delivery_id,
            event_type,
            payload,
            status: WebhookEventStatus::Pending,
            retry_count: 0,
            max_retries: 3,
            error_message: None,
            next_retry_at: None,
            received_at: now,
            processed_at: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Check if event can be retried
    pub fn can_retry(&self) -> bool {
        self.retry_count < self.max_retries
    }

    /// Calculate next retry time using exponential backoff
    /// Backoff: 1s, 2s, 4s, 8s, etc.
    pub fn calculate_next_retry(&self) -> DateTime<Utc> {
        let backoff_seconds = 2_i64.pow(self.retry_count as u32);
        Utc::now() + chrono::Duration::seconds(backoff_seconds)
    }

    /// Mark event as processing
    pub fn mark_processing(&mut self) {
        self.status = WebhookEventStatus::Processing;
        self.updated_at = Utc::now();
    }

    /// Mark event as completed
    pub fn mark_completed(&mut self) {
        self.status = WebhookEventStatus::Completed;
        self.processed_at = Some(Utc::now());
        self.updated_at = Utc::now();
    }

    /// Mark event as failed and prepare for retry
    pub fn mark_failed(&mut self, error: String) {
        self.error_message = Some(error);
        self.updated_at = Utc::now();

        if self.can_retry() {
            self.retry_count += 1;
            self.status = WebhookEventStatus::Pending;
            self.next_retry_at = Some(self.calculate_next_retry());
        } else {
            self.retry_count += 1;
            self.status = WebhookEventStatus::DeadLetter;
            self.next_retry_at = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webhook_event_new() {
        let event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            r#"{"action":"opened"}"#.to_string(),
        );

        assert_eq!(event.delivery_id, "delivery-123");
        assert_eq!(event.event_type, "pull_request");
        assert_eq!(event.status, WebhookEventStatus::Pending);
        assert_eq!(event.retry_count, 0);
        assert_eq!(event.max_retries, 3);
        assert!(event.can_retry());
    }

    #[test]
    fn test_webhook_event_mark_processing() {
        let mut event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            "{}".to_string(),
        );

        event.mark_processing();
        assert_eq!(event.status, WebhookEventStatus::Processing);
    }

    #[test]
    fn test_webhook_event_mark_completed() {
        let mut event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            "{}".to_string(),
        );

        event.mark_completed();
        assert_eq!(event.status, WebhookEventStatus::Completed);
        assert!(event.processed_at.is_some());
    }

    #[test]
    fn test_webhook_event_mark_failed_with_retries() {
        let mut event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            "{}".to_string(),
        );

        // First failure - should remain pending with retry scheduled
        event.mark_failed("test error".to_string());
        assert_eq!(event.status, WebhookEventStatus::Pending);
        assert_eq!(event.retry_count, 1);
        assert!(event.next_retry_at.is_some());
        assert!(event.can_retry());

        // Second failure
        event.mark_failed("test error 2".to_string());
        assert_eq!(event.status, WebhookEventStatus::Pending);
        assert_eq!(event.retry_count, 2);
        assert!(event.can_retry());

        // Third failure
        event.mark_failed("test error 3".to_string());
        assert_eq!(event.status, WebhookEventStatus::Pending);
        assert_eq!(event.retry_count, 3);
        assert!(!event.can_retry());
    }

    #[test]
    fn test_webhook_event_mark_failed_max_retries() {
        let mut event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            "{}".to_string(),
        );

        // Exhaust all retries
        for i in 1..=3 {
            event.mark_failed(format!("error {}", i));
        }

        // Should still be pending after 3 retries
        assert_eq!(event.retry_count, 3);
        assert!(!event.can_retry());

        // Fourth failure should move to dead letter
        event.mark_failed("final error".to_string());
        assert_eq!(event.status, WebhookEventStatus::DeadLetter);
        assert_eq!(event.retry_count, 4);
        assert!(event.next_retry_at.is_none());
    }

    #[test]
    fn test_webhook_event_exponential_backoff() {
        let mut event = WebhookEvent::new(
            "delivery-123".to_string(),
            "pull_request".to_string(),
            "{}".to_string(),
        );

        let now = Utc::now();

        // First retry: 2^0 = 1 second
        let next = event.calculate_next_retry();
        assert!(next > now);
        assert!(next <= now + chrono::Duration::seconds(2));

        // Second retry: 2^1 = 2 seconds
        event.retry_count = 1;
        let next = event.calculate_next_retry();
        assert!(next > now + chrono::Duration::seconds(1));
        assert!(next <= now + chrono::Duration::seconds(3));

        // Third retry: 2^2 = 4 seconds
        event.retry_count = 2;
        let next = event.calculate_next_retry();
        assert!(next > now + chrono::Duration::seconds(3));
        assert!(next <= now + chrono::Duration::seconds(5));
    }

    #[test]
    fn test_webhook_event_status_parsing() {
        assert_eq!(
            WebhookEventStatus::from_str("pending").unwrap(),
            WebhookEventStatus::Pending
        );
        assert_eq!(
            WebhookEventStatus::from_str("processing").unwrap(),
            WebhookEventStatus::Processing
        );
        assert_eq!(
            WebhookEventStatus::from_str("completed").unwrap(),
            WebhookEventStatus::Completed
        );
        assert_eq!(
            WebhookEventStatus::from_str("failed").unwrap(),
            WebhookEventStatus::Failed
        );
        assert_eq!(
            WebhookEventStatus::from_str("dead_letter").unwrap(),
            WebhookEventStatus::DeadLetter
        );

        assert!(WebhookEventStatus::from_str("invalid").is_err());
    }

    #[test]
    fn test_webhook_event_status_as_str() {
        assert_eq!(WebhookEventStatus::Pending.as_str(), "pending");
        assert_eq!(WebhookEventStatus::Processing.as_str(), "processing");
        assert_eq!(WebhookEventStatus::Completed.as_str(), "completed");
        assert_eq!(WebhookEventStatus::Failed.as_str(), "failed");
        assert_eq!(WebhookEventStatus::DeadLetter.as_str(), "dead_letter");
    }
}
