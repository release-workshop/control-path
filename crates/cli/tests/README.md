# CLI integration tests

**Canonical guide:** [Testing in Control Path](../../../docs/developer/testing.md).

## Files

| File | Focus |
|------|--------|
| `integration_test_helpers.rs` | `TestProject`, AST/flag assertions |
| `integration_workflows.rs` | new-flag → enable → deploy workflows |
| `integration_commands.rs` | Individual commands |
| `integration_error_cases.rs` | Errors and edge cases |
| Other `integration_*.rs` | watch, SaaS, explain, lifecycle, imports, legacy prune, etc. |
| `ci_workflow_gates.rs` | CI YAML matches documented pre-merge gates |

```bash
cargo test -p controlpath-cli --test integration_workflows
```

Each test uses an isolated temp directory; see [CLI testing notes](../TESTING.md) for `assert_boolean_flag` and parallelism.
