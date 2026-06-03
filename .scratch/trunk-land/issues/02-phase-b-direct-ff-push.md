# Phase B: land on main via ff-push after green checks

Status: backlog

## Goal

Remove the bot merge job from `auto-merge-validation.yml` once we prove that required status checks on the validation commit SHA allow a maintainer `git push origin main` (fast-forward only) after CI passes.

## Depends on

- Phase A: `scripts/pushmain.sh` waits for CI and syncs local `main` (done).

## Acceptance

- [ ] On a test validation push, required contexts are green on the commit SHA before push to `main`.
- [ ] `pushmain` ff-pushes to `origin/main` (pre-push hook allows this path); branch protection accepts the push.
- [ ] `MAINLINE_MERGE_TOKEN` merge job removed; workflow only runs CI + optional validation branch cleanup.
- [ ] CONTRIBUTING / githooks docs updated.

## Risks

- Race if another land happens between CI green and push (script should fetch/rebase/retry).
- Docs-only: `Check Rust formatting` job + merge — implemented in validation workflow.
