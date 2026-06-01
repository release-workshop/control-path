# Add flag lifecycle and rot-reporting surface

Status: done
Type: AFK

## What to build

Add the user-facing lifecycle workflow and reporting surface for flag rot. Git should store declared lifecycle intent, while SaaS/cache data provides observed usage signals such as last evaluation and evaluation counts.

Deprecated flags should remain available to existing code, but new rule changes should be blocked unless explicitly forced.

## Contract (from issue 01)

Declared Git metadata: `owner`, `ticket`, `expires`, `tags`, `description`, `lifecycle`, plus free-form `metadata` (no telemetry). See “Catalog vs rules vs telemetry” and “Flag model” in `.scratch/cli-salvage-redesign/schema-decisions.md`.

- `lifecycle`: `active` (default) | `deprecated`
- `kind`: `release` | `kill_switch` | `entitlement`
- Warn on missing `owner`; warn on `kind: release` without `expires`
- SaaS telemetry (`lastEvaluated`, evaluation counts, rot suggestions) is read-only — never written to `control-path.yaml`

## Acceptance criteria

- [x] `flag deprecate` sets `lifecycle: deprecated` in the repo catalog.
- [x] Deprecated flags generate CLI warnings and block new **local** rule changes unless `--force` (`flag enable`). SaaS environment rules are dashboard-owned; the CLI warns on deprecated lifecycle during SaaS `ci` / `flag report` but does not mutate remote rules.
- [x] SaaS telemetry is surfaced through reports or warnings without writing telemetry back into `control-path.yaml` or catalog `metadata`.
- [x] Removed flags are represented as retired in SaaS history while disappearing from the Git catalog.
- [x] Tests cover deprecation, forced rule changes, telemetry-backed warnings, and removed-flag retirement behavior.

## Blocked by

- `.scratch/cli-salvage-redesign/issues/05-rebuild-local-workflow-cli.md`
- `.scratch/cli-salvage-redesign/issues/06-add-saas-catalog-sync-boundary.md`
