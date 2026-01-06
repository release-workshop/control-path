# Integration Tests

This directory contains integration tests for the Control Path CLI.

## Test Structure

- `integration_test_helpers.rs` - Common test utilities and helpers
- `integration_workflows.rs` - Tests for complete workflows (new-flag → enable → deploy)
- `integration_commands.rs` - Tests for individual CLI commands
- `integration_error_cases.rs` - Tests for error handling and edge cases

## Running Tests

### Run all integration tests
```bash
cargo test --test integration_workflows
cargo test --test integration_commands
cargo test --test integration_error_cases
```

### Run all tests (unit + integration)
```bash
cargo test
```

### Run a specific test
```bash
cargo test --test integration_workflows test_new_flag_workflow
```

## Test Helpers

The `TestProject` struct provides utilities for:
- Creating temporary test projects
- Running CLI commands
- Reading/writing files
- Checking file existence
- Verifying command success/failure

## Test Coverage

Integration tests cover:
- ✅ Complete workflows (new-flag → enable → deploy)
- ✅ Individual command execution
- ✅ Error handling and edge cases
- ✅ Flag management operations (add, list, show, remove)
- ✅ Environment management operations (add, sync, list, remove)
- ✅ File I/O operations
- ✅ Command validation
- ✅ Output formats (table, JSON, YAML)
- ✅ Debug UI command structure
- ✅ Explain command edge cases (invalid JSON, missing files)

## Notes

- Tests use temporary directories that are automatically cleaned up
- Each test is independent and isolated
- Tests verify actual file operations (not mocked)
- Tests check both success and failure cases

