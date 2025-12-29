# Webhook Configuration

Orchestrate's webhook system can be configured to control which GitHub events are processed and how they are filtered.

## Configuration File

Create a `config.yaml` file in the project root (or specify a custom path) with the following structure:

```yaml
webhooks:
  secret: ${GITHUB_WEBHOOK_SECRET}
  events:
    <event-name>:
      agent: <agent-type>
      filter:
        <filter-rules>
```

## Supported Event Types

### Pull Request Events

#### `pull_request.opened`

Triggered when a PR is opened. Spawns a `pr-shepherd` agent.

**Available Filters:**
- `base_branch`: List of branches to watch (e.g., `[main, develop]`)
- `skip_forks`: Set to `true` to ignore PRs from forked repositories

**Example:**
```yaml
webhooks:
  events:
    pull_request.opened:
      agent: pr_shepherd
      filter:
        base_branch: [main, develop]
        skip_forks: true
```

#### `pull_request_review.submitted`

Triggered when a review is submitted. Spawns an `issue-fixer` agent when changes are requested.

**Available Filters:**
- `base_branch`: List of branches to watch
- `skip_forks`: Set to `true` to ignore reviews on forked PRs

**Example:**
```yaml
webhooks:
  events:
    pull_request_review.submitted:
      agent: issue_fixer
      filter:
        base_branch: [main, develop]
```

### CI/CD Events

#### `check_run.completed`

Triggered when a CI check run completes. Spawns an `issue-fixer` agent for failures.

**Available Filters:**
- `conclusion`: List of conclusions to handle (e.g., `[failure, timed_out]`)

**Example:**
```yaml
webhooks:
  events:
    check_run.completed:
      agent: issue_fixer
      filter:
        conclusion: [failure, timed_out]
```

#### `check_suite.completed`

Triggered when a CI check suite completes. Spawns an `issue-fixer` agent for failures.

**Available Filters:**
- `conclusion`: List of conclusions to handle

**Example:**
```yaml
webhooks:
  events:
    check_suite.completed:
      agent: issue_fixer
      filter:
        conclusion: [failure, timed_out]
```

### Repository Events

#### `push`

Triggered when commits are pushed. Spawns a `regression-tester` agent for pushes to main/master.

**Note:** Push events to main/master are handled by default. Feature branch pushes are automatically filtered out by the handler.

**Example:**
```yaml
webhooks:
  events:
    push:
      agent: regression_tester
```

### Issue Events

#### `issues.opened`

Triggered when an issue is opened. Spawns an `issue-triager` agent.

**Available Filters:**
- `labels`: List of required labels (issue must have at least one)
- `author`: List of allowed author usernames
- `paths`: List of path patterns (for issues mentioning specific files)

**Example:**
```yaml
webhooks:
  events:
    issues.opened:
      agent: issue_triager
      filter:
        labels: [bug, enhancement, security]
        author: [team-lead, security-team]
```

## Filter Behavior

### Logic

- **No filter**: All events of that type are processed
- **Multiple filter conditions**: ALL conditions must pass (AND logic)
- **List values**: At least ONE value must match (OR logic within a list)

### Examples

**Only PRs to main or develop:**
```yaml
filter:
  base_branch: [main, develop]
```

**Only failures, not successes:**
```yaml
filter:
  conclusion: [failure, timed_out]
```

**Must have bug OR enhancement label:**
```yaml
filter:
  labels: [bug, enhancement]
```

**Must target main AND not be from a fork:**
```yaml
filter:
  base_branch: [main]
  skip_forks: true
```

## Environment Variables

Sensitive values like webhook secrets should use environment variable substitution:

```yaml
webhooks:
  secret: ${GITHUB_WEBHOOK_SECRET}
```

The `${VAR_NAME}` syntax will be replaced with the value of the environment variable at runtime.

## Default Behavior

If no configuration file is provided:
- All webhook events are processed using default handlers
- No filtering is applied (except built-in security filters like fork skipping for PR opened)

## Loading Configuration

### From Code

```rust
use orchestrate_core::WebhookConfig;

// Load from file
let config = WebhookConfig::from_yaml_file("config.yaml")?;

// Load from string
let yaml = r#"
webhooks:
  events:
    pull_request.opened:
      agent: pr_shepherd
"#;
let config = WebhookConfig::from_yaml_str(yaml)?;
```

### With Webhook Processor

```rust
use orchestrate_web::webhook_processor::{WebhookProcessor, WebhookProcessorConfig};

let processor = WebhookProcessor::new(database, WebhookProcessorConfig::default())
    .with_config(config);
```

## Security Considerations

1. **Always use environment variables for secrets** - Never commit secrets to version control
2. **Enable `skip_forks: true`** for PR events to prevent malicious code execution
3. **Filter events by branch** to limit agent spawning to important branches
4. **Use label filters** for issues to prevent spam triggering agents

## Example Configurations

### Minimal Configuration

```yaml
webhooks:
  secret: ${GITHUB_WEBHOOK_SECRET}
  events:
    pull_request.opened:
      agent: pr_shepherd
      filter:
        skip_forks: true
```

### Production Configuration

```yaml
webhooks:
  secret: ${GITHUB_WEBHOOK_SECRET}
  events:
    pull_request.opened:
      agent: pr_shepherd
      filter:
        base_branch: [main, develop, release/*]
        skip_forks: true

    pull_request_review.submitted:
      agent: issue_fixer
      filter:
        base_branch: [main, develop]

    check_run.completed:
      agent: issue_fixer
      filter:
        conclusion: [failure, timed_out]

    check_suite.completed:
      agent: issue_fixer
      filter:
        conclusion: [failure, timed_out]

    push:
      agent: regression_tester

    issues.opened:
      agent: issue_triager
      filter:
        labels: [bug, security, critical]
```

### Development Configuration

```yaml
webhooks:
  secret: ${GITHUB_WEBHOOK_SECRET}
  events:
    pull_request.opened:
      agent: pr_shepherd
      filter:
        base_branch: [main, develop, feature/*]
        # Allow forks in development for testing
        skip_forks: false

    issues.opened:
      agent: issue_triager
      # No filters - process all issues in development
```

## Troubleshooting

### Events Not Being Processed

1. Check that the event type is in your configuration
2. Verify filter conditions are being met
3. Check logs for "Event not configured" or "Event filtered out" messages
4. Ensure environment variables are set correctly

### Unexpected Events Being Processed

1. Review filter conditions - remember they use AND logic
2. Check for missing filters (no filter = process all)
3. Verify list values are correct (case-sensitive)

## Future Enhancements

Planned features for webhook configuration:

- [ ] Agent type overriding via configuration
- [ ] Regex pattern matching for path filters
- [ ] Custom filter functions
- [ ] Rate limiting per event type
- [ ] Event batching configuration
- [ ] Conditional agent spawning based on payload analysis
