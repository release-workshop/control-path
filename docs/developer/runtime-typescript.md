# Runtime TypeScript Internals

The runtime package lives in `runtime/typescript` and provides artifact/kill-switch loading,
polling coordination, and evaluation helpers used by generated SDK output.

## Main responsibilities

- Parse and validate compiled artifact payloads.
- Load kill switch files (local and remote).
- Manage refresh state with ETag-aware polling.
- Evaluate boolean flags with deterministic precedence.
- Expose `GeneratedEvaluatorRuntime` for generated SDK delegation.

## Key contract points

- Generated SDK remains thin and forwards to runtime APIs.
- `init({ artifact })` seeds runtime state for first use.
- Re-init without artifact preserves loaded runtime data and restarts background loops.
- Failed updates keep last good state rather than clearing evaluator state.

## Polling model

- kill switch and artifact refresh loops run independently
- defaults differ by concern (kill switches poll more aggressively)
- jitter and init spread avoid synchronized fleet spikes

## Testing focus

Runtime tests should prioritize contract behavior:

- evaluator instance isolation
- init/re-init state semantics
- refresh coordinator update/rollback behavior
- loader validation and path/url hardening

## Working in this package

From `runtime/typescript`:

```bash
npm ci
npm run lint
npm run typecheck
npm test
```

Update `CHANGELOG.md` and package docs when changing runtime behavior.
