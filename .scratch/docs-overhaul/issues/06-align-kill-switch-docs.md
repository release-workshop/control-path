# Align docs to kill-switch model

Status: ready-for-agent
Type: AFK

## Parent

- `.scratch/docs-overhaul/issues/01-structure-docs-reset.md`

## What to build

Update documentation to reflect the kill-switch-only model and strict command semantics. This slice should rename and rewrite user-facing guidance so docs match current CLI behavior and the domain glossary, with no remaining legacy override concepts.

## Acceptance criteria

- [ ] `docs/user/overrides.md` is replaced by `docs/user/kill-switches.md` and all references are updated.
- [ ] README and relevant user/developer pages reference canonical kill-switch terminology and command forms.
- [ ] Docs describe strict semantics introduced in prior slices (local-mode mutating operations, explicit `--env`, and valid kill-switch flag targeting).
- [ ] Examples and troubleshooting guidance are copy/paste accurate for the current CLI.
- [ ] A doc review pass confirms no legacy "override" guidance remains in user/developer docs for this feature area.

## Blocked by

- `.scratch/docs-overhaul/issues/04-canonicalize-kill-switch-cli.md`
- `.scratch/docs-overhaul/issues/05-enforce-kill-switch-contract.md`
