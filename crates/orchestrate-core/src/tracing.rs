//! OpenTelemetry distributed tracing support
//!
//! Provides instrumentation for agent execution, tool calls, database queries,
//! and HTTP requests with support for multiple trace exporters (Jaeger, OTLP).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[cfg(test)]
use std::sync::Arc;

/// Tracing exporter types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TracingExporter {
    /// Export to Jaeger
    Jaeger,
    /// Export to OTLP endpoint
    Otlp,
    /// No tracing (disabled)
    None,
}

impl TracingExporter {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TracingExporter::Jaeger => "jaeger",
            TracingExporter::Otlp => "otlp",
            TracingExporter::None => "none",
        }
    }

    /// Parse from string representation
    pub fn from_str(s: &str) -> crate::Result<Self> {
        match s.to_lowercase().as_str() {
            "jaeger" => Ok(TracingExporter::Jaeger),
            "otlp" => Ok(TracingExporter::Otlp),
            "none" => Ok(TracingExporter::None),
            _ => Err(crate::Error::Other(format!(
                "Unknown tracing exporter: {}",
                s
            ))),
        }
    }
}

/// Tracing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Exporter type
    pub exporter: TracingExporter,
    /// Jaeger endpoint (e.g., "http://localhost:14268/api/traces")
    pub jaeger_endpoint: Option<String>,
    /// OTLP endpoint (e.g., "http://localhost:4317")
    pub otlp_endpoint: Option<String>,
    /// Service name for traces
    pub service_name: String,
    /// Sample rate (0.0 to 1.0, 1.0 = 100%)
    pub sample_rate: f64,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            exporter: TracingExporter::None,
            jaeger_endpoint: Some("http://localhost:14268/api/traces".to_string()),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            service_name: "orchestrate".to_string(),
            sample_rate: 1.0,
        }
    }
}

impl TracingConfig {
    /// Create new tracing config with exporter
    pub fn new(exporter: TracingExporter) -> Self {
        Self {
            exporter,
            ..Default::default()
        }
    }

    /// Set service name
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    /// Set sample rate
    pub fn with_sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Set Jaeger endpoint
    pub fn with_jaeger_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.jaeger_endpoint = Some(endpoint.into());
        self
    }

    /// Set OTLP endpoint
    pub fn with_otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = Some(endpoint.into());
        self
    }
}

/// Span attributes for tracing
#[derive(Debug, Clone, Default)]
pub struct SpanAttributes {
    attributes: HashMap<String, String>,
}

impl SpanAttributes {
    /// Create new span attributes
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an attribute
    pub fn add(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }

    /// Get an attribute
    pub fn get(&self, key: &str) -> Option<&str> {
        self.attributes.get(key).map(|s| s.as_str())
    }

    /// Get all attributes
    pub fn all(&self) -> &HashMap<String, String> {
        &self.attributes
    }
}

/// Trace context for propagation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// Trace ID
    pub trace_id: String,
    /// Parent span ID
    pub parent_span_id: Option<String>,
    /// Sampling decision
    pub sampled: bool,
}

impl TraceContext {
    /// Create new trace context
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::new_v4().to_string(),
            parent_span_id: None,
            sampled: true,
        }
    }

    /// Create trace context with specific trace ID
    pub fn with_trace_id(trace_id: impl Into<String>) -> Self {
        Self {
            trace_id: trace_id.into(),
            parent_span_id: None,
            sampled: true,
        }
    }

    /// Set parent span ID
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(parent_id.into());
        self
    }

    /// Set sampling decision
    pub fn with_sampled(mut self, sampled: bool) -> Self {
        self.sampled = sampled;
        self
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Span recorder for testing
#[derive(Debug, Clone)]
pub struct SpanRecord {
    pub name: String,
    pub attributes: HashMap<String, String>,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
}

/// Tracing provider for managing trace exports
pub struct TracingProvider {
    config: TracingConfig,
    trace_context: Option<TraceContext>,
    // For testing - store recorded spans
    #[cfg(test)]
    recorded_spans: Arc<std::sync::Mutex<Vec<SpanRecord>>>,
}

impl TracingProvider {
    /// Create new tracing provider
    pub fn new(config: TracingConfig) -> Self {
        Self {
            config,
            trace_context: None,
            #[cfg(test)]
            recorded_spans: Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }

    /// Set the trace context for this provider
    pub fn set_trace_context(&mut self, ctx: TraceContext) {
        self.trace_context = Some(ctx);
    }

    /// Get the current trace context
    pub fn trace_context(&self) -> Option<&TraceContext> {
        self.trace_context.as_ref()
    }

    /// Initialize the tracing provider
    pub fn init(&mut self) -> crate::Result<()> {
        // For now, just validate the configuration
        match self.config.exporter {
            TracingExporter::Jaeger => {
                if self.config.jaeger_endpoint.is_none() {
                    return Err(crate::Error::Other(
                        "Jaeger endpoint not configured".to_string(),
                    ));
                }
            }
            TracingExporter::Otlp => {
                if self.config.otlp_endpoint.is_none() {
                    return Err(crate::Error::Other(
                        "OTLP endpoint not configured".to_string(),
                    ));
                }
            }
            TracingExporter::None => {}
        }

        Ok(())
    }

    /// Get the current configuration
    pub fn config(&self) -> &TracingConfig {
        &self.config
    }

    /// Check if tracing is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.exporter != TracingExporter::None
    }

    /// Create a new trace context
    pub fn new_trace_context(&self) -> TraceContext {
        TraceContext::new()
    }

    /// Start a span (stub for now)
    pub fn start_span(&self, name: impl Into<String>, attributes: SpanAttributes) -> Span {
        let name = name.into();
        let trace_ctx = self.trace_context.as_ref().cloned().unwrap_or_default();

        #[cfg(test)]
        {
            let record = SpanRecord {
                name: name.clone(),
                attributes: attributes.all().clone(),
                trace_id: trace_ctx.trace_id.clone(),
                parent_span_id: trace_ctx.parent_span_id.clone(),
            };
            self.recorded_spans.lock().unwrap().push(record);
        }

        Span {
            name,
            attributes,
            trace_context: trace_ctx,
        }
    }

    /// Get recorded spans (for testing)
    #[cfg(test)]
    pub fn recorded_spans(&self) -> Vec<SpanRecord> {
        self.recorded_spans.lock().unwrap().clone()
    }

    /// Clear recorded spans (for testing)
    #[cfg(test)]
    pub fn clear_spans(&self) {
        self.recorded_spans.lock().unwrap().clear();
    }
}

/// A tracing span
pub struct Span {
    name: String,
    #[allow(dead_code)]
    attributes: SpanAttributes,
    trace_context: TraceContext,
}

impl Span {
    /// Get span name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get trace context
    pub fn trace_context(&self) -> &TraceContext {
        &self.trace_context
    }

    /// End the span (stub for now)
    pub fn end(self) {
        // Span will be automatically ended when dropped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracing_exporter_as_str() {
        assert_eq!(TracingExporter::Jaeger.as_str(), "jaeger");
        assert_eq!(TracingExporter::Otlp.as_str(), "otlp");
        assert_eq!(TracingExporter::None.as_str(), "none");
    }

    #[test]
    fn test_tracing_exporter_from_str() {
        assert_eq!(
            TracingExporter::from_str("jaeger").unwrap(),
            TracingExporter::Jaeger
        );
        assert_eq!(
            TracingExporter::from_str("OTLP").unwrap(),
            TracingExporter::Otlp
        );
        assert_eq!(
            TracingExporter::from_str("none").unwrap(),
            TracingExporter::None
        );
        assert!(TracingExporter::from_str("invalid").is_err());
    }

    #[test]
    fn test_tracing_config_default() {
        let config = TracingConfig::default();
        assert_eq!(config.exporter, TracingExporter::None);
        assert_eq!(config.service_name, "orchestrate");
        assert_eq!(config.sample_rate, 1.0);
        assert!(config.jaeger_endpoint.is_some());
        assert!(config.otlp_endpoint.is_some());
    }

    #[test]
    fn test_tracing_config_builder() {
        let config = TracingConfig::new(TracingExporter::Jaeger)
            .with_service_name("test-service")
            .with_sample_rate(0.5)
            .with_jaeger_endpoint("http://custom:14268/api/traces");

        assert_eq!(config.exporter, TracingExporter::Jaeger);
        assert_eq!(config.service_name, "test-service");
        assert_eq!(config.sample_rate, 0.5);
        assert_eq!(
            config.jaeger_endpoint.unwrap(),
            "http://custom:14268/api/traces"
        );
    }

    #[test]
    fn test_tracing_config_sample_rate_clamping() {
        let config1 = TracingConfig::default().with_sample_rate(1.5);
        assert_eq!(config1.sample_rate, 1.0);

        let config2 = TracingConfig::default().with_sample_rate(-0.1);
        assert_eq!(config2.sample_rate, 0.0);
    }

    #[test]
    fn test_span_attributes() {
        let attrs = SpanAttributes::new()
            .add("agent_id", "123")
            .add("agent_type", "story_developer");

        assert_eq!(attrs.get("agent_id"), Some("123"));
        assert_eq!(attrs.get("agent_type"), Some("story_developer"));
        assert_eq!(attrs.get("nonexistent"), None);
        assert_eq!(attrs.all().len(), 2);
    }

    #[test]
    fn test_trace_context_creation() {
        let ctx = TraceContext::new();
        assert!(!ctx.trace_id.is_empty());
        assert!(ctx.parent_span_id.is_none());
        assert!(ctx.sampled);
    }

    #[test]
    fn test_trace_context_with_trace_id() {
        let ctx = TraceContext::with_trace_id("custom-trace-id")
            .with_parent("parent-span-id")
            .with_sampled(false);

        assert_eq!(ctx.trace_id, "custom-trace-id");
        assert_eq!(ctx.parent_span_id, Some("parent-span-id".to_string()));
        assert!(!ctx.sampled);
    }

    #[test]
    fn test_tracing_provider_init_jaeger() {
        let config = TracingConfig::new(TracingExporter::Jaeger);
        let mut provider = TracingProvider::new(config);
        assert!(provider.init().is_ok());
    }

    #[test]
    fn test_tracing_provider_init_jaeger_no_endpoint() {
        let mut config = TracingConfig::new(TracingExporter::Jaeger);
        config.jaeger_endpoint = None;
        let mut provider = TracingProvider::new(config);
        assert!(provider.init().is_err());
    }

    #[test]
    fn test_tracing_provider_init_otlp() {
        let config = TracingConfig::new(TracingExporter::Otlp);
        let mut provider = TracingProvider::new(config);
        assert!(provider.init().is_ok());
    }

    #[test]
    fn test_tracing_provider_init_otlp_no_endpoint() {
        let mut config = TracingConfig::new(TracingExporter::Otlp);
        config.otlp_endpoint = None;
        let mut provider = TracingProvider::new(config);
        assert!(provider.init().is_err());
    }

    #[test]
    fn test_tracing_provider_init_none() {
        let config = TracingConfig::new(TracingExporter::None);
        let mut provider = TracingProvider::new(config);
        assert!(provider.init().is_ok());
    }

    #[test]
    fn test_tracing_provider_is_enabled() {
        let provider1 = TracingProvider::new(TracingConfig::new(TracingExporter::Jaeger));
        assert!(provider1.is_enabled());

        let provider2 = TracingProvider::new(TracingConfig::new(TracingExporter::None));
        assert!(!provider2.is_enabled());
    }

    #[test]
    fn test_tracing_provider_new_trace_context() {
        let config = TracingConfig::new(TracingExporter::Jaeger);
        let provider = TracingProvider::new(config);

        let ctx1 = provider.new_trace_context();
        let ctx2 = provider.new_trace_context();

        // Each context should have a unique trace ID
        assert_ne!(ctx1.trace_id, ctx2.trace_id);
    }

    #[test]
    fn test_span_creation() {
        let config = TracingConfig::new(TracingExporter::Jaeger);
        let provider = TracingProvider::new(config);

        let attrs = SpanAttributes::new()
            .add("agent_id", "test-123")
            .add("agent_type", "story_developer");

        let span = provider.start_span("agent_execution", attrs);
        assert_eq!(span.name(), "agent_execution");
        assert!(!span.trace_context().trace_id.is_empty());
    }

    #[test]
    fn test_span_recording() {
        let config = TracingConfig::new(TracingExporter::Jaeger);
        let provider = TracingProvider::new(config);

        let attrs1 = SpanAttributes::new().add("tool_name", "Bash");
        let _span1 = provider.start_span("tool_call", attrs1);

        let attrs2 = SpanAttributes::new().add("query_type", "SELECT");
        let _span2 = provider.start_span("database_query", attrs2);

        let spans = provider.recorded_spans();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].name, "tool_call");
        assert_eq!(spans[0].attributes.get("tool_name"), Some(&"Bash".to_string()));
        assert_eq!(spans[1].name, "database_query");
        assert_eq!(spans[1].attributes.get("query_type"), Some(&"SELECT".to_string()));
    }

    #[test]
    fn test_clear_spans() {
        let config = TracingConfig::new(TracingExporter::Jaeger);
        let provider = TracingProvider::new(config);

        let attrs = SpanAttributes::new().add("test", "value");
        let _span = provider.start_span("test_span", attrs);

        assert_eq!(provider.recorded_spans().len(), 1);

        provider.clear_spans();
        assert_eq!(provider.recorded_spans().len(), 0);
    }
}
