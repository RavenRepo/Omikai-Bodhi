# Git Agent Instructions for Bodhi

## Branch Naming Convention

Must follow: `<type>/<description>`
- `feature/<name>` - New features
- `fix/<name>` - Bug fixes
- `docs/<name>` - Documentation
- `refactor/<name>` - Code refactoring
- `test/<name>` - Tests

**Valid:** `feature/tui-implementation`, `fix/error-handling`
**Invalid:** `my-feature`, `bug-fix`, `Feature/Foo`

## PR Review Checklist

Before approving, verify:

### 1. Code Quality
- [ ] `cargo fmt` has been run
- [ ] `cargo clippy` passes without errors (warnings allowed)
- [ ] `cargo build` succeeds

### 2. Commit Messages
Must follow Conventional Commits format:
```
<type>(<scope>): <description>

[optional body]
```

Valid types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

### 3. Required Checks
- [ ] All tests pass (`cargo test`)
- [ ] Code builds without errors
- [ ] No merge conflicts

### 4. Branch Protection (if applicable)
- [ ] PR has at least 1 approval
- [ ] CI/status checks pass
- [ ] Branch is up to date with main

## Merging Process

### Step 1: Fetch & Update
```bash
git fetch origin
git checkout main
git pull origin main
```

### Step 2: Check Merge
```bash
git merge --no-ff feature/<name>
# Resolve any conflicts
```

### Step 3: Verify Build
```bash
cargo build
cargo fmt
cargo clippy
```

### Step 4: Push
```bash
git push origin main
```

### Step 5: Delete Branch (Optional)
```bash
git branch -d feature/<name>
git push origin --delete feature/<name>
```

## Squash vs Merge

- **Squash**: For small, single-commit PRs
- **Merge commit**: For feature branches with multiple meaningful commits

## Common Mistakes to Avoid

1. **Don't merge without running checks** - Always verify build first
2. **Don't force push to main** - Never force push to protected branches
3. **Don't skip commit message format** - Enforce Conventional Commits
4. **Don't merge with unresolved conflicts** - Resolve in feature branch first
5. **Don't approve your own PRs** - Get at least 1 review

## Fast-Forward Merge Command
```bash
git merge --ff feature/<name>
```
Only use if the branch is directly behind main (no diverging commits).

## Rollback (if needed)
```bash
git revert <commit-hash>
git push origin main
```

---

Following these instructions ensures code quality and prevents mistakes during the merge process.
