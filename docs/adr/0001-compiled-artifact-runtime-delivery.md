# Compiled artifact runtime delivery and split deploy velocities

Status: accepted

v2 stores the **flag catalog** and **environment rules** in one `control-path.yaml`, but production still ships them on two speeds: catalog changes require SDK regeneration and app deploy; rule-only changes replace the **compiled artifact** at the **artifact URL** without rebuilding the SDK. Kill switches remain a third, faster-moving runtime file at the **kill switch URL**.

## Decision

1. **Product terms:** **compiled artifact** (rules binary), **artifact URL** (poll endpoint). Evaluation order: kill switch file → compiled artifact → catalog default.

2. **Local mode Git config:** `artifacts.<env>.url` mirrors `kill_switches.<env>.url`. Validator rejects `artifacts` when `mode: saas`.

3. **SDK polling:** When an **artifact URL** exists for the loaded environment, the SDK polls after the first load (bundled file or URL). Independent jittered loop from kill switches, with a **longer default interval** than kill switches (incidents vs rule deploys).

4. **Conditional fetch:** ETag / 304 on artifact polls. Unchanged remote → keep in-memory artifact; no signature re-verification.

5. **Signatures:** When `saas.require_ast_signature` / public key is configured, verify only on **new bytes** after a successful fetch.

6. **Hot-swap:** Successful poll replaces the in-memory compiled artifact and rebuilds flag index maps (no restart).

7. **Migration guardrails:** Older SDKs may consume newer artifacts; flag names in the artifact that are not in the generated SDK are **ignored**. Reject a poll (keep last good) on environment mismatch or **zero** flag-name overlap with the SDK.

8. **SaaS URLs:** Not in Git. At `generate-sdk`, embed **artifact URL** and **kill switch URL** per environment with a `.controlpath/<env>.ast` on disk (typically from `saas sync`; sync prunes stale files on download — see `crates/compiler/src/catalog/cdn.rs`), using a stable platform CDN path contract (`saas.project`, catalog identity, environment).

9. **Publish paths:** Local rule changes → compile/deploy → upload to **artifact URL**. SaaS rule changes → platform compiles → CDN → SDK poll. **Flag catalog** changes are never applied by artifact replacement alone.

## Considered options

- **Poll only when `init` uses HTTPS:** Rejected; bundled file + remote URL is the common prod pattern.
- **Single combined poll tick** for kill switches and artifacts: Rejected; independent timers avoid one slow CDN blocking the other.
- **Reject artifacts with extra flag names (strict B):** Rejected; breaks forward-compatible rollout (old SDK, new rules).
- **Catalog identity embedded in artifact binary:** Deferred; overlap + env checks are sufficient for now.

## Consequences

- Issue 11 delivered kill-switch polling; **artifact polling** and `artifacts` schema are follow-up work (issues 12–13).
- `loadFromURL` for compiled artifacts must gain ETag support (kill-switch loader already has it).
- SaaS `generate-sdk` must embed CDN URLs for kill switches as well as artifacts (symmetry; local mode already embeds kill switches from Git).
- Optional later ADR if the artifact wire format gains an embedded **catalog identity** field for stronger mismatch detection.
