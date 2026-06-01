# Issue 01 — implementer handoff

Branch: `platform-spine/01-validation-modes`

## Summary

- Added `ValidationMode` (`Authoring`, `SdkGenerate`, `Compile`) in `crates/compiler/src/catalog/validate.rs` with docs on `validate_catalog_value` / `load_and_validate_catalog`.
- Import cross-catalog rules (environment rules for imported flags) run only in `SdkGenerate` and `Compile`; `Authoring` validates schema + semantics on the document alone.
- Removed `skip_validation` / `no_validate` from CLI ops, `compile_catalog_envs`, unchecked SDK load paths, deploy flag, and GitHub Actions `skip-validation` inputs.
- `compile_catalog_envs` uses `load_catalog_bundle_for_compile` (`ValidationMode::Compile` on the post-import pass); SDK paths use `SdkGenerate`.
- SaaS CI returns the validated bundle from `load_catalog_bundle` (no unchecked re-parse).
- Local `ci` validates the catalog once via compile (`ValidationMode::Compile`), not a separate SdkGenerate load.

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

- `SdkGenerate` and `Compile` run identical checks until issue 06; modes are split so compile-only rules can land without touching SDK callers.
- See `.scratch/platform-spine/migration-01-validation-modes.md` for consumer-facing breaking changes.
- Issue 03 may further consolidate catalog entry points.
