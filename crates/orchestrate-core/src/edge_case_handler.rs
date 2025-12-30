//! Edge Case Handling
//!
//! Epic 016: Autonomous Epic Processing - Story 14
//!
//! Handles common edge cases in autonomous processing including:
//! - Delayed CI reviews (GitHub Copilot async comments)
//! - Merge conflicts when multiple branches merge
//! - Flaky tests with retry logic
//! - External service downtime (GitHub, CI)
//! - Story dependency failures
//! - Review ping-pong (repeated changes requested)
//! - Context overflow for large changes
//! - Logging all edge cases for learning

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types of edge cases that can occur during autonomous processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeCaseType {
    /// CI review is delayed (e.g., GitHub Copilot async comments)
    DelayedCiReview,
    /// Merge conflict detected when merging branches
    MergeConflict,
    /// Test is flaky and needs retry
    FlakyTest,
    /// External service is down (GitHub, CI provider)
    ServiceDowntime,
    /// A dependent story failed
    DependencyFailure,
    /// Review ping-pong (too many review iterations)
    ReviewPingPong,
    /// Context window is overflowing
    ContextOverflow,
    /// Rate limit hit on API
    RateLimit,
    /// Timeout waiting for response
    Timeout,
    /// Authentication or permission error
    AuthError,
    /// Network error
    NetworkError,
    /// Unknown/other edge case
    Unknown,
}

impl EdgeCaseType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DelayedCiReview => "delayed_ci_review",
            Self::MergeConflict => "merge_conflict",
            Self::FlakyTest => "flaky_test",
            Self::ServiceDowntime => "service_downtime",
            Self::DependencyFailure => "dependency_failure",
            Self::ReviewPingPong => "review_ping_pong",
            Self::ContextOverflow => "context_overflow",
            Self::RateLimit => "rate_limit",
            Self::Timeout => "timeout",
            Self::AuthError => "auth_error",
            Self::NetworkError => "network_error",
            Self::Unknown => "unknown",
        }
    }

    /// Get recommended action for this edge case type
    pub fn recommended_action(&self) -> EdgeCaseAction {
        match self {
            Self::DelayedCiReview => EdgeCaseAction::Wait {
                max_wait: Duration::hours(2),
                check_interval: Duration::minutes(5),
            },
            Self::MergeConflict => EdgeCaseAction::SpawnResolver {
                resolver_type: "conflict_resolver".to_string(),
            },
            Self::FlakyTest => EdgeCaseAction::Retry {
                max_retries: 3,
                backoff_seconds: 30,
            },
            Self::ServiceDowntime => EdgeCaseAction::Wait {
                max_wait: Duration::minutes(30),
                check_interval: Duration::minutes(2),
            },
            Self::DependencyFailure => EdgeCaseAction::Block {
                reason: "Dependent story failed".to_string(),
            },
            Self::ReviewPingPong => EdgeCaseAction::Escalate {
                severity: "high".to_string(),
                reason: "Review iteration limit exceeded".to_string(),
            },
            Self::ContextOverflow => EdgeCaseAction::Summarize {
                max_tokens: 50000,
            },
            Self::RateLimit => EdgeCaseAction::Backoff {
                initial_delay: Duration::minutes(1),
                max_delay: Duration::minutes(30),
            },
            Self::Timeout => EdgeCaseAction::Retry {
                max_retries: 2,
                backoff_seconds: 60,
            },
            Self::AuthError => EdgeCaseAction::Escalate {
                severity: "critical".to_string(),
                reason: "Authentication or permission error".to_string(),
            },
            Self::NetworkError => EdgeCaseAction::Retry {
                max_retries: 5,
                backoff_seconds: 10,
            },
            Self::Unknown => EdgeCaseAction::Log,
        }
    }
}

impl std::str::FromStr for EdgeCaseType {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "delayed_ci_review" => Ok(Self::DelayedCiReview),
            "merge_conflict" => Ok(Self::MergeConflict),
            "flaky_test" => Ok(Self::FlakyTest),
            "service_downtime" => Ok(Self::ServiceDowntime),
            "dependency_failure" => Ok(Self::DependencyFailure),
            "review_ping_pong" => Ok(Self::ReviewPingPong),
            "context_overflow" => Ok(Self::ContextOverflow),
            "rate_limit" => Ok(Self::RateLimit),
            "timeout" => Ok(Self::Timeout),
            "auth_error" => Ok(Self::AuthError),
            "network_error" => Ok(Self::NetworkError),
            "unknown" => Ok(Self::Unknown),
            _ => Err(crate::Error::Other(format!("Invalid edge case type: {}", s))),
        }
    }
}

impl std::fmt::Display for EdgeCaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Resolution status for an edge case
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeCaseResolution {
    /// Edge case is pending resolution
    Pending,
    /// Edge case was resolved automatically
    AutoResolved,
    /// Edge case was resolved by human intervention
    ManualResolved,
    /// Edge case was bypassed/skipped
    Bypassed,
    /// Edge case resolution failed
    Failed,
    /// Edge case is being retried
    Retrying,
    /// Waiting for external condition
    Waiting,
}

impl EdgeCaseResolution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::AutoResolved => "auto_resolved",
            Self::ManualResolved => "manual_resolved",
            Self::Bypassed => "bypassed",
            Self::Failed => "failed",
            Self::Retrying => "retrying",
            Self::Waiting => "waiting",
        }
    }

    pub fn is_resolved(&self) -> bool {
        matches!(self, Self::AutoResolved | Self::ManualResolved | Self::Bypassed)
    }
}

impl std::str::FromStr for EdgeCaseResolution {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "auto_resolved" => Ok(Self::AutoResolved),
            "manual_resolved" => Ok(Self::ManualResolved),
            "bypassed" => Ok(Self::Bypassed),
            "failed" => Ok(Self::Failed),
            "retrying" => Ok(Self::Retrying),
            "waiting" => Ok(Self::Waiting),
            _ => Err(crate::Error::Other(format!(
                "Invalid edge case resolution: {}",
                s
            ))),
        }
    }
}

impl std::fmt::Display for EdgeCaseResolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Actions that can be taken for edge cases
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum EdgeCaseAction {
    /// Retry the operation
    Retry {
        max_retries: u32,
        backoff_seconds: u32,
    },
    /// Wait for a condition
    Wait {
        max_wait: Duration,
        check_interval: Duration,
    },
    /// Spawn a specialized resolver agent
    SpawnResolver { resolver_type: String },
    /// Apply exponential backoff
    Backoff {
        initial_delay: Duration,
        max_delay: Duration,
    },
    /// Summarize/compress context
    Summarize { max_tokens: u32 },
    /// Block and mark for human intervention
    Block { reason: String },
    /// Escalate to higher priority
    Escalate { severity: String, reason: String },
    /// Skip and continue
    Skip { reason: String },
    /// Just log the edge case
    Log,
}

/// An edge case event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCaseEvent {
    /// Unique ID
    pub id: i64,
    /// Session ID where edge case occurred
    pub session_id: Option<String>,
    /// Agent ID involved
    pub agent_id: Option<String>,
    /// Story ID if applicable
    pub story_id: Option<String>,
    /// Type of edge case
    pub edge_case_type: EdgeCaseType,
    /// Resolution status
    pub resolution: EdgeCaseResolution,
    /// Action taken
    pub action_taken: Option<String>,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Error message or details
    pub error_message: Option<String>,
    /// Additional context
    pub context: serde_json::Value,
    /// When the edge case was detected
    pub detected_at: DateTime<Utc>,
    /// When it was resolved
    pub resolved_at: Option<DateTime<Utc>>,
    /// Resolution notes
    pub resolution_notes: Option<String>,
}

impl EdgeCaseEvent {
    pub fn new(edge_case_type: EdgeCaseType) -> Self {
        Self {
            id: 0,
            session_id: None,
            agent_id: None,
            story_id: None,
            edge_case_type,
            resolution: EdgeCaseResolution::Pending,
            action_taken: None,
            retry_count: 0,
            error_message: None,
            context: serde_json::json!({}),
            detected_at: Utc::now(),
            resolved_at: None,
            resolution_notes: None,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_story(mut self, story_id: impl Into<String>) -> Self {
        self.story_id = Some(story_id.into());
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error_message = Some(error.into());
        self
    }

    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = context;
        self
    }

    pub fn resolve(&mut self, resolution: EdgeCaseResolution, notes: Option<String>) {
        self.resolution = resolution;
        self.resolved_at = Some(Utc::now());
        self.resolution_notes = notes;
    }

    pub fn increment_retry(&mut self) {
        self.retry_count += 1;
        self.resolution = EdgeCaseResolution::Retrying;
    }

    pub fn duration(&self) -> Option<Duration> {
        self.resolved_at.map(|resolved| resolved - self.detected_at)
    }
}

/// Configuration for edge case handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCaseConfig {
    /// Maximum retries for flaky tests
    pub flaky_test_max_retries: u32,
    /// Base backoff for flaky tests in seconds
    pub flaky_test_backoff_seconds: u32,
    /// Maximum wait time for delayed CI reviews in minutes
    pub delayed_review_max_wait_minutes: u32,
    /// Check interval for delayed reviews in minutes
    pub delayed_review_check_interval_minutes: u32,
    /// Maximum review iterations before escalation
    pub review_ping_pong_threshold: u32,
    /// Maximum context tokens before summarization
    pub context_overflow_threshold: u32,
    /// Initial rate limit backoff in seconds
    pub rate_limit_initial_backoff_seconds: u32,
    /// Maximum rate limit backoff in seconds
    pub rate_limit_max_backoff_seconds: u32,
    /// Whether to auto-retry network errors
    pub auto_retry_network_errors: bool,
    /// Maximum network error retries
    pub network_error_max_retries: u32,
}

impl Default for EdgeCaseConfig {
    fn default() -> Self {
        Self {
            flaky_test_max_retries: 3,
            flaky_test_backoff_seconds: 30,
            delayed_review_max_wait_minutes: 120,
            delayed_review_check_interval_minutes: 5,
            review_ping_pong_threshold: 5,
            context_overflow_threshold: 100_000,
            rate_limit_initial_backoff_seconds: 60,
            rate_limit_max_backoff_seconds: 1800,
            auto_retry_network_errors: true,
            network_error_max_retries: 5,
        }
    }
}

/// Handler result with recommended action
#[derive(Debug, Clone)]
pub struct HandlerResult {
    /// The edge case event
    pub event: EdgeCaseEvent,
    /// Recommended action
    pub action: EdgeCaseAction,
    /// Whether processing should continue
    pub should_continue: bool,
    /// Optional message for logging
    pub message: Option<String>,
}

/// Edge case detection and handling
#[derive(Debug, Clone)]
pub struct EdgeCaseHandler {
    config: EdgeCaseConfig,
    /// Track retry counts per context
    retry_tracker: HashMap<String, u32>,
    /// Track review iterations per PR
    review_iterations: HashMap<String, u32>,
    /// Track rate limit state
    rate_limit_state: HashMap<String, RateLimitState>,
}

#[derive(Debug, Clone)]
struct RateLimitState {
    last_hit: DateTime<Utc>,
    current_backoff_seconds: u32,
    hit_count: u32,
}

impl EdgeCaseHandler {
    pub fn new() -> Self {
        Self::with_config(EdgeCaseConfig::default())
    }

    pub fn with_config(config: EdgeCaseConfig) -> Self {
        Self {
            config,
            retry_tracker: HashMap::new(),
            review_iterations: HashMap::new(),
            rate_limit_state: HashMap::new(),
        }
    }

    /// Detect edge case type from error message or context
    pub fn detect_edge_case(&self, error_message: &str, context: &serde_json::Value) -> EdgeCaseType {
        let error_lower = error_message.to_lowercase();

        // Check for specific patterns
        if error_lower.contains("merge conflict") || error_lower.contains("cannot be merged") {
            return EdgeCaseType::MergeConflict;
        }

        if error_lower.contains("rate limit") || error_lower.contains("429") {
            return EdgeCaseType::RateLimit;
        }

        if error_lower.contains("timeout") || error_lower.contains("timed out") {
            return EdgeCaseType::Timeout;
        }

        if error_lower.contains("flaky")
            || error_lower.contains("intermittent")
            || (error_lower.contains("test") && error_lower.contains("failed") && context.get("retry_count").is_some())
        {
            return EdgeCaseType::FlakyTest;
        }

        if error_lower.contains("copilot")
            || error_lower.contains("pending review")
            || error_lower.contains("review not ready")
        {
            return EdgeCaseType::DelayedCiReview;
        }

        if error_lower.contains("service unavailable")
            || error_lower.contains("502")
            || error_lower.contains("503")
            || error_lower.contains("504")
        {
            return EdgeCaseType::ServiceDowntime;
        }

        if error_lower.contains("dependency")
            || error_lower.contains("depends on")
            || error_lower.contains("prerequisite failed")
        {
            return EdgeCaseType::DependencyFailure;
        }

        if error_lower.contains("changes requested")
            || error_lower.contains("review iteration")
            || context.get("review_count").and_then(|v| v.as_u64()).unwrap_or(0) > 3
        {
            return EdgeCaseType::ReviewPingPong;
        }

        if error_lower.contains("context")
            || error_lower.contains("token limit")
            || error_lower.contains("too long")
            || context.get("token_count").and_then(|v| v.as_u64()).unwrap_or(0)
                > self.config.context_overflow_threshold as u64
        {
            return EdgeCaseType::ContextOverflow;
        }

        if error_lower.contains("unauthorized")
            || error_lower.contains("forbidden")
            || error_lower.contains("401")
            || error_lower.contains("403")
        {
            return EdgeCaseType::AuthError;
        }

        if error_lower.contains("network")
            || error_lower.contains("connection")
            || error_lower.contains("dns")
        {
            return EdgeCaseType::NetworkError;
        }

        EdgeCaseType::Unknown
    }

    /// Handle a detected edge case and return recommended action
    pub fn handle(&mut self, event: &mut EdgeCaseEvent) -> HandlerResult {
        let context_key = format!(
            "{}:{}:{}",
            event.session_id.as_deref().unwrap_or("none"),
            event.agent_id.as_deref().unwrap_or("none"),
            event.edge_case_type.as_str()
        );

        let action = match event.edge_case_type {
            EdgeCaseType::FlakyTest => self.handle_flaky_test(&context_key, event),
            EdgeCaseType::DelayedCiReview => self.handle_delayed_review(event),
            EdgeCaseType::MergeConflict => self.handle_merge_conflict(event),
            EdgeCaseType::ServiceDowntime => self.handle_service_downtime(event),
            EdgeCaseType::DependencyFailure => self.handle_dependency_failure(event),
            EdgeCaseType::ReviewPingPong => self.handle_review_ping_pong(&context_key, event),
            EdgeCaseType::ContextOverflow => self.handle_context_overflow(event),
            EdgeCaseType::RateLimit => self.handle_rate_limit(&context_key, event),
            EdgeCaseType::Timeout => self.handle_timeout(&context_key, event),
            EdgeCaseType::NetworkError => self.handle_network_error(&context_key, event),
            EdgeCaseType::AuthError => self.handle_auth_error(event),
            EdgeCaseType::Unknown => HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Log,
                should_continue: true,
                message: Some("Unknown edge case, logging for analysis".to_string()),
            },
        };

        // Update action taken
        event.action_taken = Some(format!("{:?}", action.action));

        action
    }

    fn handle_flaky_test(&mut self, context_key: &str, event: &mut EdgeCaseEvent) -> HandlerResult {
        let retry_count = self.retry_tracker.entry(context_key.to_string()).or_insert(0);
        *retry_count += 1;
        event.retry_count = *retry_count;

        if *retry_count <= self.config.flaky_test_max_retries {
            let backoff = self.config.flaky_test_backoff_seconds * *retry_count;
            HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Retry {
                    max_retries: self.config.flaky_test_max_retries,
                    backoff_seconds: backoff,
                },
                should_continue: true,
                message: Some(format!(
                    "Flaky test retry {}/{}, waiting {}s",
                    retry_count, self.config.flaky_test_max_retries, backoff
                )),
            }
        } else {
            event.resolution = EdgeCaseResolution::Failed;
            HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Escalate {
                    severity: "medium".to_string(),
                    reason: format!("Flaky test failed after {} retries", retry_count),
                },
                should_continue: false,
                message: Some("Flaky test exceeded retry limit, escalating".to_string()),
            }
        }
    }

    fn handle_delayed_review(&self, event: &mut EdgeCaseEvent) -> HandlerResult {
        event.resolution = EdgeCaseResolution::Waiting;
        HandlerResult {
            event: event.clone(),
            action: EdgeCaseAction::Wait {
                max_wait: Duration::minutes(self.config.delayed_review_max_wait_minutes as i64),
                check_interval: Duration::minutes(
                    self.config.delayed_review_check_interval_minutes as i64,
                ),
            },
            should_continue: true,
            message: Some(format!(
                "Waiting for delayed CI review (max {}min)",
                self.config.delayed_review_max_wait_minutes
            )),
        }
    }

    fn handle_merge_conflict(&self, event: &mut EdgeCaseEvent) -> HandlerResult {
        HandlerResult {
            event: event.clone(),
            action: EdgeCaseAction::SpawnResolver {
                resolver_type: "conflict_resolver".to_string(),
            },
            should_continue: true,
            message: Some("Spawning conflict resolver agent".to_string()),
        }
    }

    fn handle_service_downtime(&self, event: &mut EdgeCaseEvent) -> HandlerResult {
        event.resolution = EdgeCaseResolution::Waiting;
        HandlerResult {
            event: event.clone(),
            action: EdgeCaseAction::Wait {
                max_wait: Duration::minutes(30),
                check_interval: Duration::minutes(2),
            },
            should_continue: true,
            message: Some("Service downtime detected, waiting for recovery".to_string()),
        }
    }

    fn handle_dependency_failure(&self, event: &mut EdgeCaseEvent) -> HandlerResult {
        HandlerResult {
            event: event.clone(),
            action: EdgeCaseAction::Block {
                reason: "Dependent story failed".to_string(),
            },
            should_continue: false,
            message: Some("Blocking due to dependency failure".to_string()),
        }
    }

    fn handle_review_ping_pong(
        &mut self,
        context_key: &str,
        event: &mut EdgeCaseEvent,
    ) -> HandlerResult {
        let iterations = self
            .review_iterations
            .entry(context_key.to_string())
            .or_insert(0);
        *iterations += 1;

        if *iterations >= self.config.review_ping_pong_threshold {
            event.resolution = EdgeCaseResolution::Failed;
            HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Escalate {
                    severity: "high".to_string(),
                    reason: format!(
                        "Review ping-pong: {} iterations exceeded threshold",
                        iterations
                    ),
                },
                should_continue: false,
                message: Some("Review iteration limit exceeded, escalating".to_string()),
            }
        } else {
            HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Log,
                should_continue: true,
                message: Some(format!("Review iteration {}", iterations)),
            }
        }
    }

    fn handle_context_overflow(&self, event: &mut EdgeCaseEvent) -> HandlerResult {
        HandlerResult {
            event: event.clone(),
            action: EdgeCaseAction::Summarize {
                max_tokens: self.config.context_overflow_threshold / 2,
            },
            should_continue: true,
            message: Some("Context overflow detected, triggering summarization".to_string()),
        }
    }

    fn handle_rate_limit(&mut self, context_key: &str, event: &mut EdgeCaseEvent) -> HandlerResult {
        let state = self
            .rate_limit_state
            .entry(context_key.to_string())
            .or_insert(RateLimitState {
                last_hit: Utc::now(),
                current_backoff_seconds: self.config.rate_limit_initial_backoff_seconds,
                hit_count: 0,
            });

        state.hit_count += 1;
        state.last_hit = Utc::now();

        // Exponential backoff
        let backoff = state.current_backoff_seconds.min(self.config.rate_limit_max_backoff_seconds);
        state.current_backoff_seconds = (state.current_backoff_seconds * 2)
            .min(self.config.rate_limit_max_backoff_seconds);

        event.resolution = EdgeCaseResolution::Waiting;

        HandlerResult {
            event: event.clone(),
            action: EdgeCaseAction::Backoff {
                initial_delay: Duration::seconds(backoff as i64),
                max_delay: Duration::seconds(self.config.rate_limit_max_backoff_seconds as i64),
            },
            should_continue: true,
            message: Some(format!(
                "Rate limit hit (count: {}), backing off {}s",
                state.hit_count, backoff
            )),
        }
    }

    fn handle_timeout(&mut self, context_key: &str, event: &mut EdgeCaseEvent) -> HandlerResult {
        let retry_count = self.retry_tracker.entry(context_key.to_string()).or_insert(0);
        *retry_count += 1;
        event.retry_count = *retry_count;

        if *retry_count <= 2 {
            HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Retry {
                    max_retries: 2,
                    backoff_seconds: 60,
                },
                should_continue: true,
                message: Some(format!("Timeout retry {}/2", retry_count)),
            }
        } else {
            event.resolution = EdgeCaseResolution::Failed;
            HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Escalate {
                    severity: "medium".to_string(),
                    reason: "Timeout after multiple retries".to_string(),
                },
                should_continue: false,
                message: Some("Timeout exceeded retry limit".to_string()),
            }
        }
    }

    fn handle_network_error(
        &mut self,
        context_key: &str,
        event: &mut EdgeCaseEvent,
    ) -> HandlerResult {
        if !self.config.auto_retry_network_errors {
            return HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Escalate {
                    severity: "medium".to_string(),
                    reason: "Network error (auto-retry disabled)".to_string(),
                },
                should_continue: false,
                message: Some("Network error, auto-retry disabled".to_string()),
            };
        }

        let retry_count = self.retry_tracker.entry(context_key.to_string()).or_insert(0);
        *retry_count += 1;
        event.retry_count = *retry_count;

        if *retry_count <= self.config.network_error_max_retries {
            let backoff = 10 * *retry_count;
            HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Retry {
                    max_retries: self.config.network_error_max_retries,
                    backoff_seconds: backoff,
                },
                should_continue: true,
                message: Some(format!(
                    "Network error retry {}/{}, waiting {}s",
                    retry_count, self.config.network_error_max_retries, backoff
                )),
            }
        } else {
            event.resolution = EdgeCaseResolution::Failed;
            HandlerResult {
                event: event.clone(),
                action: EdgeCaseAction::Escalate {
                    severity: "high".to_string(),
                    reason: "Network error persisted after retries".to_string(),
                },
                should_continue: false,
                message: Some("Network error exceeded retry limit".to_string()),
            }
        }
    }

    fn handle_auth_error(&self, event: &mut EdgeCaseEvent) -> HandlerResult {
        event.resolution = EdgeCaseResolution::Failed;
        HandlerResult {
            event: event.clone(),
            action: EdgeCaseAction::Escalate {
                severity: "critical".to_string(),
                reason: "Authentication or permission error requires manual intervention"
                    .to_string(),
            },
            should_continue: false,
            message: Some("Auth error detected, immediate escalation required".to_string()),
        }
    }

    /// Reset retry counter for a context
    pub fn reset_retries(&mut self, session_id: Option<&str>, agent_id: Option<&str>) {
        let prefix = format!(
            "{}:{}:",
            session_id.unwrap_or("none"),
            agent_id.unwrap_or("none")
        );
        self.retry_tracker.retain(|k, _| !k.starts_with(&prefix));
    }

    /// Reset rate limit state for a service
    pub fn reset_rate_limit(&mut self, service: &str) {
        self.rate_limit_state.remove(service);
    }

    /// Get statistics about edge cases
    pub fn get_stats(&self) -> EdgeCaseStats {
        EdgeCaseStats {
            active_retries: self.retry_tracker.len(),
            active_review_iterations: self.review_iterations.len(),
            rate_limited_services: self.rate_limit_state.len(),
        }
    }
}

impl Default for EdgeCaseHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about edge case handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCaseStats {
    pub active_retries: usize,
    pub active_review_iterations: usize,
    pub rate_limited_services: usize,
}

/// Learning record for edge case patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeCaseLearning {
    pub id: i64,
    pub edge_case_type: EdgeCaseType,
    pub pattern: String,
    pub success_rate: f64,
    pub avg_resolution_time_seconds: Option<f64>,
    pub recommended_action: String,
    pub occurrence_count: u32,
    pub last_occurrence: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl EdgeCaseLearning {
    pub fn new(edge_case_type: EdgeCaseType, pattern: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: 0,
            edge_case_type,
            pattern: pattern.into(),
            success_rate: 0.0,
            avg_resolution_time_seconds: None,
            recommended_action: edge_case_type.recommended_action().to_string(),
            occurrence_count: 1,
            last_occurrence: now,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn record_occurrence(&mut self, success: bool, resolution_time_seconds: Option<f64>) {
        self.occurrence_count += 1;
        self.last_occurrence = Utc::now();
        self.updated_at = Utc::now();

        // Update success rate (rolling average)
        let success_value = if success { 1.0 } else { 0.0 };
        self.success_rate = (self.success_rate * (self.occurrence_count - 1) as f64 + success_value)
            / self.occurrence_count as f64;

        // Update average resolution time
        if let Some(time) = resolution_time_seconds {
            self.avg_resolution_time_seconds = Some(match self.avg_resolution_time_seconds {
                Some(avg) => {
                    (avg * (self.occurrence_count - 1) as f64 + time) / self.occurrence_count as f64
                }
                None => time,
            });
        }
    }
}

impl std::fmt::Display for EdgeCaseAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Retry { max_retries, backoff_seconds } => {
                write!(f, "retry(max={}, backoff={}s)", max_retries, backoff_seconds)
            }
            Self::Wait { max_wait, check_interval } => {
                write!(
                    f,
                    "wait(max={}min, interval={}min)",
                    max_wait.num_minutes(),
                    check_interval.num_minutes()
                )
            }
            Self::SpawnResolver { resolver_type } => {
                write!(f, "spawn_resolver({})", resolver_type)
            }
            Self::Backoff { initial_delay, max_delay } => {
                write!(
                    f,
                    "backoff(initial={}s, max={}s)",
                    initial_delay.num_seconds(),
                    max_delay.num_seconds()
                )
            }
            Self::Summarize { max_tokens } => write!(f, "summarize(max_tokens={})", max_tokens),
            Self::Block { reason } => write!(f, "block({})", reason),
            Self::Escalate { severity, reason } => {
                write!(f, "escalate(severity={}, reason={})", severity, reason)
            }
            Self::Skip { reason } => write!(f, "skip({})", reason),
            Self::Log => write!(f, "log"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==================== EdgeCaseType Tests ====================

    #[test]
    fn test_edge_case_type_as_str() {
        assert_eq!(EdgeCaseType::DelayedCiReview.as_str(), "delayed_ci_review");
        assert_eq!(EdgeCaseType::MergeConflict.as_str(), "merge_conflict");
        assert_eq!(EdgeCaseType::FlakyTest.as_str(), "flaky_test");
        assert_eq!(EdgeCaseType::ServiceDowntime.as_str(), "service_downtime");
        assert_eq!(EdgeCaseType::DependencyFailure.as_str(), "dependency_failure");
        assert_eq!(EdgeCaseType::ReviewPingPong.as_str(), "review_ping_pong");
        assert_eq!(EdgeCaseType::ContextOverflow.as_str(), "context_overflow");
        assert_eq!(EdgeCaseType::RateLimit.as_str(), "rate_limit");
        assert_eq!(EdgeCaseType::Timeout.as_str(), "timeout");
        assert_eq!(EdgeCaseType::AuthError.as_str(), "auth_error");
        assert_eq!(EdgeCaseType::NetworkError.as_str(), "network_error");
        assert_eq!(EdgeCaseType::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_edge_case_type_from_str() {
        assert_eq!(
            "delayed_ci_review".parse::<EdgeCaseType>().unwrap(),
            EdgeCaseType::DelayedCiReview
        );
        assert_eq!(
            "merge_conflict".parse::<EdgeCaseType>().unwrap(),
            EdgeCaseType::MergeConflict
        );
        assert_eq!(
            "flaky_test".parse::<EdgeCaseType>().unwrap(),
            EdgeCaseType::FlakyTest
        );
        assert!("invalid".parse::<EdgeCaseType>().is_err());
    }

    #[test]
    fn test_edge_case_type_recommended_action() {
        match EdgeCaseType::FlakyTest.recommended_action() {
            EdgeCaseAction::Retry { max_retries, .. } => {
                assert_eq!(max_retries, 3);
            }
            _ => panic!("Expected Retry action for flaky test"),
        }

        match EdgeCaseType::MergeConflict.recommended_action() {
            EdgeCaseAction::SpawnResolver { resolver_type } => {
                assert_eq!(resolver_type, "conflict_resolver");
            }
            _ => panic!("Expected SpawnResolver action for merge conflict"),
        }
    }

    // ==================== EdgeCaseResolution Tests ====================

    #[test]
    fn test_resolution_is_resolved() {
        assert!(EdgeCaseResolution::AutoResolved.is_resolved());
        assert!(EdgeCaseResolution::ManualResolved.is_resolved());
        assert!(EdgeCaseResolution::Bypassed.is_resolved());
        assert!(!EdgeCaseResolution::Pending.is_resolved());
        assert!(!EdgeCaseResolution::Failed.is_resolved());
        assert!(!EdgeCaseResolution::Retrying.is_resolved());
        assert!(!EdgeCaseResolution::Waiting.is_resolved());
    }

    // ==================== EdgeCaseEvent Tests ====================

    #[test]
    fn test_edge_case_event_new() {
        let event = EdgeCaseEvent::new(EdgeCaseType::FlakyTest);
        assert_eq!(event.edge_case_type, EdgeCaseType::FlakyTest);
        assert_eq!(event.resolution, EdgeCaseResolution::Pending);
        assert_eq!(event.retry_count, 0);
    }

    #[test]
    fn test_edge_case_event_builder() {
        let event = EdgeCaseEvent::new(EdgeCaseType::MergeConflict)
            .with_session("session-123")
            .with_agent("agent-456")
            .with_story("story-789")
            .with_error("Cannot merge due to conflicts");

        assert_eq!(event.session_id, Some("session-123".to_string()));
        assert_eq!(event.agent_id, Some("agent-456".to_string()));
        assert_eq!(event.story_id, Some("story-789".to_string()));
        assert_eq!(
            event.error_message,
            Some("Cannot merge due to conflicts".to_string())
        );
    }

    #[test]
    fn test_edge_case_event_resolve() {
        let mut event = EdgeCaseEvent::new(EdgeCaseType::FlakyTest);
        event.resolve(
            EdgeCaseResolution::AutoResolved,
            Some("Test passed on retry".to_string()),
        );

        assert_eq!(event.resolution, EdgeCaseResolution::AutoResolved);
        assert!(event.resolved_at.is_some());
        assert_eq!(
            event.resolution_notes,
            Some("Test passed on retry".to_string())
        );
    }

    #[test]
    fn test_edge_case_event_increment_retry() {
        let mut event = EdgeCaseEvent::new(EdgeCaseType::FlakyTest);
        assert_eq!(event.retry_count, 0);

        event.increment_retry();
        assert_eq!(event.retry_count, 1);
        assert_eq!(event.resolution, EdgeCaseResolution::Retrying);

        event.increment_retry();
        assert_eq!(event.retry_count, 2);
    }

    // ==================== EdgeCaseHandler Detection Tests ====================

    #[test]
    fn test_detect_merge_conflict() {
        let handler = EdgeCaseHandler::new();
        let detected =
            handler.detect_edge_case("Merge conflict in src/main.rs", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::MergeConflict);

        let detected = handler.detect_edge_case(
            "PR cannot be merged automatically",
            &serde_json::json!({}),
        );
        assert_eq!(detected, EdgeCaseType::MergeConflict);
    }

    #[test]
    fn test_detect_rate_limit() {
        let handler = EdgeCaseHandler::new();
        let detected =
            handler.detect_edge_case("Rate limit exceeded", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::RateLimit);

        let detected =
            handler.detect_edge_case("HTTP 429 Too Many Requests", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::RateLimit);
    }

    #[test]
    fn test_detect_timeout() {
        let handler = EdgeCaseHandler::new();
        let detected =
            handler.detect_edge_case("Request timeout after 30s", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::Timeout);

        let detected =
            handler.detect_edge_case("Operation timed out", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::Timeout);
    }

    #[test]
    fn test_detect_flaky_test() {
        let handler = EdgeCaseHandler::new();
        let detected =
            handler.detect_edge_case("Flaky test failure", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::FlakyTest);

        let detected = handler.detect_edge_case(
            "Test failed",
            &serde_json::json!({"retry_count": 1}),
        );
        assert_eq!(detected, EdgeCaseType::FlakyTest);
    }

    #[test]
    fn test_detect_service_downtime() {
        let handler = EdgeCaseHandler::new();
        let detected =
            handler.detect_edge_case("Service unavailable", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::ServiceDowntime);

        let detected =
            handler.detect_edge_case("HTTP 503", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::ServiceDowntime);
    }

    #[test]
    fn test_detect_context_overflow() {
        let handler = EdgeCaseHandler::new();
        let detected =
            handler.detect_edge_case("Context too long", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::ContextOverflow);

        let detected = handler.detect_edge_case(
            "Request",
            &serde_json::json!({"token_count": 150000}),
        );
        assert_eq!(detected, EdgeCaseType::ContextOverflow);
    }

    #[test]
    fn test_detect_auth_error() {
        let handler = EdgeCaseHandler::new();
        let detected =
            handler.detect_edge_case("Unauthorized access", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::AuthError);

        let detected =
            handler.detect_edge_case("403 Forbidden", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::AuthError);
    }

    #[test]
    fn test_detect_network_error() {
        let handler = EdgeCaseHandler::new();
        let detected =
            handler.detect_edge_case("Network connection failed", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::NetworkError);

        let detected =
            handler.detect_edge_case("DNS resolution failed", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::NetworkError);
    }

    #[test]
    fn test_detect_unknown() {
        let handler = EdgeCaseHandler::new();
        let detected =
            handler.detect_edge_case("Some random error", &serde_json::json!({}));
        assert_eq!(detected, EdgeCaseType::Unknown);
    }

    // ==================== EdgeCaseHandler Handling Tests ====================

    #[test]
    fn test_handle_flaky_test_retry() {
        let mut handler = EdgeCaseHandler::new();
        let mut event = EdgeCaseEvent::new(EdgeCaseType::FlakyTest)
            .with_session("session-1")
            .with_agent("agent-1");

        let result = handler.handle(&mut event);

        assert!(result.should_continue);
        match result.action {
            EdgeCaseAction::Retry { max_retries, .. } => {
                assert_eq!(max_retries, 3);
            }
            _ => panic!("Expected Retry action"),
        }
        assert_eq!(event.retry_count, 1);
    }

    #[test]
    fn test_handle_flaky_test_escalate_after_max_retries() {
        let mut handler = EdgeCaseHandler::new();
        let mut event = EdgeCaseEvent::new(EdgeCaseType::FlakyTest)
            .with_session("session-1")
            .with_agent("agent-1");

        // Exhaust retries
        for _ in 0..4 {
            handler.handle(&mut event);
        }

        let result = handler.handle(&mut event);
        assert!(!result.should_continue);
        match result.action {
            EdgeCaseAction::Escalate { severity, .. } => {
                assert_eq!(severity, "medium");
            }
            _ => panic!("Expected Escalate action"),
        }
    }

    #[test]
    fn test_handle_merge_conflict() {
        let mut handler = EdgeCaseHandler::new();
        let mut event = EdgeCaseEvent::new(EdgeCaseType::MergeConflict);

        let result = handler.handle(&mut event);

        assert!(result.should_continue);
        match result.action {
            EdgeCaseAction::SpawnResolver { resolver_type } => {
                assert_eq!(resolver_type, "conflict_resolver");
            }
            _ => panic!("Expected SpawnResolver action"),
        }
    }

    #[test]
    fn test_handle_rate_limit_backoff() {
        let mut handler = EdgeCaseHandler::new();
        let mut event = EdgeCaseEvent::new(EdgeCaseType::RateLimit)
            .with_session("session-1")
            .with_agent("agent-1");

        let result = handler.handle(&mut event);

        assert!(result.should_continue);
        match result.action {
            EdgeCaseAction::Backoff { initial_delay, .. } => {
                assert!(initial_delay.num_seconds() > 0);
            }
            _ => panic!("Expected Backoff action"),
        }
        assert_eq!(event.resolution, EdgeCaseResolution::Waiting);
    }

    #[test]
    fn test_handle_auth_error_immediate_escalation() {
        let mut handler = EdgeCaseHandler::new();
        let mut event = EdgeCaseEvent::new(EdgeCaseType::AuthError);

        let result = handler.handle(&mut event);

        assert!(!result.should_continue);
        match result.action {
            EdgeCaseAction::Escalate { severity, .. } => {
                assert_eq!(severity, "critical");
            }
            _ => panic!("Expected Escalate action"),
        }
        assert_eq!(event.resolution, EdgeCaseResolution::Failed);
    }

    #[test]
    fn test_handle_dependency_failure() {
        let mut handler = EdgeCaseHandler::new();
        let mut event = EdgeCaseEvent::new(EdgeCaseType::DependencyFailure);

        let result = handler.handle(&mut event);

        assert!(!result.should_continue);
        match result.action {
            EdgeCaseAction::Block { reason } => {
                assert!(reason.contains("Dependent"));
            }
            _ => panic!("Expected Block action"),
        }
    }

    #[test]
    fn test_handle_context_overflow() {
        let mut handler = EdgeCaseHandler::new();
        let mut event = EdgeCaseEvent::new(EdgeCaseType::ContextOverflow);

        let result = handler.handle(&mut event);

        assert!(result.should_continue);
        match result.action {
            EdgeCaseAction::Summarize { max_tokens } => {
                assert!(max_tokens > 0);
            }
            _ => panic!("Expected Summarize action"),
        }
    }

    #[test]
    fn test_handle_review_ping_pong() {
        let mut handler = EdgeCaseHandler::new();
        let mut event = EdgeCaseEvent::new(EdgeCaseType::ReviewPingPong)
            .with_session("session-1")
            .with_agent("agent-1");

        // First few iterations should continue
        for _ in 0..4 {
            let result = handler.handle(&mut event);
            assert!(result.should_continue);
        }

        // After threshold, should escalate
        let result = handler.handle(&mut event);
        assert!(!result.should_continue);
        match result.action {
            EdgeCaseAction::Escalate { severity, .. } => {
                assert_eq!(severity, "high");
            }
            _ => panic!("Expected Escalate action"),
        }
    }

    // ==================== EdgeCaseHandler Management Tests ====================

    #[test]
    fn test_reset_retries() {
        let mut handler = EdgeCaseHandler::new();
        let mut event = EdgeCaseEvent::new(EdgeCaseType::FlakyTest)
            .with_session("session-1")
            .with_agent("agent-1");

        handler.handle(&mut event);
        handler.handle(&mut event);

        let stats = handler.get_stats();
        assert!(stats.active_retries > 0);

        handler.reset_retries(Some("session-1"), Some("agent-1"));

        let stats = handler.get_stats();
        assert_eq!(stats.active_retries, 0);
    }

    // ==================== EdgeCaseLearning Tests ====================

    #[test]
    fn test_edge_case_learning_new() {
        let learning = EdgeCaseLearning::new(EdgeCaseType::FlakyTest, "test.*failed.*retry");

        assert_eq!(learning.edge_case_type, EdgeCaseType::FlakyTest);
        assert_eq!(learning.pattern, "test.*failed.*retry");
        assert_eq!(learning.occurrence_count, 1);
        assert_eq!(learning.success_rate, 0.0);
    }

    #[test]
    fn test_edge_case_learning_record_occurrence() {
        let mut learning = EdgeCaseLearning::new(EdgeCaseType::FlakyTest, "test pattern");

        learning.record_occurrence(true, Some(30.0));
        assert_eq!(learning.occurrence_count, 2);
        assert_eq!(learning.success_rate, 0.5); // 1 success out of 2
        // First time measurement is just recorded as-is
        assert_eq!(learning.avg_resolution_time_seconds, Some(30.0));

        learning.record_occurrence(true, Some(20.0));
        assert_eq!(learning.occurrence_count, 3);
        assert!((learning.success_rate - 0.666).abs() < 0.01);

        // Check average resolution time: (30.0 * 2 + 20.0) / 3 = 26.67
        // The formula uses occurrence_count-1 as the weight for previous avg
        let avg_time = learning.avg_resolution_time_seconds.unwrap();
        assert!((avg_time - 26.67).abs() < 0.1);
    }

    // ==================== EdgeCaseConfig Tests ====================

    #[test]
    fn test_config_default() {
        let config = EdgeCaseConfig::default();

        assert_eq!(config.flaky_test_max_retries, 3);
        assert_eq!(config.flaky_test_backoff_seconds, 30);
        assert_eq!(config.delayed_review_max_wait_minutes, 120);
        assert_eq!(config.review_ping_pong_threshold, 5);
        assert_eq!(config.context_overflow_threshold, 100_000);
        assert!(config.auto_retry_network_errors);
    }

    // ==================== EdgeCaseAction Display Tests ====================

    #[test]
    fn test_edge_case_action_display() {
        let action = EdgeCaseAction::Retry {
            max_retries: 3,
            backoff_seconds: 30,
        };
        assert_eq!(format!("{}", action), "retry(max=3, backoff=30s)");

        let action = EdgeCaseAction::SpawnResolver {
            resolver_type: "conflict_resolver".to_string(),
        };
        assert_eq!(format!("{}", action), "spawn_resolver(conflict_resolver)");

        let action = EdgeCaseAction::Log;
        assert_eq!(format!("{}", action), "log");
    }
}
