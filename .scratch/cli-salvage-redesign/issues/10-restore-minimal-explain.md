# Restore minimal explain on shared semantics

Status: done
Type: AFK

## What to build

Restore a minimal `explain` command for boolean flag evaluation using shared compiler/runtime semantics, not a duplicated evaluator implementation. The command should help developers understand which rule layer matched and why.

The interactive debug UI remains deferred.

## Contract (from issue 01)

Evaluation order (local mode): **kill switch file → AST → catalog default**. See “Kill switch files” in `.scratch/cli-salvage-redesign/schema-decisions.md`.

Explain output should distinguish:

- Kill-switch file override (if present and listing the flag)
- Matched environment rule (`when`, rollout bucket, `reason`)
- Catalog default fallback
- Imported namespace flags evaluated from merged projections

## Acceptance criteria

- [x] `explain` evaluates boolean flags using shared artifact/runtime semantics.
- [x] Output shows the matched layer (kill switch, rule, or default), matched rule details, and relevant targeting/rollout reasoning.
- [x] Deprecated flags and imported namespaces are handled correctly in explanations.
- [x] The command works with locally compiled artifacts and with downloaded SaaS-compiled artifacts.
- [x] Tests cover kill-switch override, targeted rules, rollout rules, default fallback, imported flags, deprecated flags, and missing identity diagnostics.

## Blocked by

- `.scratch/cli-salvage-redesign/issues/03-compile-local-boolean-rules-to-ast.md`
- `.scratch/cli-salvage-redesign/issues/05-rebuild-local-workflow-cli.md`
- `.scratch/cli-salvage-redesign/issues/11-align-typescript-runtime-with-v2-semantics.md`
