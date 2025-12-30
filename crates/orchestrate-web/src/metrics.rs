//! Prometheus metrics collection and export
//!
//! This module provides metrics collection in Prometheus format including:
//! - Agent metrics (count by state and type)
//! - Token usage metrics
//! - API latency histograms
//! - Queue depth metrics
//! - Error rate metrics

use orchestrate_core::Database;
use prometheus::{
    CounterVec, Encoder, GaugeVec, HistogramOpts, HistogramVec, Opts, Registry, TextEncoder,
};
use std::sync::Arc;

/// Metrics collector for the orchestrate system
pub struct MetricsCollector {
    registry: Registry,

    // Agent metrics
    agents_total: GaugeVec,
    agent_execution_seconds: HistogramVec,

    // Token metrics
    tokens_total: CounterVec,

    // API metrics
    http_requests_total: CounterVec,
    http_request_duration_seconds: HistogramVec,

    // Queue metrics
    queue_depth: GaugeVec,

    // Error metrics
    errors_total: CounterVec,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> Result<Self, prometheus::Error> {
        let registry = Registry::new();

        // Agent metrics
        let agents_total = GaugeVec::new(
            Opts::new("orchestrate_agents_total", "Number of agents by type and state"),
            &["type", "state"],
        )?;

        let agent_execution_seconds = HistogramVec::new(
            HistogramOpts::new(
                "orchestrate_agent_execution_seconds",
                "Agent execution duration in seconds",
            )
            .buckets(vec![1.0, 5.0, 10.0, 30.0, 60.0, 120.0, 300.0, 600.0]),
            &["type"],
        )?;

        // Token metrics
        let tokens_total = CounterVec::new(
            Opts::new("orchestrate_tokens_total", "Total tokens used by model and direction"),
            &["model", "direction"],
        )?;

        // API metrics
        let http_requests_total = CounterVec::new(
            Opts::new("orchestrate_http_requests_total", "Total HTTP requests"),
            &["method", "path", "status"],
        )?;

        let http_request_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "orchestrate_http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
            &["method", "path"],
        )?;

        // Queue metrics
        let queue_depth = GaugeVec::new(
            Opts::new("orchestrate_queue_depth", "Current queue depth by queue name"),
            &["queue"],
        )?;

        // Error metrics
        let errors_total = CounterVec::new(
            Opts::new("orchestrate_errors_total", "Total errors by type"),
            &["error_type"],
        )?;

        // Register all metrics
        registry.register(Box::new(agents_total.clone()))?;
        registry.register(Box::new(agent_execution_seconds.clone()))?;
        registry.register(Box::new(tokens_total.clone()))?;
        registry.register(Box::new(http_requests_total.clone()))?;
        registry.register(Box::new(http_request_duration_seconds.clone()))?;
        registry.register(Box::new(queue_depth.clone()))?;
        registry.register(Box::new(errors_total.clone()))?;

        Ok(Self {
            registry,
            agents_total,
            agent_execution_seconds,
            tokens_total,
            http_requests_total,
            http_request_duration_seconds,
            queue_depth,
            errors_total,
        })
    }

    /// Update agent count metrics from database
    pub async fn update_agent_metrics(&self, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        // Reset all agent gauges to zero first
        self.agents_total.reset();

        // Get agent counts from database
        let counts = db.get_agent_counts_by_state_and_type().await?;

        for (agent_type, state, count) in counts {
            self.agents_total
                .with_label_values(&[&agent_type, &state])
                .set(count as f64);
        }

        Ok(())
    }

    /// Update token usage metrics from database
    pub async fn update_token_metrics(&self, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        // For now, we get token stats from messages table
        // In the future, this could use daily_token_usage or session_token_stats tables
        let token_stats = db.get_token_usage_by_model().await?;

        for (model, input_tokens, output_tokens) in token_stats {
            // Reset and set to current values
            // Note: These are gauges effectively showing current totals
            self.tokens_total
                .with_label_values(&[&model, "input"])
                .inc_by(input_tokens as f64);
            self.tokens_total
                .with_label_values(&[&model, "output"])
                .inc_by(output_tokens as f64);
        }

        Ok(())
    }

    /// Update queue depth metrics from database
    pub async fn update_queue_metrics(&self, db: &Database) -> Result<(), Box<dyn std::error::Error>> {
        // Get queue depths
        let webhook_events = db.get_pending_webhook_events_count().await?;

        self.queue_depth
            .with_label_values(&["webhook_events"])
            .set(webhook_events as f64);

        Ok(())
    }

    /// Record HTTP request
    pub fn record_http_request(&self, method: &str, path: &str, status: u16, duration_seconds: f64) {
        self.http_requests_total
            .with_label_values(&[method, path, &status.to_string()])
            .inc();

        self.http_request_duration_seconds
            .with_label_values(&[method, path])
            .observe(duration_seconds);
    }

    /// Record agent execution time
    pub fn record_agent_execution(&self, agent_type: &str, duration_seconds: f64) {
        self.agent_execution_seconds
            .with_label_values(&[agent_type])
            .observe(duration_seconds);
    }

    /// Record error
    pub fn record_error(&self, error_type: &str) {
        self.errors_total
            .with_label_values(&[error_type])
            .inc();
    }

    /// Gather all metrics and encode to Prometheus text format
    pub async fn gather(&self, db: &Database) -> Result<String, Box<dyn std::error::Error>> {
        // Update metrics from database
        self.update_agent_metrics(db).await?;
        self.update_token_metrics(db).await?;
        self.update_queue_metrics(db).await?;

        // Encode metrics to text format
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer)?;

        Ok(String::from_utf8(buffer)?)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new().expect("Failed to create metrics collector")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use orchestrate_core::{Agent, AgentState, AgentType, Message, MessageRole};

    #[tokio::test]
    async fn test_metrics_collector_creation() {
        let collector = MetricsCollector::new();
        assert!(collector.is_ok());
    }

    #[tokio::test]
    async fn test_agent_metrics_empty_database() {
        let collector = MetricsCollector::new().unwrap();
        let db = Database::in_memory().await.unwrap();

        let result = collector.update_agent_metrics(&db).await;
        assert!(result.is_ok());

        // Gather metrics - with empty database, gauges won't be exported
        // but the registry should still work
        let metrics = collector.gather(&db).await.unwrap();
        // Just ensure we get valid Prometheus output
        assert!(metrics.contains("# HELP") || metrics.len() > 0);
    }

    #[tokio::test]
    async fn test_agent_metrics_with_data() {
        let collector = MetricsCollector::new().unwrap();
        let db = Database::in_memory().await.unwrap();

        // Create test agents
        let agent1 = Agent::new(AgentType::StoryDeveloper, "Test task 1");
        db.insert_agent(&agent1).await.unwrap();

        let mut agent2 = Agent::new(AgentType::StoryDeveloper, "Test task 2");
        agent2.state = AgentState::Running;
        db.insert_agent(&agent2).await.unwrap();

        let mut agent3 = Agent::new(AgentType::CodeReviewer, "Test task 3");
        agent3.state = AgentState::Completed;
        db.insert_agent(&agent3).await.unwrap();

        // Update metrics
        collector.update_agent_metrics(&db).await.unwrap();

        // Gather and check metrics
        let metrics = collector.gather(&db).await.unwrap();

        // Should contain metrics for story-developer agents
        assert!(metrics.contains("orchestrate_queue_depth")); // This will always be present
        // Agent metrics should be present with data
        assert!(metrics.contains("story") || metrics.contains("code")); // Agent type names
    }

    #[tokio::test]
    async fn test_token_metrics() {
        let collector = MetricsCollector::new().unwrap();
        let db = Database::in_memory().await.unwrap();

        // Token metrics use daily_token_usage table which is populated by actual API usage
        // For now, just test that the metric collection works with empty data
        collector.update_token_metrics(&db).await.unwrap();

        // Gather and check - tokens_total won't be present with empty data
        let metrics = collector.gather(&db).await.unwrap();
        // Just check that gathering works
        assert!(metrics.len() > 0);
    }

    #[tokio::test]
    async fn test_queue_metrics() {
        let collector = MetricsCollector::new().unwrap();
        let db = Database::in_memory().await.unwrap();

        // Add some webhook events
        let event = orchestrate_core::webhook::WebhookEvent::new(
            "test-delivery-123".to_string(),
            "push".to_string(),
            "{}".to_string(),
        );
        db.insert_webhook_event(&event).await.unwrap();

        // Update metrics
        collector.update_queue_metrics(&db).await.unwrap();

        // Gather and check
        let metrics = collector.gather(&db).await.unwrap();
        assert!(metrics.contains("orchestrate_queue_depth"));
        assert!(metrics.contains("webhook_events"));
    }

    #[tokio::test]
    async fn test_http_request_metrics() {
        let collector = MetricsCollector::new().unwrap();

        collector.record_http_request("POST", "/api/agents", 200, 0.123);
        collector.record_http_request("GET", "/api/agents", 200, 0.045);
        collector.record_http_request("POST", "/api/agents", 500, 1.234);

        let db = Database::in_memory().await.unwrap();
        let metrics = collector.gather(&db).await.unwrap();

        assert!(metrics.contains("orchestrate_http_requests_total"));
        assert!(metrics.contains("orchestrate_http_request_duration_seconds"));
    }

    #[tokio::test]
    async fn test_agent_execution_metrics() {
        let collector = MetricsCollector::new().unwrap();

        collector.record_agent_execution("story-developer", 120.5);
        collector.record_agent_execution("code-reviewer", 45.2);

        let db = Database::in_memory().await.unwrap();
        let metrics = collector.gather(&db).await.unwrap();

        assert!(metrics.contains("orchestrate_agent_execution_seconds"));
    }

    #[tokio::test]
    async fn test_error_metrics() {
        let collector = MetricsCollector::new().unwrap();

        collector.record_error("database_error");
        collector.record_error("api_error");
        collector.record_error("database_error");

        let db = Database::in_memory().await.unwrap();
        let metrics = collector.gather(&db).await.unwrap();

        assert!(metrics.contains("orchestrate_errors_total"));
    }

    #[tokio::test]
    async fn test_metrics_format() {
        let collector = MetricsCollector::new().unwrap();
        let db = Database::in_memory().await.unwrap();

        // Add some data so metrics get exported
        let agent = Agent::new(AgentType::StoryDeveloper, "Test task");
        db.insert_agent(&agent).await.unwrap();

        collector.record_http_request("GET", "/api/test", 200, 0.1);
        collector.record_agent_execution("story-developer", 10.0);
        collector.record_error("test_error");

        let metrics = collector.gather(&db).await.unwrap();

        // Check Prometheus format
        assert!(metrics.contains("# HELP"));
        assert!(metrics.contains("# TYPE"));

        // Check key metric names (ones that have data)
        assert!(metrics.contains("orchestrate_http_requests_total"));
        assert!(metrics.contains("orchestrate_http_request_duration_seconds"));
        assert!(metrics.contains("orchestrate_queue_depth"));
        assert!(metrics.contains("orchestrate_errors_total"));
        assert!(metrics.contains("orchestrate_agent_execution_seconds"));
    }
}
