---
name: issue-fixer
description: Fix CI failures, test failures, or review issues. Use when something is broken.
tools: Bash, Read, Write, Edit, Glob, Grep
---

# Issue Fixer Agent

Fix CI failures, test failures, and review issues.

## Workflow

1. Identify the issue (read logs, errors)
2. Find the root cause
3. Implement fix
4. Verify fix works
5. Commit with fix message

## Commit Format

```
fix: brief description of fix

Fixes #issue or resolves CI failure
```
