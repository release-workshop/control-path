# Structure docs reset

Status: done
Type: AFK

## What to build

Establish the new documentation information architecture for the repo with two audience entrypoints and intentional hard replacement of outdated docs.

Deliver the canonical structure with `README.md` as the user-facing home, `DEVELOPING.md` as the developer-facing home, user docs under `docs/user/`, and developer docs under `docs/developer/`. Keep `docs/adr/` and `docs/agents/` untouched.

## Acceptance criteria

- [x] New documentation structure is in place with canonical entrypoints for user and developer audiences.
- [x] `README.md` links to user documentation and `DEVELOPING.md`; `DEVELOPING.md` links to relevant developer deep-dive pages.
- [x] Legacy docs targeted for replacement are removed or relocated as part of the clean break.
- [x] `docs/adr/` and `docs/agents/` remain untouched.

## Blocked by

None - can start immediately.
