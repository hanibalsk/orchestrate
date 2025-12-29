# Deployment Strategies

This document describes the deployment strategies available in the Orchestrate deployment system.

## Overview

The deployment strategy framework provides four main deployment strategies:

1. **Rolling** - Gradual replacement of instances
2. **Blue-Green** - Switch between two identical environments
3. **Canary** - Route percentage of traffic to new version
4. **Recreate** - Stop old, start new (for dev environments)

Each strategy supports configurable health checks to ensure deployment success.

## Rolling Deployment

Rolling deployment gradually replaces old instances with new ones in batches.

### Configuration

```rust
use orchestrate_core::{DeploymentStrategy, BatchSize, HealthCheck};

// Create rolling strategy with absolute batch size
let strategy = DeploymentStrategy::rolling(
    BatchSize::Count(5),  // Update 5 instances at a time
    30                     // Wait 30 seconds between batches
);

// Or use percentage-based batches
let strategy = DeploymentStrategy::rolling(
    BatchSize::Percent(25),  // Update 25% of instances at a time
    60                        // Wait 60 seconds between batches
);
```

### Health Checks

```rust
let health_check = HealthCheck {
    endpoint: "/health".to_string(),
    expected_status: 200,
    timeout_seconds: 10,
    max_retries: 3,
    retry_interval_seconds: 5,
    headers: HashMap::new(),
};

let strategy = DeploymentStrategy::rolling(BatchSize::Count(5), 30)
    .with_health_check(health_check);
```

### Use Cases

- Production deployments with no downtime
- Gradual rollout to minimize risk
- Ability to stop mid-deployment if issues detected

## Blue-Green Deployment

Blue-Green deployment maintains two identical environments and switches traffic between them.

### Configuration

```rust
use orchestrate_core::{DeploymentStrategy, BlueGreenEnvironment};

// Create blue-green strategy
let strategy = DeploymentStrategy::blue_green(BlueGreenEnvironment::Blue);
```

### Customization

```rust
let mut strategy = DeploymentStrategy::blue_green(BlueGreenEnvironment::Blue);

if let Some(config) = &mut strategy.blue_green {
    config.switch_wait_seconds = 60;           // Wait 60s before switching
    config.keep_old_environment = true;         // Keep old env for rollback
    config.old_environment_ttl_seconds = 7200;  // Keep for 2 hours
}
```

### Flow

1. Deploy to inactive environment (e.g., if Blue is active, deploy to Green)
2. Run health checks on new environment
3. Switch load balancer to new environment
4. Keep old environment for quick rollback if needed

### Use Cases

- Zero-downtime deployments
- Instant rollback capability
- Testing in production-like environment before switching

## Canary Deployment

Canary deployment gradually routes traffic to the new version while monitoring metrics.

### Configuration

```rust
use orchestrate_core::DeploymentStrategy;

// Define traffic progression
let traffic_steps = vec![10, 25, 50, 100];  // 10% -> 25% -> 50% -> 100%

let strategy = DeploymentStrategy::canary(traffic_steps);
```

### Customization

```rust
let mut strategy = DeploymentStrategy::canary(vec![10, 25, 50, 100]);

if let Some(config) = &mut strategy.canary {
    config.step_wait_seconds = 600;  // Wait 10 minutes between steps
    config.monitored_metrics = vec![
        "error_rate".to_string(),
        "latency_p99".to_string(),
        "request_count".to_string(),
    ];
    config.error_threshold_percent = 3.0;  // Rollback if error rate > 3%
}
```

### Flow

1. Deploy to canary instances (10% of traffic)
2. Monitor metrics for anomalies
3. If metrics are healthy, gradually increase traffic (25%, 50%, 100%)
4. Rollback automatically if metrics degrade

### Use Cases

- High-risk deployments
- Testing new versions with real production traffic
- Automatic rollback on metric degradation

## Recreate Deployment

Recreate deployment stops all old instances before starting new ones.

### Configuration

```rust
use orchestrate_core::DeploymentStrategy;

let strategy = DeploymentStrategy::recreate();
```

### Characteristics

- Downtime during deployment
- Simplest strategy
- Useful for development environments

### Use Cases

- Development environments
- Services where downtime is acceptable
- Stateful applications that can't run multiple versions

## Health Checks

All strategies support configurable health checks.

### Basic Health Check

```rust
use orchestrate_core::HealthCheck;

let health_check = HealthCheck::default();  // Uses /health, 200 status
```

### Custom Health Check

```rust
let health_check = HealthCheck {
    endpoint: "/api/v1/health".to_string(),
    expected_status: 200,
    timeout_seconds: 15,
    max_retries: 5,
    retry_interval_seconds: 3,
    headers: {
        let mut headers = HashMap::new();
        headers.insert("X-Health-Check".to_string(), "true".to_string());
        headers
    },
};
```

### Multiple Health Checks

```rust
let strategy = DeploymentStrategy::rolling(BatchSize::Count(5), 30)
    .with_health_check(HealthCheck {
        endpoint: "/health".to_string(),
        ..Default::default()
    })
    .with_health_check(HealthCheck {
        endpoint: "/ready".to_string(),
        ..Default::default()
    });
```

## Strategy Validation

All strategies include validation to ensure configuration correctness:

```rust
let strategy = DeploymentStrategy::rolling(BatchSize::Count(5), 30);

// Validate before using
match strategy.validate() {
    Ok(_) => println!("Strategy configuration is valid"),
    Err(e) => eprintln!("Invalid strategy: {}", e),
}
```

### Validation Rules

**Rolling:**
- Batch size count must be > 0
- Batch size percent must be 1-100

**Blue-Green:**
- Must have blue_green configuration

**Canary:**
- Must have at least one traffic step
- Traffic percentages must be 0-100
- Error threshold must be 0-100

**Recreate:**
- Must have recreate configuration

## Strategy Selection Guide

| Scenario | Recommended Strategy | Why |
|----------|---------------------|-----|
| Production API | Rolling or Canary | No downtime, gradual rollout |
| Production with high risk | Canary | Automatic rollback on metrics |
| Production requiring instant rollback | Blue-Green | Instant traffic switch |
| Stateless microservices | Rolling | Simple and effective |
| Development environment | Recreate | Simplest, downtime acceptable |
| Database migrations | Blue-Green | Test migration before switching |

## Configuration per Environment

Strategies can be configured per environment in your environment configuration:

```yaml
environments:
  development:
    type: development
    deployment_strategy:
      type: recreate
      allow_downtime: true

  staging:
    type: staging
    deployment_strategy:
      type: rolling
      batch_size: 50%
      batch_wait_seconds: 30
      health_checks:
        - endpoint: /health
          expected_status: 200

  production:
    type: production
    deployment_strategy:
      type: canary
      traffic_steps: [10, 25, 50, 100]
      step_wait_seconds: 600
      monitored_metrics:
        - error_rate
        - latency_p99
      error_threshold_percent: 2.0
      health_checks:
        - endpoint: /health
          expected_status: 200
        - endpoint: /ready
          expected_status: 200
```

## Next Steps

- Implement deployment execution engine
- Add metric monitoring integration
- Create deployment CLI commands
- Build deployment dashboard UI
