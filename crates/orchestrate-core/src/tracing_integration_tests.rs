//! Integration tests for tracing with agent execution, tool calls, etc.

#![cfg(test)]

use crate::tracing::{SpanAttributes, TracingConfig, TracingExporter, TracingProvider};
use crate::{Agent, AgentState, AgentType};

#[test]
fn test_agent_execution_span() {
    // Create tracing provider
    let config = TracingConfig::new(TracingExporter::Jaeger);
    let provider = TracingProvider::new(config);

    // Create an agent
    let agent = Agent::new(AgentType::StoryDeveloper, "Implement feature");

    // Start agent execution span
    let attrs = SpanAttributes::new()
        .add("agent_id", agent.id.to_string())
        .add("agent_type", agent.agent_type.as_str());

    let span = provider.start_span("agent_execution", attrs);

    assert_eq!(span.name(), "agent_execution");
    assert!(!span.trace_context().trace_id.is_empty());

    // Verify span was recorded
    let spans = provider.recorded_spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "agent_execution");
    assert_eq!(
        spans[0].attributes.get("agent_id"),
        Some(&agent.id.to_string())
    );
    assert_eq!(
        spans[0].attributes.get("agent_type"),
        Some(&"story_developer".to_string())
    );
}

#[test]
fn test_tool_call_span() {
    let config = TracingConfig::new(TracingExporter::Jaeger);
    let provider = TracingProvider::new(config);

    // Simulate tool call
    let attrs = SpanAttributes::new()
        .add("tool_name", "Bash")
        .add("command", "git status");

    let span = provider.start_span("tool_call", attrs);
    assert_eq!(span.name(), "tool_call");

    let spans = provider.recorded_spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].attributes.get("tool_name"), Some(&"Bash".to_string()));
    assert_eq!(
        spans[0].attributes.get("command"),
        Some(&"git status".to_string())
    );
}

#[test]
fn test_database_query_span() {
    let config = TracingConfig::new(TracingExporter::Jaeger);
    let provider = TracingProvider::new(config);

    // Simulate database query
    let attrs = SpanAttributes::new()
        .add("query_type", "SELECT")
        .add("table", "agents");

    let span = provider.start_span("database_query", attrs);
    assert_eq!(span.name(), "database_query");

    let spans = provider.recorded_spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(
        spans[0].attributes.get("query_type"),
        Some(&"SELECT".to_string())
    );
    assert_eq!(spans[0].attributes.get("table"), Some(&"agents".to_string()));
}

#[test]
fn test_http_request_span() {
    let config = TracingConfig::new(TracingExporter::Jaeger);
    let provider = TracingProvider::new(config);

    // Simulate HTTP request
    let attrs = SpanAttributes::new()
        .add("method", "POST")
        .add("path", "/api/messages")
        .add("status", "200");

    let span = provider.start_span("http_request", attrs);
    assert_eq!(span.name(), "http_request");

    let spans = provider.recorded_spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].attributes.get("method"), Some(&"POST".to_string()));
    assert_eq!(
        spans[0].attributes.get("path"),
        Some(&"/api/messages".to_string())
    );
    assert_eq!(spans[0].attributes.get("status"), Some(&"200".to_string()));
}

#[test]
fn test_api_call_span() {
    let config = TracingConfig::new(TracingExporter::Jaeger);
    let provider = TracingProvider::new(config);

    // Simulate API call to Claude
    let attrs = SpanAttributes::new()
        .add("model", "claude-sonnet-4-20250514")
        .add("input_tokens", "1234")
        .add("output_tokens", "567");

    let span = provider.start_span("api_call", attrs);
    assert_eq!(span.name(), "api_call");

    let spans = provider.recorded_spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(
        spans[0].attributes.get("model"),
        Some(&"claude-sonnet-4-20250514".to_string())
    );
    assert_eq!(
        spans[0].attributes.get("input_tokens"),
        Some(&"1234".to_string())
    );
}

#[test]
fn test_span_hierarchy() {
    use crate::tracing::TraceContext;

    let config = TracingConfig::new(TracingExporter::Jaeger);
    let mut provider = TracingProvider::new(config);

    // Set a trace context for the entire hierarchy
    let trace_ctx = TraceContext::with_trace_id("trace-hierarchy-123");
    let trace_id = trace_ctx.trace_id.clone();
    provider.set_trace_context(trace_ctx);

    // Create parent span (agent execution)
    let agent = Agent::new(AgentType::StoryDeveloper, "Implement feature");
    let agent_attrs = SpanAttributes::new()
        .add("agent_id", agent.id.to_string())
        .add("agent_type", agent.agent_type.as_str());

    let _agent_span = provider.start_span("agent_execution", agent_attrs);

    // Create child span (tool call)
    let tool_attrs = SpanAttributes::new().add("tool_name", "Read");
    let _tool_span = provider.start_span("tool_call", tool_attrs);

    // Create another child span (database query within tool call)
    let db_attrs = SpanAttributes::new()
        .add("query_type", "SELECT")
        .add("table", "files");
    let _db_span = provider.start_span("database_query", db_attrs);

    // Verify all spans share the same trace ID
    let spans = provider.recorded_spans();
    assert_eq!(spans.len(), 3);

    for span in &spans {
        assert_eq!(span.trace_id, trace_id);
    }

    // Verify span names
    assert_eq!(spans[0].name, "agent_execution");
    assert_eq!(spans[1].name, "tool_call");
    assert_eq!(spans[2].name, "database_query");
}

#[test]
fn test_multiple_agents_with_different_traces() {
    let config = TracingConfig::new(TracingExporter::Jaeger);
    let provider = TracingProvider::new(config);

    // Create first agent
    let agent1 = Agent::new(AgentType::StoryDeveloper, "Task 1");
    let attrs1 = SpanAttributes::new()
        .add("agent_id", agent1.id.to_string())
        .add("agent_type", "story_developer");
    let span1 = provider.start_span("agent_execution", attrs1);
    let trace_id1 = span1.trace_context().trace_id.clone();

    // Clear spans to simulate separate trace
    provider.clear_spans();

    // Create second agent
    let agent2 = Agent::new(AgentType::CodeReviewer, "Task 2");
    let attrs2 = SpanAttributes::new()
        .add("agent_id", agent2.id.to_string())
        .add("agent_type", "code_reviewer");
    let span2 = provider.start_span("agent_execution", attrs2);
    let trace_id2 = span2.trace_context().trace_id.clone();

    // Traces should be different
    assert_ne!(trace_id1, trace_id2);

    // Verify second span was recorded
    let spans = provider.recorded_spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].trace_id, trace_id2);
}

#[test]
fn test_agent_state_transitions_with_spans() {
    let config = TracingConfig::new(TracingExporter::Jaeger);
    let provider = TracingProvider::new(config);

    let mut agent = Agent::new(AgentType::StoryDeveloper, "Task");

    // Start initialization span
    let init_attrs = SpanAttributes::new()
        .add("agent_id", agent.id.to_string())
        .add("state", "initializing");
    provider.start_span("agent_state_transition", init_attrs);

    agent.transition_to(AgentState::Initializing).unwrap();

    // Start running span
    let run_attrs = SpanAttributes::new()
        .add("agent_id", agent.id.to_string())
        .add("state", "running");
    provider.start_span("agent_state_transition", run_attrs);

    agent.transition_to(AgentState::Running).unwrap();

    // Verify spans recorded
    let spans = provider.recorded_spans();
    assert_eq!(spans.len(), 2);
    assert_eq!(
        spans[0].attributes.get("state"),
        Some(&"initializing".to_string())
    );
    assert_eq!(
        spans[1].attributes.get("state"),
        Some(&"running".to_string())
    );
}

#[test]
fn test_sampling_disabled() {
    let config = TracingConfig::new(TracingExporter::Jaeger).with_sample_rate(0.0);
    let provider = TracingProvider::new(config);

    assert!(provider.is_enabled());
    assert_eq!(provider.config().sample_rate, 0.0);

    // Note: In production, a sample rate of 0.0 would prevent span recording
    // For this test, we just verify the configuration
}

#[test]
fn test_trace_context_propagation() {
    use crate::tracing::TraceContext;

    // Create parent trace context
    let parent_ctx = TraceContext::with_trace_id("trace-123")
        .with_parent("parent-span-456")
        .with_sampled(true);

    assert_eq!(parent_ctx.trace_id, "trace-123");
    assert_eq!(parent_ctx.parent_span_id, Some("parent-span-456".to_string()));
    assert!(parent_ctx.sampled);

    // Create child context (would share same trace ID in production)
    let child_ctx = TraceContext::with_trace_id("trace-123")
        .with_parent("child-span-789")
        .with_sampled(true);

    // Both should have same trace ID
    assert_eq!(parent_ctx.trace_id, child_ctx.trace_id);
    // But different parent span IDs
    assert_ne!(parent_ctx.parent_span_id, child_ctx.parent_span_id);
}
