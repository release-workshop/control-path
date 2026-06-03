# Local filesystem refresh targets

Status: accepted

Extends [0001](./0001-compiled-artifact-runtime-delivery.md). Self-hosted deployments often deliver **environment rules** and **kill switch file** updates via a mounted volume or sidecar-written file rather than HTTP object storage. The runtime must refresh from those files on the same staggered poll model as **artifact URL** and **kill switch URL**, without owning how files are published.

## Decision

1. **Product terms:** **artifact path** and **kill switch path** — absolute filesystem locations where the SDK refreshes the **compiled artifact** and **kill switch file**, respectively. See `CONTEXT.md`. Distinct from **artifact URL** / **kill switch URL** (HTTP(S) poll endpoints only).

2. **Local mode Git config:** `artifacts.<env>` and `kill_switches.<env>` accept `{ path: "/abs/path" }` or `{ url: "https://..." }`, mutually exclusive per target. Validator rejects `path` when `mode: saas`. Paths must be POSIX absolute (start with `/`); relative paths and native Windows paths are invalid in v1.

3. **Refresh-only paths:** Configured paths are refresh targets only — the same split as URLs in ADR 0001. Cold start loads via `init({ artifact })` (bundled path, mount path, or URL). The configured **artifact path** may differ from the init source (e.g. bundled `.ast` in the image, live rules on a volume). **Kill switch path** and **kill switch URL** are refresh-only; no bundled kill switch at init (first successful refresh loads state).

4. **Mechanism:** Interval poll with `mtime` and size check — not `fs.watch` / inotify. Reuse the existing jittered timers (kill switches faster than artifacts). Unchanged `mtime`/size → skip read. Same independent loops as URL polling.

5. **Failed refresh:** Missing file, I/O error, invalid bytes, or rejected guardrail → keep last-good in-memory state; log warning; evaluation continues until a later refresh succeeds.

6. **In-progress writes:** Read when `mtime`/size changes. Invalid bytes after read → last-good (no stable-size gate, no companion marker protocol). User docs recommend atomic replace (write-then-rename) for publishers.

7. **Init guardrails:** When either **artifact URL** or **artifact path** is configured for the loaded environment, `init({ artifact })` runs environment match and SDK flag-name overlap checks. The same guardrails apply before a refresh hot-swaps new artifact bytes.

8. **SDK embedding:** At `generate-sdk`, embed **artifact path** and **kill switch path** maps alongside URL maps (local mode only). Runtime selects file refresh vs URL fetch per environment from embedded config.

9. **Publish paths:** Unchanged from ADR 0001 — Control Path compiles; the developer places bytes at the **artifact path**, **kill switch path**, **artifact URL**, or **kill switch URL**. No CLI upload requirement.

## Considered options

- **Overload `url` with filesystem paths or `file://` URIs:** Rejected; blurs **artifact URL** / **kill switch URL** glossary terms and the HTTP-only schema contract.
- **Separate top-level `artifact_paths` / `kill_switch_paths` sections:** Rejected; duplicates structure without benefit over `path` on existing targets.
- **`fs.watch` or watch-with-poll-fallback:** Rejected; unreliable across bind mounts, atomic renames, and in-progress writes; converges on poll heuristics anyway.
- **Relative paths in catalog (cwd- or config-relative):** Rejected; fragile in containers where process cwd differs from repo layout.
- **Native Windows absolute paths (`C:\...`) in v1:** Rejected; Linux container mounts are the primary target; defer cross-platform path validation.
- **Configurable base path / env var for relative resolution:** Rejected; adds API and ops surface inconsistent with “hosting is the developer’s problem.”
- **Fail-open to catalog defaults on missing file:** Rejected; accidental flag flips during brief publish gaps or sidecar restarts.
- **Init auto-load from configured path when `init()` omits `artifact`:** Rejected; init behavior should not silently change based on embedded config; mirror bundled-init + remote-refresh model from ADR 0001.
- **Stable-size gate before read:** Rejected; adds latency and still fails on slow writers.

## Consequences

- Schema: extend `artifactTarget` and `killSwitchTarget` with optional `path` (XOR `url`).
- Runtime: file refresh coordinators parallel to URL refresh (`refreshArtifactFromFile`, `refreshKillSwitchFromFile`); extend `shouldValidateArtifactAtInit` to include **artifact path**.
- Generated SDK: `ARTIFACT_PATHS` and `KILL_SWITCH_PATHS` (or equivalent) embedded next to URL maps; `GeneratedEvaluatorRuntime` starts file poll loops when paths are configured.
- Compiler/CLI validation: reject relative paths; reject `path` + `url` on the same target; reject paths in SaaS mode.
- User docs: document volume-mount / sidecar pattern, absolute paths, atomic replace, and refresh-only semantics.
- ADR 0001 remains the base contract for HTTP delivery; this ADR adds the filesystem transport only.
