# Enforce kill-switch operational contract

Status: ready-for-agent
Type: AFK

## Parent

- `.scratch/docs-overhaul/issues/01-structure-docs-reset.md`

## What to build

Tighten kill-switch command semantics so incident operations are explicit and safe. This slice should enforce mode boundaries and target validity at command execution time, preventing ambiguous or semantically invalid kill-switch writes.

## Acceptance criteria

- [ ] Mutating kill-switch commands (`set`, `clear`) are restricted to local mode and fail with actionable messaging in SaaS mode.
- [ ] Kill-switch commands require explicit `--env` for target environment selection.
- [ ] `kill-switch set` validates that the target flag exists in the catalog.
- [ ] `kill-switch set` validates that the target flag is `kind: kill_switch` before writing values.
- [ ] Tests cover mode restrictions, explicit env requirements, and flag kind/existence validation.

## Blocked by

- `.scratch/docs-overhaul/issues/04-canonicalize-kill-switch-cli.md`
