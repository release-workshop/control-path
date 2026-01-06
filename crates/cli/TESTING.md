# Testing Guide for Control Path CLI

## Overview

The Control Path CLI has comprehensive test coverage including:
- **Unit tests**: Test individual functions and components (in `#[cfg(test)]` modules)
- **Integration tests**: Test complete workflows and CLI commands end-to-end (in `tests/` directory)

## Running Tests

### Run all tests
```bash
cargo test
```

### Run only unit tests
```bash
cargo test --lib
```

### Run only integration tests
```bash
cargo test --test integration_workflows
cargo test --test integration_commands
cargo test --test integration_error_cases
```

### Run a specific test
```bash
cargo test test_new_flag_workflow
```

### Run tests with output
```bash
cargo test -- --nocapture
```

## Test Structure

### Unit Tests

Unit tests are co-located with source code in `#[cfg(test)]` modules:
- `src/commands/*.rs` - Each command has unit tests
- `src/utils/*.rs` - Utility functions have unit tests
- `src/generator/*.rs` - SDK generation has unit tests

### Integration Tests

Integration tests are in the `tests/` directory:
- `tests/integration_test_helpers.rs` - Common test utilities
- `tests/integration_workflows.rs` - Complete workflow tests
- `tests/integration_commands.rs` - Individual command tests
- `tests/integration_error_cases.rs` - Error handling tests
- `tests/integration_watch.rs` - Watch mode tests (limited due to async nature)

## Test Coverage

### Commands Tested

✅ **Core Commands**:
- `validate` - Validates definitions and deployment files
- `compile` - Compiles deployment files to AST
- `generate-sdk` - Generates type-safe SDKs
- `init` - Initializes new projects
- `setup` - Complete project setup

✅ **Workflow Commands**:
- `new-flag` - Creates new flags
- `enable` - Enables flags in environments
- `deploy` - Deploys flags (validates + compiles)

✅ **Management Commands**:
- `flag add/list/show/remove` - Flag management
- `env add/sync/list` - Environment management

✅ **Debug Commands**:
- `explain` - Explains flag evaluation
- `debug` - Interactive debug UI (unit tests only)

✅ **Development Commands**:
- `watch` - File watching (unit tests + basic integration test)
- `completion` - Shell completion generation

### Test Scenarios

✅ **Success Cases**:
- All commands with valid input
- Complete workflows (new-flag → enable → deploy)
- File operations (read/write/validate)
- SDK generation
- AST compilation

✅ **Error Cases**:
- Missing files
- Invalid input
- Duplicate flags/environments
- Invalid flag/environment names
- Missing dependencies
- Invalid file formats

✅ **Edge Cases**:
- Empty files
- Missing optional parameters
- Invalid JSON/YAML
- File permission issues
- Concurrent operations

## Test Helpers

The `TestProject` helper provides:
- Temporary project directories
- File operations (read/write/check existence)
- CLI command execution
- Success/failure assertions

Example:
```rust
let project = TestProject::with_definitions(&simple_flag_definition("my_flag"));
project.run_command_success(&["validate"]);
assert!(project.file_exists("flags.definitions.yaml"));
```

## Limitations

### Watch Mode
Watch mode integration tests are limited because:
- Watch mode runs indefinitely
- Requires async file system watching
- Needs time for file changes to be detected

Watch mode is tested via:
- Unit tests for watch logic components
- Basic integration test for command structure
- Manual testing during development

### Interactive Mode
Interactive mode testing is limited because:
- Requires user input simulation
- Dialoguer library doesn't easily support programmatic input
- Best tested manually

Interactive mode is tested via:
- Unit tests for non-interactive paths
- Manual testing during development

## Adding New Tests

### Unit Test Example
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_function() {
        // Test implementation
        assert_eq!(my_function(), expected_value);
    }
}
```

### Integration Test Example
```rust
mod integration_test_helpers;
use integration_test_helpers::*;

#[test]
fn test_my_command() {
    let project = TestProject::new();
    project.run_command_success(&["my-command", "--flag", "value"]);
    assert!(project.file_exists("expected_file"));
}
```

## Best Practices

1. **Isolation**: Each test should be independent
2. **Cleanup**: Use temporary directories (automatically cleaned up)
3. **Real Operations**: Use actual file I/O (not mocked)
4. **Verify Outcomes**: Test observable results, not implementation details
5. **Error Cases**: Test both success and failure scenarios
6. **Edge Cases**: Test boundary conditions and empty inputs

## Continuous Integration

Tests should pass in CI/CD:
- All unit tests
- All integration tests
- No flaky tests
- Fast execution (< 1 minute for full suite)

## Coverage Goals

- **Unit Tests**: > 80% code coverage
- **Integration Tests**: All critical workflows covered
- **Error Cases**: All error paths tested
- **Edge Cases**: Common edge cases covered

## Coverage Reporting

### Option 1: cargo-tarpaulin (Recommended for CI/CD)

**Installation** (macOS):
```bash
# Install pkg-config (required for OpenSSL)
brew install pkg-config openssl

# Set OpenSSL directory (if needed)
export OPENSSL_DIR=$(brew --prefix openssl)

# Install cargo-tarpaulin
cargo install cargo-tarpaulin
```

**Installation** (Linux):
```bash
# Install OpenSSL development packages
sudo apt-get install libssl-dev pkg-config  # Ubuntu/Debian
# OR
sudo yum install openssl-devel pkg-config   # Fedora/RHEL

# Install cargo-tarpaulin
cargo install cargo-tarpaulin
```

**Usage**:
```bash
# Run coverage report
cargo tarpaulin --out Html --output-dir coverage

# View HTML report
open coverage/tarpaulin-report.html
```

### Option 2: cargo-llvm-cov (Alternative, no OpenSSL required)

**Installation**:
```bash
cargo install cargo-llvm-cov
```

**Usage**:
```bash
# Run coverage
cargo llvm-cov --all-features --workspace

# Generate HTML report
cargo llvm-cov --all-features --workspace --html
```

### Option 3: Use CI/CD Coverage

If local installation is problematic, coverage is automatically calculated in CI/CD:
- GitHub Actions workflow runs on push/PR
- Results available in workflow artifacts
- No local installation required

