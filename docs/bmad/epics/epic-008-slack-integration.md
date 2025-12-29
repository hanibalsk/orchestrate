# Epic 008: Slack Integration

Implement bi-directional Slack integration for notifications and interactive commands.

**Priority:** High
**Effort:** Medium
**Use Cases:** UC-401

## Overview

Enable rich Slack integration that provides real-time notifications, interactive approval buttons, slash commands for status checks, and thread-based discussions for PR reviews. This keeps teams informed without leaving their communication platform.

## Stories

### Story 1: Slack App Configuration

Set up Slack app and authentication.

**Acceptance Criteria:**
- [ ] Document Slack app creation process
- [ ] Configure OAuth scopes (chat:write, commands, interactive)
- [ ] Store Slack credentials securely
- [ ] Implement OAuth flow for workspace installation
- [ ] `orchestrate slack connect` command
- [ ] Verify connection with test message

**Required Scopes:**
- `chat:write` - Send messages
- `chat:write.public` - Send to public channels
- `commands` - Slash commands
- `users:read` - User lookup
- `reactions:write` - Add reactions

### Story 2: Notification Service

Send notifications to Slack channels.

**Acceptance Criteria:**
- [ ] Create notification service in orchestrate-core
- [ ] Support channel selection per notification type
- [ ] Support direct messages to users
- [ ] Support thread replies
- [ ] Rich message formatting with blocks
- [ ] Rate limiting to avoid spam
- [ ] Notification templates

**Notification Types:**
```yaml
notifications:
  channels:
    default: "#orchestrate"
    deployments: "#deployments"
    alerts: "#alerts"

  templates:
    agent_completed:
      channel: default
      blocks:
        - type: section
          text: "✅ Agent *{agent_type}* completed task"
        - type: context
          text: "Duration: {duration} | Tokens: {tokens}"
```

### Story 3: Agent Lifecycle Notifications

Notify on agent state changes.

**Acceptance Criteria:**
- [ ] Notify when agent starts (configurable)
- [ ] Notify when agent completes successfully
- [ ] Notify when agent fails with error summary
- [ ] Notify when agent needs input
- [ ] Include links to dashboard
- [ ] Configurable verbosity per channel

**Success Message:**
```
✅ Story Developer completed

Task: Implement user authentication
Epic: epic-002
Duration: 15m 32s
Tokens: 45,230

[View Details](https://orchestrate.example.com/agents/abc123)
```

**Failure Message:**
```
❌ Code Reviewer failed

Task: Review PR #123
Error: API rate limit exceeded

Retrying in 5 minutes...

[View Logs](https://orchestrate.example.com/agents/def456)
```

### Story 4: PR Notifications

Rich PR status notifications.

**Acceptance Criteria:**
- [ ] Notify when PR is created
- [ ] Notify when PR review is requested
- [ ] Notify when PR has comments
- [ ] Notify when CI passes/fails
- [ ] Notify when PR is merged
- [ ] Thread updates for same PR
- [ ] Action buttons for common operations

**PR Created Message:**
```
🔀 New PR Created

*#123: Add user authentication*
Branch: feature/auth → main
Author: orchestrate-bot

Files: 12 changed (+450/-120)
Status: ⏳ Waiting for CI

[View PR](https://github.com/org/repo/pull/123) | [Approve] | [Request Changes]
```

### Story 5: Interactive Approval Buttons

Handle approval requests via Slack buttons.

**Acceptance Criteria:**
- [ ] Send approval requests with interactive buttons
- [ ] Handle button click callbacks
- [ ] Verify user has approval permissions
- [ ] Update message after approval/rejection
- [ ] Record approval in audit log
- [ ] Support approval with comment

**Approval Request:**
```
⚠️ Approval Required

Deployment to *production* is pending approval.

Version: 1.2.0
Changes: 15 commits since last deploy
Requested by: @alice

[Approve ✓] [Reject ✗] [View Changes]
```

**After Approval:**
```
✅ Approved by @bob

Deployment to *production* is now in progress...
```

### Story 6: Slash Commands

Implement Slack slash commands.

**Acceptance Criteria:**
- [ ] `/orchestrate status` - Show system status
- [ ] `/orchestrate agents` - List running agents
- [ ] `/orchestrate pr <number>` - PR status
- [ ] `/orchestrate deploy <env>` - Trigger deployment
- [ ] `/orchestrate approve <id>` - Approve from Slack
- [ ] `/orchestrate help` - Show available commands
- [ ] Ephemeral responses for user-specific info

**Status Response:**
```
📊 Orchestrate Status

Agents: 3 running, 2 queued
PRs: 1 open (#123)
Queue: 2 items waiting

Recent Activity:
• Story completed: "Add login form" (2m ago)
• PR merged: #122 (15m ago)
• Deployment: staging v1.1.9 (1h ago)
```

### Story 7: Thread-Based PR Discussions

Use threads for PR conversations.

**Acceptance Criteria:**
- [ ] Create thread for each PR
- [ ] Post review comments to thread
- [ ] Post CI status updates to thread
- [ ] Post agent activity to thread
- [ ] Link thread to PR in database
- [ ] Archive thread when PR closed

### Story 8: User Mention Support

Map GitHub users to Slack users.

**Acceptance Criteria:**
- [ ] Create user mapping table
- [ ] `orchestrate slack map-user --github <gh> --slack <slack>`
- [ ] Automatically mention users on their PRs
- [ ] Mention code owners on relevant PRs
- [ ] Mention assignees on failures
- [ ] DM for urgent notifications

### Story 9: Slack CLI Commands

CLI commands for Slack management.

**Acceptance Criteria:**
- [ ] `orchestrate slack connect --token <token>` - Connect workspace
- [ ] `orchestrate slack disconnect` - Disconnect
- [ ] `orchestrate slack status` - Show connection status
- [ ] `orchestrate slack channels` - List available channels
- [ ] `orchestrate slack channel set --type <type> --channel <ch>` - Set channel
- [ ] `orchestrate slack test` - Send test message
- [ ] `orchestrate slack map-user` - Map users

### Story 10: Slack REST API

Add REST endpoints for Slack.

**Acceptance Criteria:**
- [ ] `POST /api/slack/connect` - OAuth callback
- [ ] `GET /api/slack/status` - Connection status
- [ ] `POST /api/slack/interactions` - Handle button clicks
- [ ] `POST /api/slack/commands` - Handle slash commands
- [ ] `GET /api/slack/channels` - List channels
- [ ] `POST /api/slack/notify` - Send notification

### Story 11: Notification Preferences UI

Web UI for notification configuration.

**Acceptance Criteria:**
- [ ] Slack connection status on dashboard
- [ ] Channel configuration per notification type
- [ ] User mapping interface
- [ ] Test notification button
- [ ] Notification history view
- [ ] Mute/unmute controls

## Definition of Done

- [ ] All stories completed and tested
- [ ] Slack app documented and published
- [ ] Interactive buttons working
- [ ] Slash commands responding
- [ ] Thread management operational
- [ ] User documentation complete
