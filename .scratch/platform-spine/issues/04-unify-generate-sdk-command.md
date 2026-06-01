# Unify generate-sdk command and ops helper

Status: ready-for-agent
Type: AFK

## What to build

Make `controlpath generate-sdk` a thin wrapper around `ops::generate_sdk_helper`: one output-path resolver, one catalog load entry point (`load_for_sdk_generate` from issue 03), one validation mode (`SdkGenerate`).

Remove duplicated logic in `commands/generate_sdk.rs` (path defaults, catalog load, language detection). JSON and human exit formatting stay in the command module.

## Acceptance criteria

- [ ] `commands/generate_sdk` delegates to ops; no duplicate `determine_output_path` / loader branching.
- [ ] CLI and `ci` / `dev` / `workflow` / `setup` paths that generate SDKs behave identically before/after for existing integration tests.
- [ ] `integration_saas` and local generate-sdk tests still pass.

## Blocked by

- `.scratch/platform-spine/issues/03-catalog-orchestration-entry-points.md`

## Unblocks

- `.scratch/platform-spine/issues/08-thin-generated-sdk-deep-runtime.md`
