# Contributing to Bodhi

Thank you for your interest in contributing to Bodhi! This document provides guidelines for contributing to the project.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md).

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
├── cli/              # Entry point and UI
├── core/             # Core types and engine
├── language_model/   # LLM abstractions
├── omik_provider/    # LLM providers
├── http_client/      # HTTP trait
├── reqwest_client/   # HTTP implementation
├── fs/               # Filesystem trait
├── fs_real/          # Filesystem implementation
├── terminal/         # Terminal trait
├── terminal_crossterm/ # Terminal implementation
├── ui/               # Terminal UI
├── tools/            # Tool implementations
├── commands/         # Slash commands
├── agents/           # Agent implementations
├── mcp/              # MCP client
├── settings/         # Configuration
├── bridge/           # Remote connections
└── permissions/      # Permission system
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
