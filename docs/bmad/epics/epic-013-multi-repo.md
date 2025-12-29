# Epic 013: Multi-Repository Orchestration

Implement coordination of work across multiple repositories.

**Priority:** Medium
**Effort:** Large
**Use Cases:** UC-506

## Overview

Enable orchestrate to manage development across multiple repositories, handling cross-repo dependencies, synchronized releases, and coordinated deployments. Essential for microservices and monorepo-to-polyrepo transitions.

## Stories

### Story 1: Repository Registry

Manage multiple repository configurations.

**Acceptance Criteria:**
- [ ] Create `repositories` table: id, name, url, path, config
- [ ] `orchestrate repo add --url <url> --path <local-path>`
- [ ] `orchestrate repo list`
- [ ] `orchestrate repo remove <name>`
- [ ] Support GitHub, GitLab, Bitbucket URLs
- [ ] Clone repository on add

### Story 2: Cross-Repository Dependencies

Track dependencies between repositories.

**Acceptance Criteria:**
- [ ] Define dependency relationships
- [ ] Detect dependency from package files
- [ ] Visualize dependency graph
- [ ] Identify circular dependencies
- [ ] `orchestrate repo dependencies`

**Dependency Definition:**
```yaml
repositories:
  api:
    url: github.com/org/api
    depends_on: [core-lib]

  web:
    url: github.com/org/web
    depends_on: [api]

  core-lib:
    url: github.com/org/core-lib
```

### Story 3: Synchronized Development

Coordinate changes across repos.

**Acceptance Criteria:**
- [ ] Create linked branches across repos
- [ ] `orchestrate repo branch create --name <name> --repos <list>`
- [ ] Track branch status across repos
- [ ] Merge coordination
- [ ] Conflict detection across repos

### Story 4: Cross-Repository PRs

Manage related PRs across repositories.

**Acceptance Criteria:**
- [ ] Link PRs across repositories
- [ ] Track linked PR status
- [ ] Coordinate PR merging order
- [ ] `orchestrate pr link --pr <number> --repo <repo>`
- [ ] Block merge if dependencies not merged

### Story 5: Coordinated Releases

Release multiple repositories together.

**Acceptance Criteria:**
- [ ] Define release groups
- [ ] Version bump across repos
- [ ] Coordinated changelog
- [ ] Atomic release (all or none)
- [ ] `orchestrate release --repos <list> --version <version>`

### Story 6: Cross-Repository Agents

Spawn agents that work across repos.

**Acceptance Criteria:**
- [ ] Agent can access multiple repos
- [ ] Agent maintains context across repos
- [ ] Coordinate parallel work in different repos
- [ ] `orchestrate spawn cross-repo-developer --repos <list> --task <task>`

### Story 7: Multi-Repo CLI Commands

CLI commands for multi-repo operations.

**Acceptance Criteria:**
- [ ] `orchestrate repo add/remove/list`
- [ ] `orchestrate repo dependencies`
- [ ] `orchestrate repo branch create/status`
- [ ] `orchestrate repo sync` - Sync all repos
- [ ] `orchestrate release --repos` - Coordinated release

### Story 8: Multi-Repo Dashboard UI

Add multi-repo views to dashboard.

**Acceptance Criteria:**
- [ ] Repository list with status
- [ ] Dependency graph visualization
- [ ] Cross-repo PR tracker
- [ ] Coordinated release UI

## Definition of Done

- [ ] All stories completed and tested
- [ ] Multi-repo branches working
- [ ] Coordinated releases tested
- [ ] Documentation complete
