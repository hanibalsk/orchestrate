---
name: bmad
description: Run BMAD (Big Model Agent-Driven) workflow for epics and stories.
---

# BMAD Skill

Execute the BMAD development workflow.

## Usage

```
/bmad [epic-pattern]
```

## Workflow Phases

1. **FIND_EPIC** - Locate epic files in `docs/bmad/epics/`
2. **CREATE_BRANCH** - Create feature branch and worktree
3. **DEVELOP_STORIES** - Implement each story with TDD
4. **CODE_REVIEW** - Review completed implementation
5. **CREATE_PR** - Create pull request
6. **WAIT_COPILOT** - Wait for CI and Copilot review
7. **FIX_ISSUES** - Address any failures or comments
8. **MERGE_PR** - Merge when all checks pass

## Epic Location

```
docs/bmad/epics/
├── epic-001-user-auth.md
├── epic-002-api-endpoints.md
└── epic-003-dashboard.md
```

## Story Format

```markdown
### Story 1.1: Implement login endpoint
- [ ] Create POST /auth/login route
- [ ] Validate credentials
- [ ] Return JWT token
- [ ] Add rate limiting
```

## Commands Used

```bash
# List epics
orchestrate bmad status

# Process specific epic
orchestrate bmad process "epic-001-*"

# Reset state (if stuck)
orchestrate bmad reset
```

## Agent Coordination

The skill spawns agents in sequence:
1. `bmad-planner` - Parse epic and create plan
2. `story-developer` - Implement each story (TDD)
3. `code-reviewer` - Review implementation
4. `pr-shepherd` - Monitor PR through merge
