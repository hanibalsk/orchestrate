# Epic 007: Monitoring & Alerting

Implement comprehensive monitoring, metrics collection, and alerting system.

**Priority:** High
**Effort:** Medium
**Use Cases:** UC-301, UC-302, UC-303, UC-304, UC-305, UC-306

## Overview

Add real-time observability to the orchestrate system including metrics collection, distributed tracing, alerting, cost analytics, and audit logging. This enables proactive issue detection and provides visibility into system health.

## Stories

### Story 1: Prometheus Metrics Endpoint

Expose metrics in Prometheus format.

**Acceptance Criteria:**
- [ ] Add `/metrics` endpoint to orchestrate-web
- [ ] Expose agent metrics (count by state, type)
- [ ] Expose token usage metrics (input, output, by model)
- [ ] Expose API latency histograms
- [ ] Expose queue depth metrics
- [ ] Expose error rate metrics
- [ ] Standard metric naming conventions

**Metrics:**
```prometheus
# Agent metrics
orchestrate_agents_total{type="story-developer",state="running"} 3
orchestrate_agent_execution_seconds{type="story-developer",quantile="0.99"} 120.5

# Token metrics
orchestrate_tokens_total{model="claude-3-opus",direction="input"} 1500000
orchestrate_tokens_total{model="claude-3-opus",direction="output"} 500000

# API metrics
orchestrate_http_requests_total{method="POST",path="/api/agents",status="200"} 150
orchestrate_http_request_duration_seconds{method="POST",path="/api/agents",quantile="0.95"} 0.25

# Queue metrics
orchestrate_queue_depth{queue="webhook_events"} 5
orchestrate_queue_depth{queue="pr_queue"} 2
```

### Story 2: Custom Business Metrics

Track domain-specific metrics.

**Acceptance Criteria:**
- [ ] PR cycle time (open to merge)
- [ ] Story completion rate
- [ ] Agent success/failure rate by type
- [ ] Code review turnaround time
- [ ] Deployment frequency
- [ ] Mean time to recovery (MTTR)
- [ ] Custom metric registration API

### Story 3: Alerting Rules Engine

Define and evaluate alerting rules.

**Acceptance Criteria:**
- [ ] Create `alert_rules` table: id, name, condition, severity, channels, enabled
- [ ] Create `alerts` table: id, rule_id, status, triggered_at, resolved_at
- [ ] Support threshold conditions (>, <, ==)
- [ ] Support rate conditions (increase by X% in Y minutes)
- [ ] Support absence conditions (no data for X minutes)
- [ ] Evaluate rules periodically (configurable interval)
- [ ] Deduplication of repeated alerts

**Rule Examples:**
```yaml
rules:
  - name: high-failure-rate
    condition: "rate(orchestrate_agent_failures_total[5m]) > 0.2"
    severity: critical
    channels: [slack, pagerduty]

  - name: queue-backup
    condition: "orchestrate_queue_depth{queue='webhook_events'} > 100"
    severity: warning
    channels: [slack]

  - name: token-budget-exceeded
    condition: "sum(orchestrate_tokens_total) > 1000000"
    severity: warning
    channels: [email]
```

### Story 4: Alert Notification Channels

Send alerts to multiple channels.

**Acceptance Criteria:**
- [ ] Slack webhook integration
- [ ] Email (SMTP) integration
- [ ] PagerDuty integration
- [ ] Generic webhook integration
- [ ] Channel configuration in settings
- [ ] Message templates per channel
- [ ] Rate limiting to prevent spam

**Slack Alert Format:**
```
🚨 CRITICAL: high-failure-rate

Agent failure rate exceeds 20% in the last 5 minutes.

Current value: 35%
Threshold: 20%
Triggered at: 2024-01-15 14:32:00 UTC

[View Dashboard](https://orchestrate.example.com/alerts/123)
```

### Story 5: Alert CLI Commands

Manage alerts via CLI.

**Acceptance Criteria:**
- [ ] `orchestrate alert rules list` - List all rules
- [ ] `orchestrate alert rules create --name <name> --condition <cond> --channel <ch>`
- [ ] `orchestrate alert rules enable/disable <name>`
- [ ] `orchestrate alert rules delete <name>`
- [ ] `orchestrate alert list --status <active|resolved>` - List alerts
- [ ] `orchestrate alert acknowledge <id>` - Acknowledge alert
- [ ] `orchestrate alert silence <name> --duration <duration>` - Silence rule
- [ ] `orchestrate alert test <name>` - Test alert delivery

### Story 6: OpenTelemetry Tracing

Add distributed tracing support.

**Acceptance Criteria:**
- [ ] Add opentelemetry-rust dependency
- [ ] Instrument agent execution with spans
- [ ] Instrument tool calls with spans
- [ ] Instrument database queries with spans
- [ ] Instrument HTTP requests with spans
- [ ] Export to Jaeger or OTLP endpoint
- [ ] Trace ID propagation in logs
- [ ] `orchestrate trace enable --exporter <jaeger|otlp>` command

**Span Hierarchy:**
```
agent_execution (agent_id, agent_type)
├── tool_call (tool_name)
│   └── database_query (query_type)
├── tool_call (tool_name)
│   └── http_request (method, path)
└── api_call (model)
```

### Story 7: Cost Analytics

Track and analyze API costs.

**Acceptance Criteria:**
- [ ] Calculate cost per token by model
- [ ] Aggregate cost by agent, epic, story
- [ ] Daily/weekly/monthly cost reports
- [ ] Cost trends and forecasting
- [ ] Budget alerts
- [ ] Cost optimization recommendations
- [ ] `orchestrate cost report` command
- [ ] `orchestrate cost budget set --monthly <amount>` command

**Cost Report:**
```
Cost Report - January 2024

Total: $847.32

By Model:
  claude-3-opus:    $612.45 (72%)
  claude-3-sonnet:  $234.87 (28%)

By Agent Type:
  story-developer:  $523.18 (62%)
  code-reviewer:    $156.92 (19%)
  pr-shepherd:      $98.45 (12%)
  Other:            $68.77 (7%)

Top Epics:
  epic-002:         $234.56
  epic-003:         $189.23
  epic-001:         $145.67

Forecast: $892.00 for February (based on current trend)
Budget: $1000.00 (84% used)
```

### Story 8: Audit Logging

Comprehensive audit trail for compliance.

**Acceptance Criteria:**
- [ ] Create `audit_log` table: id, timestamp, actor, action, resource, details
- [ ] Log agent lifecycle events (spawn, terminate)
- [ ] Log configuration changes
- [ ] Log approval decisions
- [ ] Log deployment events
- [ ] Log authentication events
- [ ] Immutable log entries
- [ ] Log retention policy
- [ ] `orchestrate audit search` command

**Audit Events:**
```json
{
  "timestamp": "2024-01-15T14:32:00Z",
  "actor": "user@example.com",
  "action": "deployment.triggered",
  "resource": "environment:production",
  "details": {
    "version": "1.2.0",
    "strategy": "canary"
  },
  "ip_address": "192.168.1.100"
}
```

### Story 9: Agent Performance Analytics

Track and compare agent effectiveness.

**Acceptance Criteria:**
- [ ] Calculate success rate per agent type
- [ ] Track average completion time
- [ ] Track token efficiency (tokens per task)
- [ ] Compare performance across models
- [ ] Identify underperforming agents
- [ ] Trend analysis over time
- [ ] `orchestrate analytics agents` command

### Story 10: Monitoring REST API

Add REST endpoints for monitoring.

**Acceptance Criteria:**
- [ ] `GET /api/metrics` - Prometheus metrics
- [ ] `GET /api/alerts/rules` - List alert rules
- [ ] `POST /api/alerts/rules` - Create rule
- [ ] `GET /api/alerts` - List alerts
- [ ] `POST /api/alerts/:id/acknowledge` - Acknowledge
- [ ] `GET /api/cost/report` - Cost report
- [ ] `GET /api/audit` - Audit log query
- [ ] `GET /api/analytics/agents` - Agent analytics

### Story 11: Monitoring Dashboard UI

Add monitoring pages to web dashboard.

**Acceptance Criteria:**
- [ ] System health overview widget
- [ ] Real-time metrics charts
- [ ] Alert list with status indicators
- [ ] Cost dashboard with trends
- [ ] Agent performance comparison charts
- [ ] Audit log viewer with search
- [ ] Alert rule configuration UI

### Story 12: Grafana Dashboard Templates

Provide pre-built Grafana dashboards.

**Acceptance Criteria:**
- [ ] System overview dashboard
- [ ] Agent performance dashboard
- [ ] Cost analytics dashboard
- [ ] Alert history dashboard
- [ ] Export as JSON templates
- [ ] Documentation for setup

## Definition of Done

- [ ] All stories completed and tested
- [ ] Metrics endpoint operational
- [ ] Alerting tested end-to-end
- [ ] Tracing exportable to Jaeger
- [ ] Cost tracking accurate
- [ ] Audit log compliance reviewed
- [ ] Grafana templates published
