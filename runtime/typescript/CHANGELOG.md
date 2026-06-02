# Changelog

## 0.3.0

### Breaking changes

- Generated SDKs now depend on `^0.3.0` and delegate init, polling, and guardrails to `GeneratedEvaluatorRuntime` instead of inlining coordinators in `index.ts`.

### Added

- `GeneratedEvaluatorRuntime` — catalog-scoped init, independent kill-switch and artifact polling, and `evaluateBooleanFlag` for generated SDK method bodies
- `DEFAULT_GENERATED_*` poll interval constants shared with generated `index.ts`

### Changed

- Generated `index.ts` is mostly embedded URLs, flag metadata, and thin delegation to the runtime module
- `GeneratedEvaluatorRuntime.init()` without `artifact` after a loaded artifact preserves kill-switch coordinator state and restarts polling (logger-only re-init)
- Failed `init({ artifact })` restores the prior artifact, coordinators, and flag index instead of leaving empty coordinators with stale runtime fields

## 0.2.0

### Breaking changes

- **`evaluate()` / `evaluateRule()`** — Boolean serve and rollout payloads from v2 ASTs are coerced to `true`/`false` (previously returned `'ON'`/`'OFF'` strings from the string table). Code comparing against string literals will fail silently.
- **Use instead:** `evaluateBoolean()` (AST only) or `resolveBooleanFlag()` (kill switch → AST → catalog default).

### Added

- Kill switch file loader: `loadKillSwitchFromFile`, `loadKillSwitchFromURL`, `KillSwitchFileNotModifiedError`
- `resolveBooleanFlag`, `refreshKillSwitchFromUrl`, `startKillSwitchPoll`
- Generated SDK embeds `KILL_SWITCH_URLS` from `kill_switches.<env>.url`
- Compiled artifact polling: `loadFromURL` ETag / 304, `ArtifactNotModifiedError`, `ArtifactRefreshCoordinator`, `validateArtifactPoll`, `assertArtifactAccepted`, `resolveExpectedArtifactEnv`
- `startJitteredPoll` / `pollInitDelayMs` aliases for shared jittered polling
- Generated SDK embeds `ARTIFACT_URLS` from `artifacts.<env>.url` (60s poll, independent of kill switches)

### Removed

- Multivariate public API: `RuleType.VARIATIONS`, `Variation`, `isVariation`, and variation-shaped `Rule` union members. Legacy artifacts with rule type `1` are rejected by `isRule` and ignored by `evaluateRule`.

### Changed

- **`loadFromURL`** returns `{ artifact, etag? }` instead of a bare `Artifact`; HTTP 304 without `If-None-Match` is an error (not `ArtifactNotModifiedError`)
- Generated `init()` validates env/overlap when `artifacts.<env>.url` is configured (same guardrails as poll)
- Generated `init()` does not await kill switch fetch (background refresh + 30s polling)
- Generated `init({ logger })` passes an optional logger to kill switch refresh; failed CDN fetches log a warning and retain prior state
- `KillSwitchRefreshCoordinator` serializes overlapping refreshes and only applies state on successful CDN updates (avoids stale failed requests overwriting newer data)
- Kill switch polling uses jittered `setTimeout` chains (default 30s + 0–10s) and a 0–5s stagger on the first post-`init()` fetch to reduce CDN thundering herd

## 0.1.0

Initial release.
