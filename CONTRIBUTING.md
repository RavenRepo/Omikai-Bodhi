# Contributing to Bodhi

Thank you for your interest in contributing to Bodhi! This document provides guidelines for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

---

## Git Workflow

### Branch Naming

| Type | Example | Description |
|------|---------|-------------|
| `feature/*` | `feature/add-tui-input` | New features |
| `fix/*` | `fix/llm-timeout-handling` | Bug fixes |
| `refactor/*` | `refactor/message-types` | Code refactoring |
| `docs/*` | `docs/api-documentation` | Documentation updates |
| `test/*` | `test/agent-scenarios` | Adding tests |
| `chore/*` | `chore/update-dependencies` | Maintenance tasks |

### Branch Rules

1. **Never commit directly to `main`** - All changes must go through a branch
2. **Keep branches focused** - One feature per branch
3. **Delete merged branches** - Clean up after merge
4. **Rebase vs Merge** - Prefer rebasing for clean history

### Workflow

```bash
# 1. Sync with upstream
git fetch upstream
git checkout main
git merge upstream/main

# 2. Create a new branch
git checkout -b feature/my-awesome-feature

# 3. Make changes and commit
git add .
git commit -m "feat: add awesome feature"

# 4. Push and create PR
git push -u origin feature/my-awesome-feature
```

---

## Issue Guidelines

### Before Creating an Issue

- [ ] Search existing issues to avoid duplicates
- [ ] Check if issue exists in planned features
- [ ] Verify bug on latest version

### Issue Types

#### 🐛 Bug Reports

```
Title: Brief description of the bug

## Environment
- OS: 
- Rust version:
- Bodhi version:

## Steps to Reproduce
1. 
2. 
3.

## Expected Behavior
What should happen

## Actual Behavior
What actually happens

## Additional Context
Screenshots, logs, etc.
```

#### 💡 Feature Requests

```
Title: Brief description of the feature

## Problem
What problem does this solve?

## Proposed Solution
How should it work?

## Alternatives Considered
Other approaches you considered

## Use Cases
When would someone use this?
```

#### ❓ Questions

```
Title: Your question

## Context
What are you trying to achieve?

## What I've Tried
What have you already attempted?

## Additional Info
Any other relevant information
```

---

## Pull Request Guidelines

### Before Submitting

- [ ] Tests pass: `cargo test`
- [ ] Code formatted: `cargo fmt`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Documentation updated
- [ ] Commits are atomic and descriptive

### PR Title Format

```
<type>(<scope>): <description>
```

| Type | Use for |
|------|---------|
| `feat` | New features |
| `fix` | Bug fixes |
| `docs` | Documentation |
| `style` | Formatting changes |
| `refactor` | Code refactoring |
| `test` | Tests |
| `chore` | Maintenance |

### PR Description Template

```markdown
## Summary
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
Describe testing performed

## Checklist
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] Code follows style guidelines
- [ ] No clippy warnings
```

### PR Process

1. **Create PR** from your branch to `main`
2. **Fill template** with description
3. **Link issues** using "Closes #123" or "Fixes #456"
4. **Request review** from maintainers
5. **Address feedback** - Make changes if needed
6. **Keep updated** - Rebase onto main if needed
7. **Squash merge** - Maintainer will squash on merge

### What Makes a Good PR

- ✅ Focused on single concern
- ✅ Clear commit messages
- ✅ Tests for new functionality
- ✅ Documentation for public APIs
- ✅ No breaking changes (or clearly documented)

---

## How to Contribute

### Reporting Bugs

1. **Search existing issues** - Check if the bug has already been reported
2. **Create a new issue** - Use the bug report template
3. **Include details** - Steps to reproduce, expected behavior, actual behavior
4. **Add environment info** - OS, Rust version, Bodhi version

### Suggesting Features

1. **Search existing proposals** - Check if your idea has been discussed
2. **Open a discussion** - Talk about your idea before creating a PR
3. **Describe the use case** - Explain why this feature would be useful
4. **Provide examples** - Show how it would work in practice

### Pull Requests

#### Before Submitting

1. **Fork the repository**
2. **Create a feature branch**: `git checkout -b feature/my-new-feature`
3. **Run tests**: `cargo test`
4. **Format code**: `cargo fmt`
5. **Run clippy**: `cargo clippy`

#### PR Guidelines

- Keep changes focused and atomic
- Write descriptive commit messages
- Include tests for new functionality
- Update documentation as needed
- Add notes to CHANGELOG.md for user-facing changes

#### PR Process

1. Submit your PR with a clear description
2. Address review feedback promptly
3. Keep the PR updated with main branch
4. Wait for CI checks to pass

## Development Setup

### Prerequisites

- Rust 1.75 or later
- Cargo
- Git

### Getting Started

```bash
# Fork the repository on GitHub

# Clone your fork
git clone https://github.com/YOUR_USERNAME/Omikai-Bodhi.git
cd Omikai-Bodhi

# Add upstream
git remote add upstream https://github.com/RavenRepo/Omikai-Bodhi.git

# Build the project
cargo build

# Run tests
cargo test

# Run clippy for code quality
cargo clippy
```

### Coding Standards

- **Formatting**: Use `cargo fmt`
- **Linting**: Follow clippy suggestions
- **Testing**: All new code should have tests
- **Documentation**: Document public APIs

### Project Structure

```
crates/
├── cli/                    # Entry point and UI
├── core/                   # Core types and QueryEngine
├── language_model/        # LLM trait abstraction
├── omik_provider/          # Multi-provider client
├── http_client/           # HTTP trait
├── reqwest_client/        # HTTP implementation
├── fs/                    # Filesystem trait
├── fs_real/               # Filesystem implementation
├── terminal/              # Terminal trait
├── terminal_crossterm/   # crossterm implementation
├── ui/                    # Ratatui UI
├── tools/                 # Tool implementations
├── commands/              # Slash commands
├── agents/                # Agent system
├── mcp/                   # MCP client
├── settings/              # Configuration
├── bridge/                # Remote connections
└── permissions/           # Permission system
```

### Commit Message Format

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation
- `style`: Formatting
- `refactor`: Code refactoring
- `test`: Tests
- `chore`: Maintenance

Example:
```
feat(tools): add file_edit tool

Add a new tool for surgical file edits with diff preview.
Includes undo functionality and safety checks.

Closes #123
```

---

## Creating GitHub Templates

### Bug Report Template (.github/ISSUE_TEMPLATE/bug_report.md)

```markdown
---
name: 🐛 Bug Report
about: Report something that isn't working correctly
title: "[Bug]: "
labels: bug
assignees: ''
---

**Describe the bug**
A clear and concise description of what the bug is.

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. See error

**Expected behavior**
A clear description of what you expected to happen.

**Screenshots**
If applicable, add screenshots to help explain the problem.

**Environment:**
- OS:
- Rust version:
- Bodhi version:

**Additional context**
Add any other context about the problem here.
```

### Feature Request Template (.github/ISSUE_TEMPLATE/feature_request.md)

```markdown
---
name: 💡 Feature Request
about: Suggest a new feature or improvement
title: "[Feature]: "
labels: enhancement
assignees: ''
---

**Is your feature request related to a problem?**
A clear and concise description of what the problem is.

**Describe the solution you'd like**
A clear and concise description of what you want to happen.

**Describe alternatives you've considered**
A clear description of any alternative solutions you've considered.

**Additional context**
Add any other context about the feature request here.
```

### PR Template (.github/PULL_REQUEST_TEMPLATE.md)

```markdown
## Summary
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
Describe testing performed

## Checklist
- [ ] Tests pass (`cargo test`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] Documentation updated
- [ ] Code follows style guidelines

## Related Issues
Closes # (if applicable)
```

---

## Writing Tests

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Test implementation
    }
}
```

### Integration Tests

Place integration tests in `tests/` directory.

### Property-Based Testing

Consider using proptest for property-based tests where appropriate.

## Documentation

### Public APIs

All public functions should have documentation comments:

```rust
/// Description of what the function does.
///
/// # Arguments
/// * `input` - Description of input parameter
///
/// # Returns
/// Description of return value
///
/// # Errors
/// Description of possible error conditions
///
/// # Example
/// ```rust
/// let result = my_function("test");
/// assert!(result.is_ok());
/// ```
```

### crate Documentation

Each crate should have a `lib.rs` with module-level documentation.

## Recognition

Contributors will be recognized in:
- CHANGELOG.md
- CONTRIBUTORS file
- GitHub release notes

## Questions?

- Open a discussion for general questions
- Open an issue for bugs or feature requests
- Join our community chat (if available)

---

Thank you for contributing to Bodhi!
