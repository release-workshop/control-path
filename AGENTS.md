## Agent skills

### Issue tracker

Issues and PRDs are tracked as local markdown under `.scratch/<feature-slug>/`. See `docs/agents/issue-tracker.md`.

### Triage labels

The repo uses the default five triage labels. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: read `CONTEXT.md` and `docs/adr/` at the repo root when present. See `docs/agents/domain.md`.

## Verification (required before finishing Rust changes)

Run these from the repo root after substantive edits to `crates/compiler`, `crates/cli`, or shared schemas. Do not skip because a subset “probably” suffices — match what CI runs.

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace
cargo build --release --bin controlpath
```

If `cargo test --workspace` flakes on parallel CLI unit tests (working-directory pollution), re-run with `cargo test --workspace -- --test-threads=1` to confirm; fix isolation if the failure reproduces serially.

When touching **`runtime/typescript/`** only, also run (from that directory, after `npm ci`):

```bash
npm run lint
npm run typecheck
npm test
```

Report any command that fails; do not claim the change is done until the relevant set passes.

## Idiomatic Rust

Write Rust that fits this codebase and passes `clippy` with `-D warnings`.

- **Follow existing patterns** in the crate you edit (naming, error types, module layout, `Result` propagation). Read neighbors before adding new abstractions.
- **Prefer the standard library** and workspace crates already in use; avoid one-off helpers, premature generics, or new dependencies without a clear need.
- **Errors:** use `thiserror` / `anyhow` the way surrounding code does; propagate with `?`; avoid stringly-typed errors in library code.
- **APIs:** small public surfaces, behavior-focused tests through public interfaces; keep JSON Schema + semantic validation separate where the compiler already does.
- **Ownership:** pass `&str`, `&Path`, and `&[T]` instead of owned values when callers do not need ownership; use `BTreeMap` / `BTreeSet` when stable key order matters (catalog, flags).
- **Control flow:** use `if let`, `match`, and early returns; collapse redundant `else { if ... }`; do not leave dead code, unused imports, or `#[allow(...)]` to silence warnings — delete or use the code.
- **Comments:** only for non-obvious invariants or cross-module contracts; let types and names carry intent.
- **MSRV:** respect `rust-version` in crate `Cargo.toml` (do not use APIs newer than the stated minimum without updating it deliberately).

When unsure, run `cargo clippy` and treat its suggestions as the bar for idiomatic code in this repo.
