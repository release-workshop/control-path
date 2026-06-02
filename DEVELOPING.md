# Developing Control Path

This is the root contributor guide for the Control Path monorepo.

## Prerequisites

- Rust toolchain (via `rustup`)
- Node.js 24+ and `npm`

Optional but recommended:

- Git hooks setup: `bash scripts/setup-git-aliases.sh`

## Local Setup

From repo root:

```bash
cargo build --workspace
cargo test --workspace
```

For TypeScript runtime work:

```bash
cd runtime/typescript
npm ci
npm run build
npm test
```

## Repo Shape

- `crates/compiler`: catalog parsing, validation, and artifact compile pipeline
- `crates/cli`: `controlpath` command orchestration and workflows
- `runtime/typescript`: low-level runtime loader/evaluator + generated runtime glue
- `schemas`: catalog, artifact, and runtime JSON schema material

## Core Concepts

- Flag catalog in `control-path.yaml` is the source of truth for flag definitions.
- Environment rules compile to `.controlpath/<env>.ast` artifacts.
- Generated SDK wraps runtime internals and exposes typed evaluator methods.
- Runtime evaluation order is kill switch file -> compiled artifact -> catalog default.

## Contributor Workflows

Common tasks:

- Add or refine command behavior in CLI: see `docs/developer/cli-internals.md`
- Change runtime loading or evaluator contract: see `docs/developer/runtime-typescript.md`
- Update architecture docs or boundaries: see `docs/developer/architecture.md`

## Verification Before Finishing

When editing `crates/compiler`, `crates/cli`, or shared schemas, run from repo root:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace
cargo build --release --bin controlpath
```

When touching only `runtime/typescript`, run from `runtime/typescript`:

```bash
npm run lint
npm run typecheck
npm test
```

## Documentation Map

- `docs/developer/architecture.md`
- `docs/developer/cli-internals.md`
- `docs/developer/runtime-typescript.md`
- `docs/developer/testing-and-quality-gates.md`
- `docs/developer/release-process.md`

Decision records and agent-facing process docs:

- `docs/adr/`
- `docs/agents/`
