# TypeScript SDK Integration

Control Path's generated SDK provides typed methods per flag and delegates runtime behavior
to `@controlpath/runtime`.

## Install runtime package

```bash
npm install @controlpath/runtime
```

## Generate SDK

From project root:

```bash
controlpath generate-sdk
```

The generated output location depends on CLI configuration and mode.

## Initialize evaluator

Typical flow:

1. Load artifact for target environment.
2. Initialize evaluator runtime.
3. Call generated methods with user/context attributes.

Generated runtime supports:

- artifact polling and refresh
- kill switch polling and refresh
- evaluation order: kill switch -> artifact rules -> catalog default

## Runtime behavior notes

- `init({ artifact })` seeds runtime state.
- Re-running `init()` without a new artifact keeps existing loaded state and restarts polling.
- Failed refresh keeps prior good state.
- Artifact and kill switch polling run on separate timers.

## Deployment velocities

- Catalog changes: regenerate SDK and redeploy app.
- Rules-only changes: publish new artifact to artifact URL.
- Incident toggles: update kill switch file for faster propagation.

## See also

- `docs/user/configuration.md`
- `docs/user/kill-switches.md`
- `runtime/typescript/README.md`
