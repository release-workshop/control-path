# @controlpath/runtime

Low-level runtime SDK for Control Path. Loads compiled AST artifacts and kill switch files, then resolves boolean flags in product evaluation order.

## Installation

```bash
npm install @controlpath/runtime
```

## Migrating to 0.2

`evaluate()` now returns booleans for v2 serve/rollout rules instead of `'ON'`/`'OFF'`. Update comparisons or switch to:

- `evaluateBoolean()` — AST rules only
- `resolveBooleanFlag()` — kill switch → AST → catalog default

See [CHANGELOG.md](./CHANGELOG.md).

## Evaluation order

1. **Kill switch file** — if the flag appears in the loaded file, its boolean value wins and AST rules are skipped.
2. **Compiled AST** — environment rules from `controlpath compile`.
3. **Catalog default** — from `control-path.yaml` when nothing else applies.

## Deploy velocities

| Change | How it ships |
| --- | --- |
| **Flag catalog** (flags, defaults, kinds, imports) | Regenerate SDK + redeploy the app |
| **Environment rules** only | Replace the compiled artifact at the **artifact URL** (SDK polls; no SDK rebuild) |
| **Kill switch** (incident) | Kill switch file at the kill switch URL (faster poll than artifacts) |

In local mode, declare `artifacts.<env>.url` in `control-path.yaml` (like `kill_switches`). The generated SDK embeds `ARTIFACT_URLS` and polls after the first load from a bundled `.controlpath/<env>.ast` or URL. Init env/overlap validation runs only when those URLs exist; omit `artifacts` for local-only projects that do not need remote rule refresh or strict init checks.

## Usage

### AST + boolean resolution

```typescript
import {
  loadFromFile,
  buildFlagNameMapFromArtifact,
  resolveBooleanFlag,
} from '@controlpath/runtime';

const artifact = await loadFromFile('./.controlpath/production.ast');
const flagNameMap = buildFlagNameMapFromArtifact(artifact);

const attributes = { id: 'user123', role: 'admin' };
const enabled = resolveBooleanFlag({
  qualifiedName: 'new_dashboard',
  flagIndex: flagNameMap['new_dashboard'],
  artifact,
  catalogDefault: false,
  attributes,
});
```

### Kill switch files

Kill switch files use the v2 boolean map (`schemas/examples/production.kill-switches.json`):

```typescript
import { loadKillSwitchFromFile, loadKillSwitchFromURL } from '@controlpath/runtime';

const local = await loadKillSwitchFromFile('.controlpath/production.kill-switches.json');

const { killSwitchFile, etag } = await loadKillSwitchFromURL(
  'https://flags.example.com/production/kill-switches.json'
);
```

Poll with the returned `etag` on later requests; `KillSwitchFileNotModifiedError` indicates HTTP 304 (no change).

### Generated SDK

Prefer the generated `@controlpath/generated` evaluator: it embeds `kill_switches.<env>.url` as `KILL_SWITCH_URLS` and `artifacts.<env>.url` as `ARTIFACT_URLS`, polls both in the background after `init()`, and calls `resolveBooleanFlag` for each flag method.

`init()` does not wait for the first remote fetch. Kill switches poll about every 30s (+ jitter); compiled artifacts poll about every 60s (+ jitter) on an independent timer. Until a refresh succeeds, flags use the last loaded artifact and kill switch state (or AST/catalog defaults on cold start). See [SDK configuration](../docs/override-sdk-config.md).

Pass an optional `logger` to `init({ artifact, logger })` to emit warnings when a refresh fails (prior state is retained).

## API reference

### Loading

- `loadFromFile`, `loadFromURL`, `loadFromBuffer` — compiled artifacts (`loadFromURL` returns `{ artifact, etag? }`; ETag / 304 when `loadOptions.etag` is set)
- `loadKillSwitchFromFile`, `loadKillSwitchFromURL` — kill switch JSON (v2 boolean map)

### Evaluation

- `resolveBooleanFlag` — kill switch → AST → catalog default
- `evaluateBoolean` — AST rules only (boolean coercion for ON/OFF serve payloads)
- `evaluate`, `evaluateRule` — low-level AST evaluation

### Types

- `KillSwitchFile`, `KillSwitchRefreshState` — kill switch file shape and polling state
- `KillSwitchRefreshCoordinator` — serialized kill switch refresh (only commits on `status === 'updated'`)
- `ArtifactRefreshCoordinator`, `refreshArtifactFromUrl`, `validateArtifactPoll`, `assertArtifactAccepted` — compiled artifact polling and init guardrails
- `startJitteredPoll`, `pollInitDelayMs` — generic jittered poll helpers (aliases of kill-switch names)
- `refreshKillSwitchFromUrl`, `startKillSwitchPoll` — low-level fetch and interval helper
- `Artifact`, `Rule` (serve and rollout only), `Expression`, `Attributes` — AST types

## Development

```bash
npm run build
npm test
npm run typecheck
npm run lint
```

## License

Elastic License 2.0
