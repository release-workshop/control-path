# Runtime loader hardening and generated SDK contract coverage

Status: done
Type: AFK

## Parent

- `.scratch/platform-spine/issues/08-thin-generated-sdk-deep-runtime.md`

## What to build

Deliver one follow-up hardening slice after issue 08 that keeps the generated SDK thin while tightening runtime internals and contract protection.

Consolidate shared URL fetch/path-validation mechanics between compiled artifact and kill-switch loaders where safe, without changing public behavior, and keep **independent poll timers** per ADR-0001.

Lock in generated SDK behavior with regression tests for per-instance evaluator isolation and `init()` re-entry semantics, and align docs/issue wording with the shipped `0.3` runtime contract.

## Acceptance criteria

- [x] Runtime loaders share a common helper for overlapping URL/path handling concerns (timeouts, redirects, conditional request plumbing, path-validation primitives) while preserving current loader-specific errors and payload validation.
- [x] ADR-0001 runtime behavior is unchanged: kill-switch and artifact polling remain independent loops with existing interval/jitter defaults.
- [x] Generated SDK contract tests cover multi-instance isolation (`new Evaluator()` instances and `evaluator` export) so state/timers do not leak across instances.
- [x] Generated/runtime tests explicitly protect `init()` no-artifact behavior to avoid accidental state clearing regressions.
- [x] Runtime docs and issue wording reflect delivered `0.3` shape (thin generated delegation, `SDK_QUALIFIED_FLAG_NAMES` naming, and this follow-up scope).
- [x] Verification passes: `runtime/typescript` lint/typecheck/test and required Rust workspace checks used by this repo.

## Blocked by

- `.scratch/platform-spine/issues/08-thin-generated-sdk-deep-runtime.md`

## Unblocks

None — hardening and contract lock-in follow-up.
