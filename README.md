# Control Path

Control Path is a Git-native feature flag workflow focused on release safety.
It uses a committed flag catalog, compiled per-environment artifacts, and a generated
TypeScript SDK for typed evaluation in application code.

## Status

This project is under active development. Expect breaking changes while the v2
catalog and platform-spine model continue to evolve.

## Quick Start

1. Build or install the `controlpath` CLI.
2. Run `controlpath setup`.
3. Add a flag with `controlpath new-flag my_feature`.
4. Enable it with `controlpath flag enable my_feature --env staging --all`.
5. Compile with `controlpath deploy --env staging`.
6. Regenerate the SDK with `controlpath generate-sdk`.

For full steps and examples, use the user docs below.

## User Documentation

- `docs/user/quickstart.md`
- `docs/user/cli.md`
- `docs/user/configuration.md`
- `docs/user/rules.md`
- `docs/user/sdk-typescript.md`
- `docs/user/kill-switches.md`
- `docs/user/troubleshooting.md`

## Developer Documentation

Contributors should start with `DEVELOPING.md`.

Deep-dive engineering docs:

- `docs/developer/architecture.md`
- `docs/developer/cli-internals.md`
- `docs/developer/runtime-typescript.md`
- `docs/developer/testing.md`
- `docs/developer/testing-and-quality-gates.md`
- `docs/developer/release-process.md`

Internal decision and agent docs (kept as-is):

- `docs/adr/`
- `docs/agents/`

## License

Control Path is licensed under Elastic License 2.0. See `LICENSE`.
