# Canonicalize kill-switch CLI

Status: ready-for-agent
Type: AFK

## Parent

- `.scratch/docs-overhaul/issues/01-structure-docs-reset.md`

## What to build

Remove the legacy override model from the CLI and code terminology so `kill-switch` is the only supported concept. This slice should deliver a clean, single command model that no longer exposes legacy aliases or non-functional compatibility surfaces.

## Acceptance criteria

- [ ] `controlpath override ...` is no longer accepted; `kill-switch` is the canonical command surface.
- [ ] Internal CLI command names and modules no longer use override terminology.
- [ ] Non-functional compatibility surfaces are removed (including legacy flags that are currently ignored and `history` behavior that does not provide true history).
- [ ] CLI help text and user-facing command descriptions consistently use kill-switch terminology.
- [ ] Existing and new tests cover removal of legacy command paths and verify canonical kill-switch behavior.

## Blocked by

None - can start immediately.
