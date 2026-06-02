# Testing and Quality Gates

Use this checklist before marking work done.

## Rust changes (`crates/compiler`, `crates/cli`, shared schemas)

Run from repo root:

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --workspace
cargo build --release --bin controlpath
```

If parallel CLI tests flake due to working-directory pollution:

```bash
cargo test --workspace -- --test-threads=1
```

## Runtime TypeScript-only changes

Run from `runtime/typescript`:

```bash
npm run lint
npm run typecheck
npm test
```

## Documentation and command updates

When behavior changes:

- update user docs under `docs/user/`
- update developer docs under `docs/developer/`
- ensure `README.md` and `DEVELOPING.md` still point to correct pages

## CI expectations

CI should enforce the same commands above. Local success should match CI expectations
to avoid merge-time surprises.
