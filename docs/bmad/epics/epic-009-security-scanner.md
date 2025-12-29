# Epic 009: Security Scanner Agent

Implement automated security analysis and remediation capabilities.

**Priority:** High
**Effort:** Medium
**Use Cases:** UC-207

## Overview

Add a security-scanner agent that automatically detects vulnerabilities in dependencies, code, secrets, and configurations. The agent can not only detect issues but also propose and apply fixes, integrating security into the automated development workflow.

## Stories

### Story 1: Security Scanner Agent Type

Create new agent type for security scanning.

**Acceptance Criteria:**
- [ ] Add `security-scanner` to AgentType enum
- [ ] Create agent prompt in `.claude/agents/security-scanner.md`
- [ ] Agent understands common vulnerability types
- [ ] Agent can run security tools and parse output
- [ ] Agent can suggest and apply fixes
- [ ] Agent follows security best practices

### Story 2: Dependency Vulnerability Scanning

Scan dependencies for known vulnerabilities.

**Acceptance Criteria:**
- [ ] Integrate `cargo audit` for Rust projects
- [ ] Integrate `npm audit` for Node.js projects
- [ ] Integrate `pip-audit` for Python projects
- [ ] Parse vulnerability reports (CVE, severity, fix version)
- [ ] Generate fix recommendations
- [ ] Auto-update dependencies when safe
- [ ] `orchestrate security scan --dependencies` command

**Report Format:**
```
Dependency Vulnerability Report

CRITICAL (1):
  📦 lodash 4.17.20
     CVE-2021-23337: Prototype Pollution
     Fix: Upgrade to 4.17.21
     [Auto-fix available]

HIGH (2):
  📦 axios 0.21.0
     CVE-2021-3749: ReDoS vulnerability
     Fix: Upgrade to 0.21.2

  📦 node-fetch 2.6.0
     CVE-2022-0235: Exposure of sensitive information
     Fix: Upgrade to 2.6.7

MEDIUM (3): ...

Total: 6 vulnerabilities found
Auto-fixable: 4
```

### Story 3: Static Application Security Testing (SAST)

Analyze code for security issues.

**Acceptance Criteria:**
- [ ] Integrate semgrep for multi-language SAST
- [ ] Detect OWASP Top 10 vulnerabilities
- [ ] Detect SQL injection patterns
- [ ] Detect XSS vulnerabilities
- [ ] Detect authentication issues
- [ ] Detect hardcoded credentials
- [ ] `orchestrate security scan --code` command

**SAST Rules:**
- SQL injection (concatenated queries)
- XSS (unescaped output)
- Command injection (shell exec with user input)
- Path traversal (unchecked file paths)
- Insecure deserialization
- Weak cryptography
- Hardcoded secrets

### Story 4: Secret Detection

Find leaked secrets in code and history.

**Acceptance Criteria:**
- [ ] Integrate gitleaks or truffleHog
- [ ] Scan current files for secrets
- [ ] Scan git history for leaked secrets
- [ ] Detect API keys, tokens, passwords
- [ ] Detect private keys and certificates
- [ ] Generate secret rotation recommendations
- [ ] `orchestrate security scan --secrets` command

**Detected Secret Types:**
- AWS credentials
- GitHub tokens
- Slack tokens
- Database connection strings
- Private keys (RSA, SSH)
- JWT secrets
- Generic high-entropy strings

### Story 5: License Compliance Scanning

Check dependency licenses for compliance.

**Acceptance Criteria:**
- [ ] Scan all dependency licenses
- [ ] Configurable allowed/denied license list
- [ ] Detect copyleft licenses (GPL)
- [ ] Detect unknown licenses
- [ ] Generate license report
- [ ] `orchestrate security scan --licenses` command

**License Policy:**
```yaml
licenses:
  allowed:
    - MIT
    - Apache-2.0
    - BSD-2-Clause
    - BSD-3-Clause
    - ISC

  denied:
    - GPL-2.0
    - GPL-3.0
    - AGPL-3.0

  review_required:
    - LGPL-2.1
    - MPL-2.0
```

### Story 6: Container Image Scanning

Scan Docker images for vulnerabilities.

**Acceptance Criteria:**
- [ ] Integrate Trivy or Grype for container scanning
- [ ] Scan base image vulnerabilities
- [ ] Scan installed packages
- [ ] Detect misconfigurations
- [ ] Recommend base image updates
- [ ] `orchestrate security scan --container <image>` command

### Story 7: Security Fix Agent

Automatically apply security fixes.

**Acceptance Criteria:**
- [ ] Upgrade vulnerable dependencies automatically
- [ ] Apply code fixes for common vulnerabilities
- [ ] Remove detected secrets and add to .gitignore
- [ ] Create PR with security fixes
- [ ] `orchestrate security fix --vuln <id>` command
- [ ] `orchestrate security fix --all-safe` command

**Fix Categories:**
- Auto-fix: Dependency upgrades (patch/minor)
- Manual review: Dependency upgrades (major)
- Manual review: Code changes
- Manual action: Secret rotation

### Story 8: Security Report Generation

Generate comprehensive security reports.

**Acceptance Criteria:**
- [ ] SARIF format for GitHub Security tab
- [ ] JSON format for CI integration
- [ ] HTML format for human review
- [ ] PDF format for compliance
- [ ] Include remediation steps
- [ ] `orchestrate security report --format <format>` command

### Story 9: Security Gate for Pipelines

Block deployments on security issues.

**Acceptance Criteria:**
- [ ] Configurable security policy
- [ ] Block on critical/high vulnerabilities
- [ ] Allow override with justification
- [ ] Integrate with pipeline approvals
- [ ] Track security exceptions

**Policy:**
```yaml
security_gate:
  block_on:
    - severity: critical
    - severity: high
      age_days: 7  # Block if not fixed within 7 days

  allow_exceptions:
    - requires_approval: true
    - max_duration_days: 30
```

### Story 10: Security CLI Commands

Comprehensive CLI for security operations.

**Acceptance Criteria:**
- [ ] `orchestrate security scan [--full|--dependencies|--code|--secrets|--licenses|--container]`
- [ ] `orchestrate security report --format <sarif|json|html|pdf>`
- [ ] `orchestrate security fix --vuln <id>` - Fix specific vulnerability
- [ ] `orchestrate security fix --all-safe` - Fix all auto-fixable
- [ ] `orchestrate security policy show` - Show security policy
- [ ] `orchestrate security exceptions list` - List active exceptions
- [ ] `orchestrate security baseline update` - Update baseline (ignore existing)

### Story 11: Security REST API

Add REST endpoints for security.

**Acceptance Criteria:**
- [ ] `POST /api/security/scan` - Trigger scan
- [ ] `GET /api/security/scans` - List scans
- [ ] `GET /api/security/scans/:id` - Get scan results
- [ ] `GET /api/security/vulnerabilities` - List vulnerabilities
- [ ] `POST /api/security/fix` - Apply fix
- [ ] `GET /api/security/report` - Download report
- [ ] `GET /api/security/policy` - Get policy
- [ ] `PUT /api/security/policy` - Update policy

### Story 12: Security Dashboard UI

Add security pages to web dashboard.

**Acceptance Criteria:**
- [ ] Security overview with risk score
- [ ] Vulnerability list with filtering
- [ ] Vulnerability detail with fix guidance
- [ ] Scan history timeline
- [ ] Fix button for auto-fixable issues
- [ ] Policy configuration UI
- [ ] Exception management

## Definition of Done

- [ ] All stories completed and tested
- [ ] All scan types operational
- [ ] SARIF reports compatible with GitHub
- [ ] Auto-fix working for dependencies
- [ ] Security gate blocking deploys
- [ ] Documentation with best practices
