#!/bin/bash
# Run pre-commit checks for staged files only (package/path affected).
#
# Usage: from repo root, or via .githooks/pre-commit
#   PRE_COMMIT_FULL=1  — run full workspace + runtime checks (legacy behavior)

# Copyright 2025 Release Workshop Ltd
# Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
# See the LICENSE file in the project root for details.

set -euo pipefail

die() {
  echo "❌ $*" >&2
  exit 1
}

repo_root() {
  git rev-parse --show-toplevel 2>/dev/null || die "Not inside a git repository."
}

# Sets COMPILER, CLI, WORKSPACE, TYPESCRIPT, WORKFLOWS, E2E, DOCS (true/false).
detect_staged_affected() {
  COMPILER=false
  CLI=false
  WORKSPACE=false
  TYPESCRIPT=false
  WORKFLOWS=false
  E2E=false
  DOCS=false

  local path
  while IFS= read -r path; do
    [ -z "$path" ] && continue
    case "$path" in
      crates/compiler/* | schemas/*)
        COMPILER=true
        ;;
      crates/cli/*)
        CLI=true
        ;;
      Cargo.toml | Cargo.lock)
        WORKSPACE=true
        ;;
      runtime/typescript/*)
        TYPESCRIPT=true
        ;;
      .github/workflows/*)
        WORKFLOWS=true
        ;;
      tests/e2e/*)
        E2E=true
        ;;
      docs/* | *.md)
        DOCS=true
        ;;
    esac
  done < <(git diff --cached --name-only)
}

run_rust_land_gates() {
  echo "🦀 Rust (affected packages)..."

  echo "  Checking code formatting..."
  cargo fmt --all -- --check

  local -a packages=()
  if [ "${PRE_COMMIT_FULL:-}" = "1" ] || [ "$WORKFLOWS" = true ] || [ "$WORKSPACE" = true ]; then
    echo "  cargo check / clippy / test — workspace"
    cargo check --workspace
    if cargo clippy --version &>/dev/null; then
      cargo clippy --workspace --all-targets --all-features -- -D warnings
    fi
    cargo test --workspace
    return 0
  fi

  if [ "$CLI" = true ]; then
    packages+=(controlpath-cli controlpath-compiler)
  elif [ "$COMPILER" = true ]; then
    packages+=(controlpath-compiler)
  else
    die "Rust gates requested but no Rust packages selected"
  fi

  local -a cargo_args=()
  local pkg
  for pkg in "${packages[@]}"; do
    cargo_args+=(-p "$pkg")
  done

  echo "  cargo check ${cargo_args[*]}"
  cargo check "${cargo_args[@]}"

  if cargo clippy --version &>/dev/null; then
    echo "  cargo clippy ${cargo_args[*]}"
    cargo clippy "${cargo_args[@]}" --all-targets --all-features -- -D warnings
  fi

  echo "  cargo test ${cargo_args[*]}"
  cargo test "${cargo_args[@]}"
}

run_typescript_gates() {
  echo "📦 TypeScript runtime SDK..."
  cd runtime/typescript
  [ -f package.json ] || die "runtime/typescript/package.json missing"
  npm run build
  npm run lint
  npm run typecheck
  npm test
}

main() {
  if git diff --cached --quiet; then
    echo "No staged changes to commit."
    exit 0
  fi

  cd "$(repo_root)"
  detect_staged_affected

  local need_rust=false
  local need_ts=false

  if [ "${PRE_COMMIT_FULL:-}" = "1" ]; then
    need_rust=true
    need_ts=true
  else
    if [ "$COMPILER" = true ] || [ "$CLI" = true ] || [ "$WORKSPACE" = true ] || [ "$WORKFLOWS" = true ]; then
      need_rust=true
    fi
    if [ "$TYPESCRIPT" = true ] || [ "$WORKFLOWS" = true ]; then
      need_ts=true
    fi
  fi

  if [ "$need_rust" = false ] && [ "$need_ts" = false ]; then
    if [ "$E2E" = true ]; then
      echo "ℹ️  Staged tests/e2e changes — skipping local E2E (run npm run test:smoke or rely on CI)."
    elif [ "$DOCS" = true ]; then
      echo "ℹ️  Docs-only commit — no code checks required."
    else
      echo "ℹ️  No staged paths that require pre-commit code checks."
    fi
    echo "✅ Pre-commit checks passed."
    exit 0
  fi

  echo "Verifying staged changes (affected checks only)..."
  echo "  Tip: PRE_COMMIT_FULL=1 git commit … for full workspace + runtime checks"

  if [ "$need_rust" = true ]; then
    if ! command -v cargo &>/dev/null; then
      echo "⚠️  Warning: cargo not found. Skipping Rust checks."
    else
      run_rust_land_gates
    fi
  fi

  if [ "$need_ts" = true ]; then
    if [ ! -d runtime/typescript ]; then
      echo "⚠️  Warning: runtime/typescript missing. Skipping TypeScript checks."
    elif ! command -v npm &>/dev/null; then
      echo "⚠️  Warning: npm not found. Skipping TypeScript checks."
    else
      run_typescript_gates
      cd "$(repo_root)"
    fi
  fi

  if [ "$E2E" = true ] && [ "$need_rust" = true ]; then
    echo "ℹ️  Staged tests/e2e changes — E2E smoke not run locally (see CI / npm run test:smoke)."
  fi

  echo "✅ Pre-commit checks passed."
}

main "$@"
