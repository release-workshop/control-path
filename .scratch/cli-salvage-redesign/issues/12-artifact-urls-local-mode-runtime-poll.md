# Artifact URLs and local-mode runtime polling for compiled artifacts

Status: done
Type: AFK

## What to build

End-to-end **local mode** path so services can host **environment rules** on a bucket/CDN and refresh them at runtime without SDK redeploy, while **flag catalog** changes still require `generate-sdk`.

Add `artifacts.<env>.url` to the v2 catalog (mirror `kill_switches`), wire **artifact URL** constants into the generated SDK, and poll with jitter on an interval **longer than** kill-switch polling. First load may use a bundled `.controlpath/<env>.ast` file; polling starts when `artifacts.<env>.url` is configured for that environment.

Runtime behavior must match `docs/adr/0001-compiled-artifact-runtime-delivery.md` and `CONTEXT.md`: ETag/304 (no replace or re-verify when unchanged), signature verification only on new bytes when configured, hot-swap artifact + flag index maps on success, keep last good on failure, ignore artifact flag names not in the SDK, reject polls on env mismatch or zero flag-name overlap.

## Acceptance criteria

- [x] v2 schema, catalog model, examples, and validator support `artifacts.<env>.url` in local mode and reject `artifacts` when `mode: saas`.
- [x] `build_sdk_catalog` exposes per-environment artifact URLs; TypeScript generator embeds `ARTIFACT_URLS` and starts independent artifact polling in `init` (slower default interval than kill switches; same jitter strategy).
- [x] TypeScript runtime: conditional fetch on compiled-artifact URL load, serialized refresh coordinator (commit only on successful update), and tests for 304, concurrent refresh, hot-swap evaluation, overlap rejection, and ignored extra flags.
- [x] Docs (`runtime/typescript/README.md`, SDK config docs) describe two deploy velocities: flag catalog → SDK; environment rules → artifact URL only.

## Blocked by

- `.scratch/cli-salvage-redesign/issues/11-align-typescript-runtime-with-v2-semantics.md`

## Unblocks

- `.scratch/cli-salvage-redesign/issues/13-saas-cdn-runtime-url-embedding.md`
