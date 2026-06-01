# Catalog document store for control-path.yaml authoring

Status: done
Type: AFK

## What to build

Add a CLI **catalog document store** module that is the single seam for reading and mutating the service `control-path.yaml`: load from disk → parse → validate (per issue 01 modes) → typed `CatalogDocument` in memory → mutate → full YAML re-serialize → atomic write.

Migrate flag and environment authoring commands off `unified_config` (`serde_json::Value` tree walks). `unified_config` shrinks to path helpers or is deleted if unused.

Re-serialization may reorder keys and drop comments; that is acceptable.

## Acceptance criteria

- [x] Store API supports at least: open/load, save, and mutations used by `flag` and `env` commands today (add/update/remove flag, environment rules where applicable in local mode).
- [x] Invalid mutations fail before write with the same diagnostics as compiler validation.
- [x] `flag add` / `flag set` (or equivalent) and one `env` workflow use only the store — no `read_unified_config` + separate `load_sdk_catalog` double-read in one command.
- [x] Integration or unit tests: round-trip edit on a fixture catalog; rejected invalid edit does not touch disk.
- [x] No remaining production use of `unified_config::read_unified_config` for catalog mutation (grep-clean or allowlist documented).

## Blocked by

- `.scratch/platform-spine/issues/01-validation-modes-replace-skip-validation.md`

## Unblocks

- `.scratch/platform-spine/issues/03-catalog-orchestration-entry-points.md`
