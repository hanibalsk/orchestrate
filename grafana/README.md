# Orchestrate Grafana Dashboards

This directory contains Grafana dashboard templates for monitoring the Orchestrate multi-agent system.

## Dashboard Overview

### 1. Agent Overview (`agent-overview.json`)
Monitors agent status, execution, and performance:
- Total, running, completed, and failed agent counts
- Agent distribution by type and state over time
- Execution duration percentiles (p50, p95)
- Agent success rates with gauges
- Agent error rates

**Key Metrics:**
- `orchestrate_agents_total{type, state}`
- `orchestrate_agent_execution_seconds{type}`
- `orchestrate_agent_success_rate{agent_type}`
- `orchestrate_errors_total{error_type}`

**Template Variables:**
- `agent_type` - Filter by agent type (story-developer, code-reviewer, etc.)

### 2. Token Usage (`token-usage.json`)
Tracks LLM API token consumption and trends:
- Total, input, and output token counts
- Token usage rate by model
- Distribution by model (pie/donut charts)
- Hourly consumption trends
- Detailed breakdown table

**Key Metrics:**
- `orchestrate_tokens_total{model, direction}`

**Template Variables:**
- `model` - Filter by AI model (claude-sonnet-4-5, etc.)

### 3. Alerts & Monitoring (`alerts.json`)
Centralized view of system health and issues:
- Active alerts count
- Error rate monitoring
- Queue depth tracking
- Failed agent counts
- Error distribution and trends
- Agent success vs failure comparison
- Success rate trends over time

**Key Metrics:**
- `ALERTS{alertstate}`
- `orchestrate_errors_total{error_type}`
- `orchestrate_queue_depth{queue}`
- `orchestrate_agents_total{state}`
- `orchestrate_agent_success_rate`

**Template Variables:**
- `error_type` - Filter by error type
- `queue` - Filter by queue name

### 4. Cost Analytics (`cost-analytics.json`)
Financial tracking and cost optimization:
- Total estimated costs (all-time, daily, monthly projection)
- Hourly cost trends (input vs output)
- Cost distribution by model
- Daily cost by model
- Detailed cost breakdown table
- Cost efficiency (cost per completed agent)

**Cost Calculation:**
- Input tokens: $3 per million tokens
- Output tokens: $15 per million tokens
- Based on Claude Sonnet pricing

**Template Variables:**
- `model` - Filter by AI model

## Installation

### Option 1: Manual Import

1. Access your Grafana instance (default: http://localhost:3000)
2. Navigate to Dashboards → Import
3. Click "Upload JSON file" or paste JSON content
4. Import each dashboard file from the `dashboards/` directory
5. Select your Prometheus data source when prompted

### Option 2: Automatic Provisioning

For production deployments with automatic dashboard loading:

1. Copy provisioning configuration:
   ```bash
   cp grafana/provisioning/dashboards.yaml /etc/grafana/provisioning/dashboards/
   ```

2. Copy dashboard files:
   ```bash
   mkdir -p /var/lib/grafana/dashboards
   cp grafana/dashboards/*.json /var/lib/grafana/dashboards/
   ```

3. Restart Grafana:
   ```bash
   sudo systemctl restart grafana-server
   ```

### Option 3: Docker Deployment

Add to your `docker-compose.yml`:

```yaml
services:
  grafana:
    image: grafana/grafana:latest
    ports:
      - "3000:3000"
    volumes:
      - grafana-storage:/var/lib/grafana
      - ./grafana/provisioning:/etc/grafana/provisioning
      - ./grafana/dashboards:/var/lib/grafana/dashboards
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_INSTALL_PLUGINS=
    depends_on:
      - prometheus

  prometheus:
    image: prom/prometheus:latest
    ports:
      - "9090:9090"
    volumes:
      - ./prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-storage:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'

volumes:
  grafana-storage:
  prometheus-storage:
```

## Configuration

### Prometheus Data Source

The dashboards use a template variable `${DS_PROMETHEUS}` for the data source. When importing:

1. Select your Prometheus data source from the dropdown
2. Or configure it in Grafana:
   - Go to Configuration → Data Sources
   - Add Prometheus
   - Set URL to your Prometheus instance (e.g., `http://prometheus:9090`)
   - Save & Test

### Prometheus Scrape Configuration

Add to your `prometheus.yml`:

```yaml
scrape_configs:
  - job_name: 'orchestrate'
    static_configs:
      - targets: ['orchestrate-web:8080']
    metrics_path: '/metrics'
    scrape_interval: 15s
```

### Dashboard Refresh Rates

Default refresh intervals:
- Agent Overview: 30 seconds
- Token Usage: 30 seconds
- Alerts: 30 seconds
- Cost Analytics: 1 minute

Adjust as needed based on your monitoring requirements and system load.

## Dashboard Features

### Template Variables

All dashboards include dynamic template variables for filtering:
- **Data Source**: Select Prometheus instance
- **Agent Type**: Filter agents by type
- **Model**: Filter by AI model
- **Error Type**: Filter errors
- **Queue**: Filter by queue name

### Time Ranges

Recommended time ranges:
- Agent Overview: Last 6 hours (real-time monitoring)
- Token Usage: Last 24 hours (daily trends)
- Alerts: Last 6 hours (recent issues)
- Cost Analytics: Last 7 days (cost trends)

### Alerts Integration

The Alerts dashboard integrates with Prometheus alerting rules. Configure alerts in your `prometheus.yml`:

```yaml
rule_files:
  - 'alerts.yml'

alerting:
  alertmanagers:
    - static_configs:
        - targets: ['alertmanager:9093']
```

## Metrics Reference

### Agent Metrics
- `orchestrate_agents_total{type, state}` - Agent count by type and state
- `orchestrate_agent_execution_seconds{type}` - Execution duration histogram
- `orchestrate_agent_success_rate{agent_type}` - Success rate by type

### Token Metrics
- `orchestrate_tokens_total{model, direction}` - Token usage counter

### System Metrics
- `orchestrate_http_requests_total{method, path, status}` - HTTP request count
- `orchestrate_http_request_duration_seconds{method, path}` - Request duration
- `orchestrate_queue_depth{queue}` - Queue depth gauge
- `orchestrate_errors_total{error_type}` - Error counter

### Business Metrics
- `orchestrate_pr_cycle_time_seconds{epic_id}` - PR cycle time histogram
- `orchestrate_story_completion_rate{epic_id}` - Story completion rate
- `orchestrate_code_review_turnaround_seconds{reviewer}` - Review turnaround
- `orchestrate_deployments_total{environment}` - Deployment counter
- `orchestrate_mttr_seconds{severity}` - Mean time to recovery

## Testing

Validate dashboard JSON files:

```bash
cd grafana
./test_dashboards.sh
```

This script checks:
- JSON syntax validity
- Required dashboard fields (panels, title)
- Grafana dashboard structure
- YAML provisioning configuration

## Customization

### Modifying Dashboards

1. Import dashboard to Grafana
2. Make changes in the UI
3. Export JSON (Dashboard settings → JSON Model)
4. Save to `dashboards/` directory
5. Commit changes

### Adding New Panels

When adding panels, ensure:
- Use `${DS_PROMETHEUS}` for data source
- Include descriptive titles and descriptions
- Add to appropriate template variable filters
- Use consistent color schemes
- Set appropriate thresholds

### Cost Pricing Updates

To update cost calculations in the Cost Analytics dashboard, modify the pricing multipliers:

Current pricing (Claude Sonnet):
- Input: $3 per 1M tokens (`* 3 / 1000000`)
- Output: $15 per 1M tokens (`* 15 / 1000000`)

Update these values in all cost calculation queries.

## Troubleshooting

### No Data Showing

1. Verify Prometheus is scraping metrics:
   ```bash
   curl http://localhost:9090/api/v1/targets
   ```

2. Check Orchestrate metrics endpoint:
   ```bash
   curl http://localhost:8080/metrics
   ```

3. Verify data source configuration in Grafana

### Dashboard Import Errors

- Ensure you're using Grafana 9.0 or later
- Check JSON syntax with `jq` or `python -m json.tool`
- Verify Prometheus data source exists

### Missing Metrics

Some metrics only appear when data exists:
- Agent metrics require agents in the database
- Token metrics require API calls with token usage
- Queue metrics require pending items

## Support

For issues or questions:
1. Check Grafana logs: `/var/log/grafana/grafana.log`
2. Check Prometheus targets: http://localhost:9090/targets
3. Verify Orchestrate metrics: http://localhost:8080/metrics
4. Review dashboard JSON for query errors

## License

Part of the Orchestrate multi-agent system.
