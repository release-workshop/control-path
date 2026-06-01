# Migration: catalog document store (issue 02)

## Authoring vs read paths

| Before | After |
|--------|--------|
| `unified_config::read_unified_config` + manual `Value` edits | `CatalogStore::open` → mutate → `save` |
| `flag add` post-check via `load_sdk_catalog` | `save` then `validate_sdk_generate()` (skipped when `--lang`; regen loads catalog) |
| `workflow new-flag` / `enable` | `save` (Authoring) + `validate_sdk_generate_if_imported()` when `imports` non-empty |
| `flag remove` / `deprecate`, `env add` / `remove` | `save` (Authoring only); same as before (no post-`load_sdk_catalog`) |
| `env sync` via `read_unified_config` + `load_sdk_catalog` | Single `load_catalog_bundle` (SdkGenerate) |

## Stricter open

`CatalogStore::open` requires **Authoring** validation to pass. Schema-invalid `control-path.yaml` files can no longer be edited with `flag` / `env` until fixed out-of-band. This is intentional hardening.

## What round-trips

- Typed catalog fields (`flags`, `environments`, `imports`, etc.)
- Top-level extension keys **outside** the catalog schema (e.g. `sdk.output`) via the `preserved` map

## What does not round-trip

- YAML comments and key order (PRD decision)
- **Flag-level** keys not on `FlagDefinition` (`#[serde(deny_unknown_fields)]`). Custom flag metadata that survived `read_unified_config` + `Value` edits is dropped on open. Use `metadata` on the flag definition if you need extension fields in-schema.

## YAML noise

Re-serialization may emit defaulted fields (e.g. `lifecycle: active`) and reorder keys. Expect noisier diffs in Git.
