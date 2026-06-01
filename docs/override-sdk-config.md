# SDK runtime configuration (kill switches and artifacts)

Boolean kill switch files override compiled AST rules at runtime. Evaluation order: **kill switch file → AST → catalog default**.

## Deploy velocities

| Change | Ship path |
| --- | --- |
| **Flag catalog** | `controlpath generate-sdk` + app deploy |
| **Environment rules** only | Upload new `.controlpath/<env>.ast` to `artifacts.<env>.url`; SDK hot-swaps on poll |
| **Kill switch** | Upload kill switch JSON to `kill_switches.<env>.url` (faster poll) |

## Generated SDK (recommended)

In **local mode**, when `kill_switches.<env>.url` and/or `artifacts.<env>.url` are declared in `control-path.yaml`, `controlpath generate-sdk` embeds them as `KILL_SWITCH_URLS` and `ARTIFACT_URLS`.

In **SaaS mode**, the same constants are embedded from the platform CDN contract (`saas.project`, effective catalog id, environment) for each `.controlpath/<env>.ast` present when you run `generate-sdk` (usually after `controlpath ci` / SaaS sync). Avoid leaving stray `*.ast` from local `compile` unless you intend those environments embedded. Use `saas.cdn_url` for a self-hosted CDN origin; `saas.api_url` is for catalog sync only. See `docs/adr/0001-compiled-artifact-runtime-delivery.md` and `crates/compiler/src/catalog/cdn.rs`.

```typescript
import { evaluator } from '@controlpath/generated';

// Loads AST; starts kill switch polling (~30s) and artifact polling (~60s) when URLs exist.
await evaluator.init({
  artifact: './.controlpath/production.ast',
  // Optional: warn when CDN refresh fails (keeps last good artifact / kill switch file)
  logger: myLogger,
});

const enabled = await evaluator.newDashboard({ id: 'user-1', role: 'admin' });
```

**Requirements:**

- Pass `artifact` to `init()` — polling is tied to `artifact.env` and does not run without an AST path or URL.
- Remote refresh is **non-blocking** during `init()`. Kill switches poll about every 30s; compiled artifacts about every 60s (independent timers). Until the first fetch succeeds, flags use the bundled artifact and catalog defaults.
- Artifact polls use ETag / 304: unchanged remote copies do not replace or re-verify the in-memory artifact. Failed or rejected polls keep the last good artifact (rejected when `env` mismatches or zero flag-name overlap with the SDK).
- **Init guardrails** (env match + SDK flag overlap) run only when `artifacts.<env>.url` is declared for the resolved environment. Catalogs with no `artifacts` block skip strict init checks — use that for local-only workflows; add `artifacts` when you want wrong-file / wrong-object failures at startup.
- Ed25519 verification on polled artifact bytes is not wired in the generated SDK `init()` yet; configure `saas.ast_public_key` / `require_ast_signature` for download-time verification during CI sync.
- Flag keys in the kill switch JSON must match **qualified** catalog names (e.g. `platform.emergency_kill_switch` for imported flags).

### Incident runbooks

After deploy or scale-up, new processes call `init()` before the first kill switch download finishes. During that window, evaluation uses **AST + catalog defaults**, not the CDN file. Existing pods with a successful prior refresh are unaffected. For incident toggles that must apply on the first request after cold start, wait for the first successful refresh or load a local kill switch file via the low-level runtime API.

Failed CDN refreshes keep the previous in-memory file and do not throw; pass `logger` to `init()` if you want warnings in application logs.

## Low-level runtime (`@controlpath/runtime`)

For custom integrations, use `loadKillSwitchFromURL` / `loadKillSwitchFromFile` and `resolveBooleanFlag`:

```typescript
import {
  loadFromFile,
  loadFromURL,
  loadKillSwitchFromURL,
  buildFlagNameMapFromArtifact,
  resolveBooleanFlag,
} from '@controlpath/runtime';

const artifact = await loadFromFile('./.controlpath/production.ast');
const { artifact: remoteArtifact, etag } = await loadFromURL(
  'https://cdn.example.com/production/rules.ast'
);
// Pass `etag` on later polls via loadOptions; HTTP 304 yields ArtifactNotModifiedError.

const { killSwitchFile } = await loadKillSwitchFromURL(
  'https://cdn.example.com/production/kill-switches.json'
);

const flagIndex = buildFlagNameMapFromArtifact(artifact)['new_dashboard'];
const value = resolveBooleanFlag({
  qualifiedName: 'new_dashboard',
  flagIndex,
  artifact,
  catalogDefault: false,
  killSwitchFile,
  attributes: { id: 'user-1' },
});
```

Use `KillSwitchRefreshCoordinator` / `ArtifactRefreshCoordinator` with `startKillSwitchPoll` or the generic aliases `startJitteredPoll` / `pollInitDelayMs` if you implement polling yourself (same pattern as the generated SDK). Coordinators serialize overlapping fetches and only apply CDN data on successful refresh.

### v0.2 breaking change

`evaluate()` and `evaluateRule()` now coerce boolean serve payloads (`ON`/`OFF`) to `true`/`false`. Use `evaluateBoolean` or `resolveBooleanFlag` for boolean flags. See `runtime/typescript/CHANGELOG.md`.

## Local development

Serve `.controlpath/<env>.kill-switches.json` from a local HTTP server and point `kill_switches.<env>.url` at it in the catalog, then regenerate the SDK.

## Troubleshooting

- **Kill switch ignored:** JSON must use `"version": "2.0"` and boolean values under `flags`
- **Stale values:** CDN cache headers; polls run every 30s plus 0–10s jitter (and the first fetch is staggered 0–5s after `init()`)
- **Imported flag not found:** Use the qualified name (`namespace.flag_key`), not the local method name
- **Slow cold start:** `init()` no longer waits on the kill switch URL; values apply after the first successful background fetch

## See Also

- [Storage Setup Guide](./override-setup.md)
- [CLI Usage Guide](./override-cli-usage.md)
- [Runtime README](../runtime/typescript/README.md)
