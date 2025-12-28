---
name: review
description: Review code, PR, or specific files for quality, security, and best practices.
---

# Review Skill

Perform code review using the code-reviewer agent.

## Usage

```
/review [target]
```

## Target Options

| Target | Description |
|--------|-------------|
| (none) | Review staged changes |
| `<file>` | Review specific file |
| `<directory>` | Review directory |
| `#<pr-number>` | Review pull request |
| `HEAD~N..HEAD` | Review last N commits |

## Examples

```bash
# Review staged changes
/review

# Review specific file
/review src/auth/login.rs

# Review PR #42
/review #42

# Review src directory
/review src/

# Review last 3 commits
/review HEAD~3..HEAD
```

## Review Output

```markdown
## Summary
Brief overview of the code

## Issues

### Critical
- [SECURITY] SQL injection vulnerability in user.rs:42
- [BUG] Null pointer in auth.rs:87

### High
- [PERFORMANCE] O(n²) loop in process.rs:123

### Medium
- [STYLE] Inconsistent naming in utils.rs

### Low
- [DOCS] Missing docstring for public function

## Recommendations
- Add input validation for user data
- Consider using prepared statements
- Add unit tests for edge cases

## Positive Notes
- Good error handling
- Clean separation of concerns
```

## Review Criteria

1. **Correctness** - Does it work as intended?
2. **Security** - Vulnerabilities (OWASP top 10)?
3. **Performance** - Bottlenecks, complexity?
4. **Maintainability** - Clear, documented?
5. **Tests** - Adequate coverage?
6. **Style** - Consistent with codebase?

## PR Review Commands

```bash
# View PR details
gh pr view <number>

# View PR diff
gh pr diff <number>

# View review comments
gh api repos/{owner}/{repo}/pulls/<number>/comments
```

## Integration

Review results can be:
- Posted as PR comment (`gh pr comment`)
- Saved to file
- Displayed in terminal
