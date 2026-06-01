# Shared explain evaluation trace

Status: done
Type: AFK

## What to build

Move evaluation tracing into the compiler/runtime layer: given **compiled artifact** bytes, **flag** name, **environment**, user attributes, optional **kill switch file** state, and catalog context for metadata → structured `ExplainTrace`.

CLI `explain` only resolves paths, loads inputs, and formats JSON/human output.

**SaaS mode (dev DX):** When `.controlpath/<env>.ast` exists (typically after sync), `explain` works without Git **environment rules**. Rule walk uses the **compiled artifact**; `reason` and **declared metadata** (`lifecycle`, etc.) come from **flag catalog** + **imports** where available. Without sync cache, fail with the same class of message as `generate-sdk`.

Trace order matches production: **kill switch file** → **compiled artifact** → catalog default. Trailing default serve rule semantics match compile (issue 06).

## Acceptance criteria

- [x] Compiler (or `catalog::explain`) exposes trace API covered by unit tests with fixture AST + catalog (no CLI required for core cases).
- [x] `explain` command shrinks to orchestration; no duplicate rule-walk logic that diverges from compile.
- [x] Tests: local mode with Git rules; SaaS mode with artifact-only rules + catalog metadata; kill switch layer; rollout bucket when applicable.
- [x] Document in issue comment or `CONTEXT.md` one paragraph on SaaS `explain` expectations (optional small edit if glossary needs it).

## Blocked by

- `.scratch/platform-spine/issues/03-catalog-orchestration-entry-points.md`
- `.scratch/platform-spine/issues/06-native-v2-compile-path.md`

## Unblocks

None required for issue 08.
