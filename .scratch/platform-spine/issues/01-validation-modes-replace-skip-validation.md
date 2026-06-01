# Replace skip_validation with explicit validation modes

Status: done
Type: AFK

## What to build

Introduce explicit **validation modes** in the compiler catalog layer (e.g. `Authoring`, `SdkGenerate`, `Compile`) that encode which checks run: JSON Schema, semantic rules, import resolution semantics. Remove ad hoc `skip_validation: bool` from CLI compile, generate-sdk, workflow, and `ci --no-validate`.

Every user-facing compile and SDK generation path uses full validation. The previous “skip schema but still validate imports” behaviour is folded into a named mode only if still required internally for tests; it is not exposed on CLI flags.

## Acceptance criteria

- [x] Compiler exposes a small `ValidationMode` (or equivalent) used by `validate_catalog` / load helpers; modes are documented next to `validate_catalog`.
- [x] `compile_catalog_envs`, `load_sdk_catalog*`, and `ci` / `workflow` / `dev` / `compile` / `generate-sdk` no longer take or pass `skip_validation` / `no_validate`.
- [x] CLI flags `--skip-validation` and `ci --no-validate` are removed; help text and integration tests updated.
- [x] Unit test per mode on a fixture catalog: schema failure, semantic failure, and import rule failure are rejected under `Compile` and `SdkGenerate`.
- [x] `cargo test --workspace` and clippy pass.

## Blocked by

None — can start immediately.

## Unblocks

- `.scratch/platform-spine/issues/02-catalog-document-store.md`
- `.scratch/platform-spine/issues/03-catalog-orchestration-entry-points.md`
