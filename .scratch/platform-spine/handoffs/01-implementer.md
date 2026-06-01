# Issue 01 — implementer handoff

Branch: `platform-spine/01-validation-modes`

## Summary

- Added `ValidationMode` (`Authoring`, `SdkGenerate`, `Compile`) in `crates/compiler/src/catalog/validate.rs` with docs on `validate_catalog_value` / `load_and_validate_catalog`.
- Import cross-catalog rules (environment rules for imported flags) run only in `SdkGenerate` and `Compile`; `Authoring` validates schema + semantics on the document alone.
- Removed `skip_validation` / `no_validate` from CLI ops, `compile_catalog_envs`, unchecked SDK load paths, deploy flag, and GitHub Actions `skip-validation` inputs.
- `compile_catalog_envs` now uses `load_validated_catalog_bundle` (full validation) and derives target envs from the parsed catalog.

## Tests run

```bash
cargo fmt --all && cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace
cargo build --release --bin controlpath
```

All passed.

## Acceptance criteria

- [x] `ValidationMode` exposed and documented next to validate entry points
- [x] No `skip_validation` / `no_validate` on compile, SDK, CI, workflow, dev paths
- [x] CLI flags removed; integration tests updated
- [x] Unit tests per mode (`validation_mode_tests.rs`)
- [x] Workspace tests + clippy green

## Known gaps

- GitHub Actions dropping `skip-validation` is a breaking change for workflow YAML that set it (intentional per PRD).
- `load_sdk_catalog_unchecked` removed; issue 03 may further consolidate catalog entry points.
