Status: done

# CLI integration test reliability and behavior assertions

## Parent

Derived from [Testing strategy review](../../../docs/developer/testing-strategy-review.md).

## What to build

Harden **CLI integration tests** so they are less dependent on process-wide working-directory locks and serial execution, and so critical workflows assert **observable behavior** (compiled artifacts or runtime evaluation) instead of degrading to YAML substring checks when the TypeScript runtime is available.

End-to-end outcome: `cargo test` for CLI integration tests is more parallel-friendly where safe, flaky global-state coupling is reduced, and workflow tests for enable/deploy/CI paths validate outcomes a user would care about—not incidental config text.

## Acceptance criteria

- [x] Avoidable `#[serial]` markers and global CWD mutex usage are removed or narrowed; remaining serial tests have a brief comment explaining why global state is unavoidable.
- [x] Critical workflow integration tests (enable, deploy, rule-based enable, CI happy paths) assert on AST presence, artifact semantics, or runtime evaluation when the runtime is built in CI—not only `serve: true` in YAML.
- [x] `cargo test --workspace` passes reliably in parallel; if serial-only tests remain, `AGENTS.md` / testing docs note when `--test-threads=1` is still required.
- [x] No new ignored or skipped tests introduced without documented exit criteria.

## Blocked by

None — can start immediately.

Optional coordination: if issue `01-strengthen-pre-merge-verification` ensures the runtime is built before CLI integration tests in CI, align assertion upgrades with that pipeline behavior.
