# CLI testing (package notes)

**Canonical repo guide:** [Testing in Control Path](../../docs/developer/testing.md) (layers, CI, coverage, E2E). **Pre-merge commands:** [Testing and quality gates](../../docs/developer/testing-and-quality-gates.md).

This file only covers CLI-specific layout and helpers.

## Layout

- **Unit tests:** `src/**` in `#[cfg(test)]` modules (commands, utils, generator).
- **Integration tests:** `tests/*.rs` — subprocess `controlpath` via `TestProject` in `integration_test_helpers.rs`.

## Running CLI tests

From `crates/cli` or repo root:

```bash
cargo test -p controlpath-cli              # unit + all integration test targets
cargo test -p controlpath-cli --test integration_workflows
cargo test -p controlpath-cli test_new_flag_workflow
```

## Parallelism and `assert_boolean_flag`

Integration tests do **not** use `#[serial]` or process-wide `set_current_dir`. Unit tests that need cwd use `test_helpers::DirGuard` and `#[serial]` — use `cargo test --lib -- --test-threads=1` only if unit tests flake.

Helpers in `integration_test_helpers.rs`:

- `assert_ast_compiled(env)` — artifact exists and is non-empty
- `assert_boolean_flag(...)` — AST always; Node evaluation when `runtime/typescript/dist` exists (required in CI)

```bash
cd runtime/typescript && npm ci && npm run build
```

## Harder-to-automate areas

- **Watch mode:** long-running; covered by unit tests and basic integration structure.
- **Interactive / debug UI:** limited programmatic input; manual verification where needed.

See [`tests/README.md`](tests/README.md) for integration file names.
