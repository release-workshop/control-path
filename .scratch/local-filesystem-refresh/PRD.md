# Local filesystem refresh targets

Self-hosted deployments need to refresh **environment rules** and **kill switch file** state from mounted files (Docker volumes, sidecars) without HTTP object storage or app restarts.

## Source decisions

- `docs/adr/0003-local-filesystem-refresh-targets.md` (extends ADR 0001)
- `CONTEXT.md` — **artifact path**, **kill switch path**

## Scope

- Catalog `path` (POSIX absolute, XOR `url`) on `artifacts` and `kill_switches` targets
- Interval poll on `mtime` + size; last-good on failure
- Refresh-only paths; init via `init({ artifact })`
- Local mode only; SaaS unchanged

## Out of scope

- CLI publish/upload to paths
- `fs.watch` / inotify
- Windows native paths in v1
- Signature verification on file reads

## Issues

1. `issues/01-kill-switch-path-refresh.md`
2. `issues/02-artifact-path-refresh.md`
3. `issues/03-document-filesystem-refresh.md`
