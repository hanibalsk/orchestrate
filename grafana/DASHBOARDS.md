# Grafana Dashboard Quick Reference

## Dashboard Files

| Dashboard | File | Panels | Purpose | Recommended Time Range |
|-----------|------|--------|---------|----------------------|
| Agent Overview | `agent-overview.json` | 9 | Monitor agent execution, status, and performance | Last 6 hours |
| Token Usage | `token-usage.json` | 7 | Track LLM token consumption and costs | Last 24 hours |
| Alerts & Monitoring | `alerts.json` | 10 | System health, errors, and alerts | Last 6 hours |
| Cost Analytics | `cost-analytics.json` | 8 | Financial tracking and cost optimization | Last 7 days |

## Panel Details

### Agent Overview Dashboard (9 panels)

1. **Total Agents** (Stat) - Count of all agents
2. **Running Agents** (Stat) - Currently executing agents
3. **Completed Agents** (Stat) - Successfully completed
4. **Failed Agents** (Stat) - Failed executions
5. **Agents by Type and State** (Time Series) - Trend over time
6. **Agent Distribution by Type** (Pie Chart) - Distribution visualization
7. **Agent Execution Duration** (Time Series) - p50/p95 percentiles
8. **Agent Success Rate** (Gauge) - Success percentage by type
9. **Agent Error Rate** (Time Series) - Error trends

**Template Variables:**
- `agent_type` - Filter by: story-developer, code-reviewer, etc.

**Key Metrics Used:**
- `orchestrate_agents_total{type, state}`
- `orchestrate_agent_execution_seconds{type}`
- `orchestrate_agent_success_rate{agent_type}`
- `orchestrate_errors_total{error_type}`

---

### Token Usage Dashboard (7 panels)

1. **Total Tokens** (Stat) - All-time total
2. **Input Tokens** (Stat) - Input token count
3. **Output Tokens** (Stat) - Output token count
4. **Token Usage Rate by Model** (Time Series) - Rate of consumption
5. **Token Distribution by Model** (Donut Chart) - Model breakdown
6. **Hourly Token Consumption** (Time Series) - Input vs Output stacked
7. **Token Usage by Model** (Table) - Detailed breakdown with totals

**Template Variables:**
- `model` - Filter by: claude-sonnet-4-5, etc.

**Key Metrics Used:**
- `orchestrate_tokens_total{model, direction}`

---

### Alerts & Monitoring Dashboard (10 panels)

1. **Active Alerts** (Stat) - Currently firing alerts
2. **Error Rate** (Stat) - Errors per second
3. **Queue Depth** (Stat) - Total items in queues
4. **Failed Agents** (Stat) - Count of failures
5. **Error Rate by Type** (Time Series) - Trends by error type
6. **Error Distribution by Type** (Pie Chart) - Breakdown
7. **Queue Depth by Queue** (Time Series) - Individual queue tracking
8. **Agent Success vs Failure** (Time Series) - Comparison over time
9. **Error Summary** (Table) - Detailed error listing
10. **Agent Success Rate Over Time** (Time Series) - Success trend

**Template Variables:**
- `error_type` - Filter by error categories
- `queue` - Filter by queue name

**Key Metrics Used:**
- `ALERTS{alertstate}`
- `orchestrate_errors_total{error_type}`
- `orchestrate_queue_depth{queue}`
- `orchestrate_agents_total{state}`
- `orchestrate_agent_success_rate`

---

### Cost Analytics Dashboard (8 panels)

1. **Total Estimated Cost** (Stat) - All-time spending
2. **Daily Cost** (Stat) - Last 24 hours
3. **Projected Monthly Cost** (Stat) - Extrapolated from daily
4. **Hourly Cost Trend** (Time Series) - Input vs Output costs stacked
5. **Cost Distribution by Model** (Donut Chart) - Model cost breakdown
6. **Daily Cost by Model** (Time Series) - Daily trends per model
7. **Cost Breakdown by Model** (Table) - Detailed analysis with calculations
8. **Cost Efficiency** (Time Series) - Cost per completed agent

**Template Variables:**
- `model` - Filter by AI model

**Cost Calculations:**
- Input tokens: $3 per 1M tokens
- Output tokens: $15 per 1M tokens
- Based on Claude Sonnet pricing

**Key Metrics Used:**
- `orchestrate_tokens_total{model, direction}`
- `orchestrate_agents_total{state="completed"}`

---

## Metric-to-Dashboard Mapping

| Metric | Dashboards Using It |
|--------|-------------------|
| `orchestrate_agents_total` | Agent Overview, Alerts, Cost Analytics |
| `orchestrate_agent_execution_seconds` | Agent Overview |
| `orchestrate_agent_success_rate` | Agent Overview, Alerts |
| `orchestrate_tokens_total` | Token Usage, Cost Analytics |
| `orchestrate_errors_total` | Agent Overview, Alerts |
| `orchestrate_queue_depth` | Alerts |
| `orchestrate_http_requests_total` | (Available, not currently used) |
| `orchestrate_http_request_duration_seconds` | (Available, not currently used) |
| `ALERTS` | Alerts |

## Prometheus Queries Reference

### Common Query Patterns

**Rate calculations:**
```promql
rate(orchestrate_tokens_total[5m])
sum(rate(orchestrate_errors_total[$__rate_interval]))
```

**Percentiles:**
```promql
histogram_quantile(0.95, sum(rate(orchestrate_agent_execution_seconds_bucket[$__rate_interval])) by (le, type))
```

**Cost calculations:**
```promql
(sum(orchestrate_tokens_total{direction="input"}) * 3 / 1000000) +
(sum(orchestrate_tokens_total{direction="output"}) * 15 / 1000000)
```

**Aggregations:**
```promql
sum by (model) (orchestrate_tokens_total)
sum(orchestrate_agents_total{state="failed"}) / sum(orchestrate_agents_total)
```

## Alert Thresholds

Based on `alerts.example.yml`:

| Alert | Threshold | Duration |
|-------|-----------|----------|
| HighAgentFailureRate | >10% failures | 5 minutes |
| HighErrorRate | >1 error/sec | 5 minutes |
| CriticalErrorRate | >5 errors/sec | 2 minutes |
| HighQueueDepth | >100 items | 10 minutes |
| DailyCostBudgetExceeded | >$20/day | 1 hour |
| SlowAgentExecution | P95 >10 minutes | 15 minutes |

## Quick Start

1. **Import all dashboards:**
   ```bash
   # Manual import in Grafana UI
   for file in grafana/dashboards/*.json; do
     echo "Import: $file"
   done
   ```

2. **Docker deployment:**
   ```bash
   cd grafana
   cp docker-compose.example.yml docker-compose.yml
   docker-compose up -d
   ```

3. **Access dashboards:**
   - Grafana: http://localhost:3000 (admin/admin)
   - Prometheus: http://localhost:9090
   - AlertManager: http://localhost:9093

4. **Configure Prometheus data source:**
   - In Grafana: Configuration → Data Sources → Add Prometheus
   - URL: http://prometheus:9090

## Customization Guide

### Change Cost Pricing

Edit all queries in `cost-analytics.json`:
- Input pricing: Change `* 3 / 1000000` to your rate
- Output pricing: Change `* 15 / 1000000` to your rate

### Add Custom Panels

1. Import dashboard to Grafana
2. Edit in UI
3. Add panel with query like:
   ```promql
   orchestrate_custom_my_metric{label="value"}
   ```
4. Export JSON and save

### Modify Alert Thresholds

Edit `alerts.example.yml`:
```yaml
- alert: MyAlert
  expr: my_metric > threshold
  for: duration
```

## Troubleshooting

### No data in dashboards
```bash
# Check Prometheus targets
curl http://localhost:9090/api/v1/targets

# Check Orchestrate metrics
curl http://localhost:8080/metrics

# Verify data source in Grafana
# Settings → Data Sources → Prometheus → Test
```

### Dashboard import errors
```bash
# Validate JSON
python3 -m json.tool dashboard.json

# Or use the test script
./test_dashboards.sh
```

### Missing metrics
Some metrics only appear when:
- Agents exist in database
- API calls have been made
- Events are in queues

## Files Overview

```
grafana/
├── dashboards/              # Dashboard JSON files
│   ├── agent-overview.json
│   ├── token-usage.json
│   ├── alerts.json
│   └── cost-analytics.json
├── provisioning/           # Auto-provisioning config
│   └── dashboards.yaml
├── alerts.example.yml      # Prometheus alert rules
├── prometheus.example.yml  # Prometheus config
├── alertmanager.example.yml # AlertManager config
├── docker-compose.example.yml # Docker deployment
├── test_dashboards.sh     # Validation script
├── README.md              # Full documentation
└── DASHBOARDS.md         # This file
```

## Support Resources

- **Grafana Docs:** https://grafana.com/docs/grafana/latest/
- **Prometheus Query:** https://prometheus.io/docs/prometheus/latest/querying/basics/
- **PromQL Examples:** https://prometheus.io/docs/prometheus/latest/querying/examples/
- **Grafana Variables:** https://grafana.com/docs/grafana/latest/dashboards/variables/
