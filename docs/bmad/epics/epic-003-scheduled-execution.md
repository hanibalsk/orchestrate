# Epic 003: Scheduled Agent Execution

Implement cron-like scheduling for recurring agent tasks.

**Priority:** High
**Effort:** Medium
**Use Cases:** UC-102

## Overview

Enable scheduling of agents to run at specific times or intervals. This supports recurring tasks like daily security scans, weekly reports, and periodic maintenance operations.

## Stories

### Story 1: Schedule Data Model

Create database schema for scheduled tasks.

**Acceptance Criteria:**
- [ ] Create `schedules` table with fields: id, name, cron_expression, agent_type, task, enabled, last_run, next_run, created_at
- [ ] Create `schedule_runs` table for execution history
- [ ] Add migration for new tables
- [ ] Implement Schedule struct in orchestrate-core
- [ ] Add CRUD operations for schedules

**Schema:**
```sql
CREATE TABLE schedules (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    cron_expression TEXT NOT NULL,
    agent_type TEXT NOT NULL,
    task TEXT NOT NULL,
    enabled BOOLEAN DEFAULT true,
    last_run_at TIMESTAMP,
    next_run_at TIMESTAMP NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

### Story 2: Cron Expression Parser

Implement cron expression parsing and next-run calculation.

**Acceptance Criteria:**
- [ ] Parse standard 5-field cron expressions (min hour day month weekday)
- [ ] Support extended syntax (@daily, @weekly, @hourly)
- [ ] Calculate next run time from current time
- [ ] Validate cron expressions on schedule creation
- [ ] Handle timezone considerations (UTC default)

**Examples:**
- `0 2 * * *` - Daily at 2 AM
- `0 0 * * 0` - Weekly on Sunday
- `*/15 * * * *` - Every 15 minutes
- `@daily` - Once per day at midnight

### Story 3: Schedule Executor Service

Background service that executes due schedules.

**Acceptance Criteria:**
- [ ] Poll database for schedules where next_run <= now
- [ ] Spawn configured agent with task
- [ ] Update last_run and calculate next next_run
- [ ] Record execution in schedule_runs table
- [ ] Handle concurrent execution prevention (locking)
- [ ] Configurable check interval (default 60s)

### Story 4: Missed Schedule Handling

Handle schedules that were missed (system downtime).

**Acceptance Criteria:**
- [ ] Detect schedules where next_run is in the past
- [ ] Configurable behavior: run_immediately, skip, or catch_up
- [ ] Log missed schedule events
- [ ] Alert on missed critical schedules
- [ ] Limit catch-up to prevent flooding

**Configuration:**
```yaml
schedules:
  missed_policy: run_immediately  # or skip, catch_up
  catch_up_limit: 3  # max catch-up runs
```

### Story 5: Schedule CLI Commands

Add CLI commands for schedule management.

**Acceptance Criteria:**
- [ ] `orchestrate schedule add --name <name> --cron <expr> --agent <type> --task <task>`
- [ ] `orchestrate schedule list` - Show all schedules with next run time
- [ ] `orchestrate schedule show <name>` - Show schedule details and history
- [ ] `orchestrate schedule pause <name>` - Disable schedule
- [ ] `orchestrate schedule resume <name>` - Enable schedule
- [ ] `orchestrate schedule delete <name>` - Remove schedule
- [ ] `orchestrate schedule run-now <name>` - Trigger immediate execution
- [ ] `orchestrate schedule history <name>` - Show execution history

### Story 6: Built-in Schedule Templates

Provide pre-configured schedules for common tasks.

**Acceptance Criteria:**
- [ ] Security scan template (daily at 2 AM)
- [ ] Dependency update check template (daily at 3 AM)
- [ ] Code quality report template (weekly Monday 9 AM)
- [ ] Documentation freshness check template (weekly)
- [ ] Database backup template (daily)
- [ ] `orchestrate schedule add-template <template-name>` command

**Templates:**
```yaml
templates:
  security-scan:
    cron: "0 2 * * *"
    agent: security-scanner
    task: "Run full security scan and report vulnerabilities"

  dependency-check:
    cron: "0 3 * * *"
    agent: dependency-checker
    task: "Check for outdated dependencies and security advisories"
```

### Story 7: Schedule REST API

Add REST endpoints for schedule management.

**Acceptance Criteria:**
- [ ] `GET /api/schedules` - List all schedules
- [ ] `POST /api/schedules` - Create schedule
- [ ] `GET /api/schedules/:id` - Get schedule details
- [ ] `PUT /api/schedules/:id` - Update schedule
- [ ] `DELETE /api/schedules/:id` - Delete schedule
- [ ] `POST /api/schedules/:id/pause` - Pause schedule
- [ ] `POST /api/schedules/:id/resume` - Resume schedule
- [ ] `POST /api/schedules/:id/run` - Trigger immediate run
- [ ] `GET /api/schedules/:id/runs` - Get execution history

### Story 8: Schedule Dashboard UI

Add schedule management to web dashboard.

**Acceptance Criteria:**
- [ ] Schedule list page showing all schedules
- [ ] Visual indicator for enabled/disabled
- [ ] Next run countdown display
- [ ] Create/edit schedule form with cron builder
- [ ] Execution history view
- [ ] Manual trigger button
- [ ] Pause/resume toggle

## Definition of Done

- [ ] All stories completed and tested
- [ ] Integration tests for scheduler
- [ ] Documentation for schedule setup
- [ ] Built-in templates documented
- [ ] Performance tested with 100+ schedules
