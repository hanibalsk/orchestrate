# Epic 006: Deployment Orchestrator Agent

Implement automated deployment capabilities with multi-environment support.

**Priority:** Critical
**Effort:** Large
**Use Cases:** UC-205, UC-206

## Overview

Enable fully automated deployments to multiple environments with support for various deployment strategies (blue-green, canary), rollback capabilities, and release management. This is critical for closing the loop from code to production.

## Stories

### Story 1: Deployer Agent Type

Create new agent type for deployments.

**Acceptance Criteria:**
- [ ] Add `deployer` to AgentType enum
- [ ] Create agent prompt in `.claude/agents/deployer.md`
- [ ] Agent understands deployment configurations
- [ ] Agent can execute deployment commands
- [ ] Agent validates deployment success
- [ ] Agent can trigger rollback on failure

### Story 2: Environment Configuration

Define and manage deployment environments.

**Acceptance Criteria:**
- [ ] Create `environments` table: id, name, type (dev/staging/prod), config, created_at
- [ ] Support environment-specific variables
- [ ] Support secrets management (encrypted storage)
- [ ] Validate environment connectivity
- [ ] `orchestrate env list/create/show/delete` commands

**Configuration:**
```yaml
environments:
  staging:
    type: staging
    url: https://staging.example.com
    provider: aws
    config:
      cluster: staging-ecs
      service: app-staging
    secrets:
      AWS_ACCESS_KEY: ${STAGING_AWS_KEY}

  production:
    type: production
    url: https://example.com
    provider: aws
    config:
      cluster: prod-ecs
      service: app-prod
    requires_approval: true
```

### Story 3: Deployment Strategies

Implement multiple deployment strategies.

**Acceptance Criteria:**
- [ ] **Rolling**: Gradual replacement of instances
- [ ] **Blue-Green**: Switch between two identical environments
- [ ] **Canary**: Route percentage of traffic to new version
- [ ] **Recreate**: Stop old, start new (for dev)
- [ ] Strategy configuration per environment
- [ ] Strategy-specific health checks

**Blue-Green Flow:**
1. Deploy to inactive environment (blue or green)
2. Run health checks
3. Switch load balancer to new environment
4. Keep old environment for quick rollback

**Canary Flow:**
1. Deploy to canary instances (10%)
2. Monitor metrics for anomalies
3. Gradually increase traffic (25%, 50%, 100%)
4. Rollback if metrics degrade

### Story 4: Pre-Deployment Validation

Validate before deployment starts.

**Acceptance Criteria:**
- [ ] Check all tests pass
- [ ] Check security scan passes
- [ ] Verify artifact exists and is signed
- [ ] Validate environment is reachable
- [ ] Check no conflicting deployments in progress
- [ ] Verify deployment window (if configured)
- [ ] `orchestrate deploy validate --env <env>` command

### Story 5: Deployment Execution

Execute deployments to target environments.

**Acceptance Criteria:**
- [ ] `orchestrate deploy --env <env> --version <version>` command
- [ ] Support for container deployments (Docker, ECS, K8s)
- [ ] Support for serverless (Lambda, Cloud Functions)
- [ ] Support for static sites (S3, Vercel, Netlify)
- [ ] Progress reporting during deployment
- [ ] Deployment timeout handling
- [ ] Record deployment in database

**Providers:**
```rust
enum DeploymentProvider {
    Docker,
    AwsEcs,
    AwsLambda,
    Kubernetes,
    Vercel,
    Netlify,
    Railway,
    Custom(String),
}
```

### Story 6: Post-Deployment Verification

Verify deployment success after completion.

**Acceptance Criteria:**
- [ ] Run smoke tests against deployed environment
- [ ] Check health endpoints
- [ ] Verify expected version is running
- [ ] Check logs for errors
- [ ] Monitor error rates for anomalies
- [ ] Mark deployment as successful or failed
- [ ] Trigger rollback if verification fails

### Story 7: Rollback Capabilities

Implement deployment rollback.

**Acceptance Criteria:**
- [ ] `orchestrate deploy rollback --env <env>` command
- [ ] Rollback to previous version automatically
- [ ] Rollback to specific version: `--version <version>`
- [ ] Fast rollback for blue-green (traffic switch)
- [ ] Record rollback events
- [ ] Notify on rollback

### Story 8: Release Management

Automate release creation and versioning.

**Acceptance Criteria:**
- [ ] `orchestrate release prepare --type <major|minor|patch>` command
- [ ] Semantic version bumping in package files
- [ ] Create release branch
- [ ] Generate changelog from commits
- [ ] Generate release notes
- [ ] `orchestrate release create --version <version>` command
- [ ] Create GitHub release with assets
- [ ] Tag release commit

**Changelog Generation:**
```markdown
## [1.2.0] - 2024-01-15

### Added
- GitHub webhook triggers (Epic 002)
- Scheduled agent execution (Epic 003)

### Fixed
- Agent timeout handling (#123)

### Changed
- Improved PR shepherd performance
```

### Story 9: Deployment CLI Commands

Comprehensive CLI for deployments.

**Acceptance Criteria:**
- [ ] `orchestrate deploy --env <env> --version <version> [--strategy <strategy>]`
- [ ] `orchestrate deploy status --env <env>` - Current deployment status
- [ ] `orchestrate deploy history --env <env>` - Deployment history
- [ ] `orchestrate deploy rollback --env <env> [--version <version>]`
- [ ] `orchestrate deploy validate --env <env>` - Pre-deployment checks
- [ ] `orchestrate deploy diff --env <env>` - Show what will change
- [ ] `orchestrate release prepare --type <type>`
- [ ] `orchestrate release create --version <version>`
- [ ] `orchestrate release publish --version <version>`
- [ ] `orchestrate release notes --from <tag> --to <tag>`

### Story 10: Deployment REST API

Add REST endpoints for deployments.

**Acceptance Criteria:**
- [ ] `GET /api/environments` - List environments
- [ ] `GET /api/environments/:name` - Get environment details
- [ ] `POST /api/deployments` - Trigger deployment
- [ ] `GET /api/deployments` - List deployments
- [ ] `GET /api/deployments/:id` - Get deployment details
- [ ] `POST /api/deployments/:id/rollback` - Trigger rollback
- [ ] `GET /api/releases` - List releases
- [ ] `POST /api/releases` - Create release
- [ ] `POST /api/releases/:version/publish` - Publish release

### Story 11: Deployment Dashboard UI

Add deployment management to web dashboard.

**Acceptance Criteria:**
- [ ] Environment overview with current versions
- [ ] Deployment history timeline
- [ ] One-click deploy button with confirmation
- [ ] Rollback button
- [ ] Deployment progress visualization
- [ ] Release management page
- [ ] Deployment comparison between environments

### Story 12: Feature Flags Integration

Support feature flag management.

**Acceptance Criteria:**
- [ ] Integrate with LaunchDarkly, Unleash, or similar
- [ ] Toggle features without deployment
- [ ] Gradual rollout via flags
- [ ] `orchestrate flags list/enable/disable` commands
- [ ] Flag status in deployment dashboard

## Definition of Done

- [ ] All stories completed and tested
- [ ] Deployment to at least one provider working
- [ ] Rollback tested and reliable
- [ ] Release automation operational
- [ ] Documentation with provider setup guides
- [ ] Security review for secrets handling
