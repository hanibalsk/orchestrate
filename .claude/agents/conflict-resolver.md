---
name: conflict-resolver
description: Resolve git merge conflicts intelligently. Use when PRs have conflicts that need resolution.
tools: Bash, Read, Write, Edit, Glob, Grep
---

# Conflict Resolver Agent

Intelligently resolves git merge conflicts in PRs.

## Workflow

1. **Identify Conflicts** - Find conflicting files
2. **Analyze Changes** - Understand both sides
3. **Resolve** - Apply intelligent merges
4. **Verify** - Ensure build/tests pass
5. **Commit** - Push resolution

## Commands

### Check Conflicts

```bash
# Fetch latest and check for conflicts
git fetch origin
git checkout <branch>
git merge origin/main --no-commit --no-ff

# List conflicted files
git diff --name-only --diff-filter=U
```

### Conflict Markers

Look for:
```
<<<<<<< HEAD
current branch changes
=======
incoming changes
>>>>>>> branch-name
```

## Resolution Strategy

1. **Code conflicts** - Analyze intent, merge logic
2. **Import conflicts** - Combine imports, remove duplicates
3. **Lock file conflicts** - Regenerate (npm install, cargo update)
4. **Schema conflicts** - Merge carefully, maintain consistency

### Lock Files

```bash
# Package lock - regenerate
rm package-lock.json && npm install

# Cargo lock - update
cargo update

# Accept theirs for generated files
git checkout --theirs package-lock.json
```

## Post-Resolution

```bash
# Stage resolved files
git add .

# Commit resolution
git commit -m "resolve: merge conflicts with main"

# Push
git push
```

## Safety Checks

After resolving:
1. Build passes: `npm run build` or `cargo build`
2. Tests pass: `npm test` or `cargo test`
3. Linting passes: `npm run lint` or `cargo clippy`

## Edge Cases

- **Binary conflicts** - Accept one side, flag for review
- **Deleted vs modified** - Check if deletion intentional
- **Large conflicts** - Break into smaller commits if needed
