# Prometheus Metrics Endpoint

The orchestrate-web server exposes Prometheus-compatible metrics at the `/metrics` endpoint.

## Endpoint

```
GET /metrics
```

- **Authentication:** Not required (public endpoint)
- **Content-Type:** text/plain; version=0.0.4

## Metrics Exposed

### Agent Metrics

```prometheus
# Number of agents by type and state
orchestrate_agents_total{type="story-developer",state="running"} 3
orchestrate_agents_total{type="code-reviewer",state="completed"} 5

# Agent execution duration histograms
orchestrate_agent_execution_seconds{type="story-developer",quantile="0.5"} 45.2
orchestrate_agent_execution_seconds{type="story-developer",quantile="0.9"} 120.5
orchestrate_agent_execution_seconds{type="story-developer",quantile="0.99"} 180.3
```

### Token Usage Metrics

```prometheus
# Total tokens used by model and direction
orchestrate_tokens_total{model="claude-3-opus",direction="input"} 1500000
orchestrate_tokens_total{model="claude-3-opus",direction="output"} 500000
orchestrate_tokens_total{model="claude-3-sonnet",direction="input"} 2300000
orchestrate_tokens_total{model="claude-3-sonnet",direction="output"} 800000
```

### HTTP API Metrics

```prometheus
# Total HTTP requests by method, path, and status
orchestrate_http_requests_total{method="POST",path="/api/agents",status="200"} 150
orchestrate_http_requests_total{method="GET",path="/api/agents",status="200"} 423

# HTTP request duration histograms
orchestrate_http_request_duration_seconds{method="POST",path="/api/agents",quantile="0.5"} 0.123
orchestrate_http_request_duration_seconds{method="POST",path="/api/agents",quantile="0.95"} 0.250
orchestrate_http_request_duration_seconds{method="POST",path="/api/agents",quantile="0.99"} 0.450
```

### Queue Depth Metrics

```prometheus
# Current queue depth by queue name
orchestrate_queue_depth{queue="webhook_events"} 5
```

### Error Metrics

```prometheus
# Total errors by type
orchestrate_errors_total{error_type="database_error"} 3
orchestrate_errors_total{error_type="api_error"} 12
```

## Usage Examples

### curl

```bash
curl http://localhost:3000/metrics
```

### Prometheus Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'orchestrate'
    static_configs:
      - targets: ['localhost:3000']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

### Grafana Dashboard

Example PromQL queries for dashboards:

```promql
# Active agents by type
sum by(type) (orchestrate_agents_total{state="running"})

# Total tokens per hour
rate(orchestrate_tokens_total[1h])

# API request rate
rate(orchestrate_http_requests_total[5m])

# 95th percentile API latency
histogram_quantile(0.95, rate(orchestrate_http_request_duration_seconds_bucket[5m]))

# Queue backlog
orchestrate_queue_depth{queue="webhook_events"}

# Error rate
rate(orchestrate_errors_total[5m])
```

## Implementation Details

### Metric Collection

Metrics are collected from the database on each `/metrics` request:
- Agent counts are aggregated from the `agents` table
- Token usage is aggregated from the `daily_token_usage` table
- Queue depth is counted from the `webhook_events` table
- HTTP and execution metrics are collected in-memory

### Performance

The metrics endpoint is designed to be lightweight:
- Database queries use indexes for fast aggregation
- Metrics are gathered only on request (no background polling)
- No authentication overhead (public endpoint)

### Standard Compliance

All metrics follow Prometheus naming conventions:
- Metric names: `orchestrate_<metric>_<unit>`
- Label names: lowercase with underscores
- Units: base units (seconds, bytes, etc.)
- Histograms: include `_bucket`, `_sum`, `_count` suffixes

## Testing

Run the metrics integration tests:

```bash
cargo test -p orchestrate-web --test metrics_integration_test
```

Run all metrics-related tests:

```bash
cargo test -p orchestrate-web metrics
```
