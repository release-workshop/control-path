# Parse and validate the new catalog schema

Status: done
Type: AFK

## What to build

Implement typed parsing and validation for the approved `control-path.yaml` schema in the shared compiler/domain layer. Invalid catalogs should fail with actionable diagnostics, and valid catalogs should load into typed Rust structures instead of ad hoc JSON mutation.

This slice establishes the schema as a shared contract for the CLI and future SaaS compiler worker.

## Contract (from issue 01)

Read `.scratch/cli-salvage-redesign/schema-decisions.md` before implementing. Do not reopen product semantics settled there.

| Artifact | Purpose |
|---|---|
| `schemas/control-path.schema.v2.json` | Service `control-path.yaml` |
| `schemas/control-path.workspace.schema.v1.json` | Monorepo `control-path.workspace.yaml` |
| `schemas/examples/*.yaml` | Representative fixtures for unit tests |

**v2 shape (replaces v1):** map-keyed `flags`, top-level `environments.<env>.rules.<flag>`, optional top-level `segments`, explicit `catalog` + `imports`, native boolean `default` (no `type`, `defaultValue`, or `ON`/`OFF`).

JSON Schema covers structure only. Semantic rules listed under “Validation beyond JSON Schema” in `schema-decisions.md` must be enforced in Rust (including SaaS-mode rejection of `environments`, `segments`, and `kill_switches`).

## Acceptance criteria

- [x] The compiler/shared layer parses the v2 boolean-only catalog into typed structures (map-keyed flags, top-level environments, catalog identity, imports).
- [x] Workspace files parse and validate; namespace resolution follows `catalog.namespace` → workspace walk-up → bare `catalog.id`.
- [x] JSON Schema validation runs against v2 and workspace schemas before semantic checks.
- [x] Validation rejects unsupported v1 fields (`type`, `variations`, per-flag `environments`, variant/experiment concepts), invalid `kind`/`lifecycle` values, duplicate import namespaces, local rules in SaaS mode, `kill_switches` in SaaS mode, imported-flag environment rules in consuming catalogs, and local flag keys that collide with import namespace prefixes.
- [x] Validation rejects `kind: kill_switch` rules that use `when` or `rollout` (serve-only).
- [x] Validation rejects SaaS telemetry fields in catalog `metadata`.
- [x] Validation warns, without hard failing by default, for missing `owner` and for `kind: release` without `expires`.
- [x] Unit tests use `schemas/examples/` fixtures and additionally cover: invalid SaaS-local-rules, invalid imports, deprecated flags, kill-switch rule constraints, namespace resolution, and metadata warning cases.

## Deliverables

- Typed catalog/workspace structs and parser in the compiler crate
- v2 JSON Schema wired into compiler validation (alongside semantic validator)
- Unit tests with fixtures derived from `schemas/examples/`

## Blocked by

None — issue 01 is complete.
