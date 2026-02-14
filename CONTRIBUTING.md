# Contributing to eth-price-tracker

Thank you for considering contributing to this project! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Code Standards](#code-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Pull Request Process](#pull-request-process)
- [Release Process](#release-process)

## Code of Conduct

### Our Standards

- Be respectful and inclusive
- Welcome constructive criticism
- Focus on what's best for the community
- Show empathy towards others

### Unacceptable Behavior

- Harassment or discriminatory language
- Trolling or insulting comments
- Publishing others' private information
- Unprofessional conduct

## Getting Started

### Prerequisites

- Rust 1.75 or later
- Git
- Alchemy API key (for integration tests)
- Foundry (for Anvil tests)

### Fork and Clone

1. Fork the repository on GitHub
2. Clone your fork:
   ```bash
   git clone https://github.com/YOUR_USERNAME/eth-price-tracker.git
   cd eth-price-tracker
   ```

3. Add upstream remote:
   ```bash
   git remote add upstream https://github.com/gaveesh89/eth-price-tracker.git
   ```

### Setup Development Environment

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Foundry (for Anvil)
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Install development tools
cargo install cargo-watch
cargo install cargo-tarpaulin  # For coverage
cargo install cargo-audit      # For security audits

# Setup pre-commit checks
cp scripts/pre-commit .git/hooks/
chmod +x .git/hooks/pre-commit
```

### Build and Test

```bash
# Build the project
cargo build

# Run all checks
make check

# Run tests
cargo test -- --test-threads=1

# Run in watch mode (for development)
cargo watch -x check -x test
```

## Development Workflow

### 1. Create a Branch

```bash
# Update your local master
git checkout master
git pull upstream master

# Create a feature branch
git checkout -b feature/your-feature-name

# Or for bug fixes
git checkout -b fix/bug-description
```

### 2. Make Changes

- Write clear, focused commits
- Follow the code standards below
- Add tests for new functionality
- Update documentation as needed

### 3. Test Your Changes

```bash
# Run all checks before committing
make check

# Run specific tests
cargo test --lib                    # Unit tests
cargo test --test '*'               # Integration tests
cargo test --doc                    # Doc tests

# Check code coverage
cargo tarpaulin --verbose --workspace --timeout 120
```

### 4. Commit Your Changes

```bash
git add .
git commit -m "feat: add new feature"
```

#### Commit Message Format

Follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New feature
- `fix:` Bug fix
- `docs:` Documentation changes
- `test:` Test additions/changes
- `refactor:` Code refactoring
- `perf:` Performance improvements
- `chore:` Build process or auxiliary tool changes
- `ci:` CI configuration changes

Examples:
```
feat: add WebSocket support for real-time updates
fix: correct decimal handling in price calculation
docs: update architecture diagram
test: add integration tests for state transitions
```

### 5. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then open a Pull Request on GitHub.

## Code Standards

### Rust Style Guide

Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/):

1. **Formatting**: Use `rustfmt`
   ```bash
   cargo fmt --check
   ```

2. **Linting**: Pass all clippy checks
   ```bash
   cargo clippy --all-targets --all-features -- -D warnings
   ```

3. **Naming Conventions**:
   - Use `snake_case` for functions, variables, modules
   - Use `CamelCase` for types, traits
   - Use `SCREAMING_SNAKE_CASE` for constants

### Code Quality Standards

#### 1. No Unsafe Code

```rust
#![forbid(unsafe_code)]
```

All code must be safe Rust. No exceptions without explicit approval.

#### 2. Error Handling

Never use `unwrap()`, `expect()`, or `panic!()` in production code:

```rust
// ❌ BAD
let value = some_result.unwrap();

// ✅ GOOD
let value = some_result?;
// or
let value = some_result.map_err(|e| TrackerError::...)?;
```

#### 3. Documentation

All public items must have documentation:

```rust
/// Brief one-line summary.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `param1` - Description of param1
///
/// # Returns
///
/// Description of return value
///
/// # Errors
///
/// When this function can error
///
/// # Examples
///
/// ```
/// use eth_uniswap_alloy::module::function;
/// let result = function(42);
/// assert!(result.is_ok());
/// ```
pub fn function(param1: u64) -> Result<(), Error> {
    // Implementation
    Ok(())
}
```

#### 4. Type Safety

Prefer strong types over primitives:

```rust
// ❌ BAD
fn process_address(addr: String) -> Result<()> { ... }

// ✅ GOOD
fn process_address(addr: Address) -> Result<()> { ... }
```

### Module Organization

Each module should:

1. Have comprehensive module-level documentation
2. Group related functionality
3. Export only public API
4. Include tests in a `mod tests` submodule

```rust
//! Module-level documentation.
//!
//! Explanation of what this module does.

// Imports
use crate::error::{TrackerError, TrackerResult};

// Constants
const DEFAULT_VALUE: u64 = 42;

// Types
pub struct MyStruct { ... }

// Implementation
impl MyStruct { ... }

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() { ... }
}
```

## Testing Guidelines

### Test Coverage Requirements

- **Minimum**: 80% code coverage
- **Target**: 90%+ code coverage
- All public APIs must have tests
- Critical paths must have multiple test scenarios

### Test Types

#### 1. Unit Tests

Located in each module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        let input = setup_test_data();
        let result = function_under_test(input);
        assert_eq!(result, expected_output);
    }

    #[test]
    fn test_error_condition() {
        let invalid_input = create_invalid_input();
        let result = function_under_test(invalid_input);
        assert!(result.is_err());
    }
}
```

#### 2. Integration Tests

Located in `tests/` directory:

```rust
#[test]
fn test_full_workflow() {
    // Setup
    let config = setup_test_config();
    
    // Execute
    let result = run_integration_test(&config);
    
    // Verify
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), expected_result);
}
```

#### 3. Documentation Tests

Embedded in documentation:

```rust
/// Calculate the sum of two numbers.
///
/// # Examples
///
/// ```
/// use mymodule::add;
/// assert_eq!(add(2, 2), 4);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

### Running Tests

```bash
# All tests
cargo test -- --test-threads=1

# Specific test
cargo test test_name

# With output
cargo test -- --nocapture

# Integration tests only
cargo test --test '*'

# With coverage
cargo tarpaulin --out Html
```

## Documentation

### What to Document

1. **All public APIs**: Functions, types, traits, modules
2. **Examples**: Show typical usage
3. **Errors**: Document when functions can fail
4. **Panics**: Document panic conditions (though we forbid panics)
5. **Safety**: Document any unsafe assumptions

### Documentation Standards

- First line should be a brief summary
- Include `# Examples` section for complex functions
- Include `# Errors` section for fallible functions
- Use proper markdown formatting
- Link to related items with `[`item`]`

### Generating Documentation

```bash
# Generate docs
cargo doc --no-deps --open

# Check doc warnings
cargo doc --no-deps 2>&1 | grep warning
```

## Pull Request Process

### Before Submitting

1. ✅ All tests pass: `make check`
2. ✅ Code is formatted: `cargo fmt`
3. ✅ No clippy warnings: `cargo clippy`
4. ✅ Documentation updated
5. ✅ CHANGELOG.md updated (if applicable)
6. ✅ Commit messages follow conventions

### PR Template

When creating a PR, include:

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests added
- [ ] Integration tests added
- [ ] All tests passing

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Comments added for complex code
- [ ] Documentation updated
- [ ] No new warnings
```

### Review Process

1. Automated CI checks must pass
2. At least one maintainer approval required
3. Address all review comments
4. Keep PR focused and reasonably sized (<500 lines preferred)

## Release Process

### Versioning

We follow [Semantic Versioning](https://semver.org/):

- **MAJOR**: Breaking changes
- **MINOR**: New features (backward compatible)
- **PATCH**: Bug fixes (backward compatible)

### Release Checklist

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Run full test suite
4. Create release tag
5. Build release binaries
6. Publish to crates.io (if applicable)

## Style Preferences

### Rust Idioms

```rust
// Prefer ? operator over match
let value = some_result?;  // ✅
match some_result { ... }  // ❌ (unless handling specific errors)

// Use iterators over indexing
items.iter().filter(...).collect()  // ✅
for i in 0..items.len() { ... }     // ❌

// Prefer early returns
if invalid { return Err(...); }  // ✅
if valid { ... } else { ... }    // ❌

// Use #[must_use] for important return values
#[must_use]
pub fn important() -> Result<T> { ... }
```

### Error Messages

```rust
// ✅ GOOD: Specific, actionable
TrackerError::config(
    "ALCHEMY_API_KEY must be set in .env file",
    None
)

// ❌ BAD: Vague, unhelpful
TrackerError::config("invalid config", None)
```

## Need Help?

- 💬 Open a [discussion](https://github.com/gaveesh89/eth-price-tracker/discussions)
- 🐛 Report [bugs](https://github.com/gaveesh89/eth-price-tracker/issues)
- 📧 Contact maintainers
- 📖 Read the [documentation](https://docs.rs/eth-uniswap-alloy)

## Recognition

Contributors will be:
- Listed in `CONTRIBUTORS.md`
- Mentioned in release notes
- Thanked in the README

Thank you for contributing! 🎉
