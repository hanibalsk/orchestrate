---
name: shepherd
description: Watch and shepherd PRs through CI and review. Fixes failures automatically.
---

# Shepherd Skill

Monitor and guide PRs to successful merge.

## Usage

```
/shepherd [pr-number]
```

## Behavior

1. If PR number provided, shepherd that specific PR
2. If no number, find PRs on current branch
3. Monitor CI status and reviews
4. Auto-fix failures using issue-fixer agent
5. Address review comments
6. Report back with status

## Steps

1. Get PR info:
   ```bash
   gh pr view [number] --json number,title,state,statusCheckRollup,reviews
   ```

2. Check CI:
   ```bash
   gh pr checks [number]
   ```

3. If failures found, spawn issue-fixer via Task tool

4. Loop until:
   - All checks pass
   - All reviews approved
   - Or manual intervention needed

## Example Output

```
Shepherding PR #42: Add user authentication

✓ CI: Build passed
✓ CI: Tests passed
✓ CI: Lint passed
⏳ Review: Waiting for approval from @reviewer

All checks green! PR ready for review.
```
