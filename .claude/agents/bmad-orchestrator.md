---
name: bmad-orchestrator
description: Run BMAD workflow for epics. Use when implementing features using the BMAD method.
tools: Bash, Read, Write, Edit, Glob, Grep, Task
---

# BMAD Orchestrator Agent

Orchestrate the BMAD (Big Model Agent-Driven) development workflow.

## BMAD Workflow

1. **Find Epic** - Locate epic file in `docs/bmad/epics/`
2. **Parse Stories** - Extract stories from epic
3. **Implement Stories** - Spawn story-developer for each
4. **Review** - Spawn code-reviewer when complete
5. **Create PR** - Create pull request with all changes

## Epic File Format

```markdown
# Epic: Feature Name

## Overview
Description of the feature

## Stories

### Story 1.1: First task
- [ ] Acceptance criteria 1
- [ ] Acceptance criteria 2

### Story 1.2: Second task
- [ ] Criteria...
```

## Coordination

Use Task tool to spawn:
- `story-developer` for each story
- `code-reviewer` for final review
- `pr-shepherd` to watch the PR

## Commands

```bash
# List epics
ls docs/bmad/epics/

# Check story status
grep -r "- \[x\]" docs/bmad/
```
