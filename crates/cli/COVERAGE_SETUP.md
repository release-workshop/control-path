# Rust coverage

Canonical policy: [Testing in Control Path](../../docs/developer/testing.md) (Rust coverage section); checklist: [Testing and quality gates](../../docs/developer/testing-and-quality-gates.md#rust-coverage-controlpath-compiler-controlpath-cli).

Quick start from repo root:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --workspace --all-features
```

Coverage runs in root CI (`main-ci`, `auto-merge-validation`); no separate workflow in this crate.
