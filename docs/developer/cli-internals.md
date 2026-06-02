# CLI Internals

`controlpath` is implemented in `crates/cli` and combines workflow-oriented commands with
lower-level primitives.

## Command topology

Workflow commands compose core operations:

- `setup`
- `new-flag`
- `flag enable`
- `deploy`
- `dev`

Core operations exposed directly:

- `validate`
- `compile`
- `generate-sdk`

Management and diagnostics:

- `flag`, `env`, `kill-switch`
- `explain`, `debug`, `watch`, `ci`, `completion`

## Design goals

- Keep user workflows short and opinionated.
- Preserve lower-level commands for scripting and CI.
- Make command output compatible with human and machine consumers (`--json`).

## Compiler integration boundary

CLI should not duplicate compiler semantics. It should:

- load files and user input
- invoke compiler validation/compile entrypoints
- map errors into actionable command-line messages

Semantic validation rules live in compiler crates.

## SaaS vs local behavior

CLI command behavior is mode-aware:

- local mode owns environment rules in Git
- SaaS mode treats environment rules as platform-owned
- explain/generate paths use synced artifact state where applicable

## Adding or changing commands

When extending commands:

- preserve existing workflows unless intentionally breaking
- update command help text and docs simultaneously
- cover changes with integration-style CLI tests where possible
