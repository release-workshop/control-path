#!/bin/bash
# Run pre-commit checks for staged files only (package/path affected).
#
# Usage: from repo root, or via .githooks/pre-commit
#   PRE_COMMIT_FULL=1       — run full workspace + runtime checks (legacy behavior)
#   PRE_COMMIT_SKIP_TESTS=1 — fmt, check, and clippy only (no cargo test / npm test)

# Copyright 2025 Release Workshop Ltd
# Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
# See the LICENSE file in the project root for details.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=scripts/pre-commit-test-scope.sh
source "${SCRIPT_DIR}/pre-commit-test-scope.sh"

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

ensure_typescript_runtime_for_cli_tests() {
  local need_runtime=false
  if [ "${PRE_COMMIT_FULL:-}" = "1" ] || [ "$WORKSPACE" = true ] || [ "$WORKFLOWS" = true ]; then
    need_runtime=true
  elif [ "$CLI" = true ] && [ "${PRE_COMMIT_CLI_FULL:-false}" = true ]; then
    need_runtime=true
  elif [ "${PRE_COMMIT_CLI_NEEDS_RUNTIME:-false}" = true ]; then
    need_runtime=true
  fi

  if [ "$need_runtime" != true ]; then
    return 0
  fi
  if [ -f runtime/typescript/dist/ast-loader.js ]; then
    return 0
  fi
  if ! command -v npm &>/dev/null; then
    echo "⚠️  Warning: npm not found; CLI integration tests may skip TypeScript evaluation."
    return 0
  fi
  echo "  Building TypeScript runtime (required for CLI integration tests)..."
  (cd runtime/typescript && npm ci && npm run build)
}

run_rust_pre_merge_checks() {
  echo "🦀 Rust (affected packages)..."

  echo "  Checking code formatting..."
  cargo fmt --all -- --check

  local run_compiler=false
  local run_cli=false
  local compiler_full=false
  local cli_full=false

  if [ "${PRE_COMMIT_FULL:-}" = "1" ] || [ "$WORKFLOWS" = true ] || [ "$WORKSPACE" = true ]; then
    echo "  cargo check / clippy / test — workspace"
    cargo check --workspace
    if cargo clippy --version &>/dev/null; then
      cargo clippy --workspace --all-targets --all-features -- -D warnings
    fi
    if [ "${PRE_COMMIT_SKIP_TESTS:-}" != "1" ]; then
      ensure_typescript_runtime_for_cli_tests
      cargo test --workspace
    else
      echo "  Skipping tests (PRE_COMMIT_SKIP_TESTS=1)"
    fi
    return 0
  fi

  if [ "$CLI" = true ]; then
    run_cli=true
    run_compiler=true
  elif [ "$COMPILER" = true ]; then
    run_compiler=true
  else
    die "Rust gates requested but no Rust packages selected"
  fi

  if [ "${#PRE_COMMIT_CLI_INTEGRATION_TESTS[@]}" -gt 0 ]; then
    run_cli=true
  fi

  compiler_full=$PRE_COMMIT_COMPILER_FULL
  cli_full=$PRE_COMMIT_CLI_FULL

  local -a cargo_args=()
  if [ "$run_compiler" = true ]; then
    cargo_args+=(-p controlpath-compiler)
  fi
  if [ "$run_cli" = true ]; then
    cargo_args+=(-p controlpath-cli)
  fi

  echo "  cargo check ${cargo_args[*]+"${cargo_args[*]}"}"
  cargo check ${cargo_args[@]+"${cargo_args[@]}"}

  if cargo clippy --version &>/dev/null; then
    echo "  cargo clippy ${cargo_args[*]+"${cargo_args[*]}"}"
    cargo clippy ${cargo_args[@]+"${cargo_args[@]}"} --all-targets --all-features -- -D warnings
  fi

  if [ "${PRE_COMMIT_SKIP_TESTS:-}" = "1" ]; then
    echo "  Skipping tests (PRE_COMMIT_SKIP_TESTS=1)"
    return 0
  fi

  ensure_typescript_runtime_for_cli_tests

  if [ "$run_compiler" = false ] && [ "$run_cli" = false ]; then
    die "Rust test scope produced no packages to test"
  fi

  if [ "$run_compiler" = false ]; then
    pre_commit_run_scoped_cargo_tests false "$cli_full"
  elif [ "$run_cli" = false ]; then
    pre_commit_run_scoped_cargo_tests "$compiler_full" false
  else
    pre_commit_run_scoped_cargo_tests "$compiler_full" "$cli_full"
  fi
}

run_typescript_gates() {
  echo "📦 TypeScript runtime SDK..."
  cd runtime/typescript
  [ -f package.json ] || die "runtime/typescript/package.json missing"
  npm run build
  npm run lint
  npm run typecheck
  if [ "${PRE_COMMIT_SKIP_TESTS:-}" = "1" ]; then
    echo "  Skipping tests (PRE_COMMIT_SKIP_TESTS=1)"
    return 0
  fi
  npm test
}

main() {
  if git diff --cached --quiet; then
    echo "No staged changes to commit."
    exit 0
  fi

  cd "$(repo_root)"
  detect_staged_affected
  pre_commit_collect_test_scope < <(git diff --cached --name-only)

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
  echo "  Tip: PRE_COMMIT_SKIP_TESTS=1 git commit … for fmt/check/clippy only"
  if [ "${PRE_COMMIT_SKIP_TESTS:-}" != "1" ] && [ "${PRE_COMMIT_FULL:-}" != "1" ] && [ "$WORKSPACE" != true ] && [ "$WORKFLOWS" != true ]; then
    if [ "$CLI" = true ] || [ "$COMPILER" = true ] || [ "${#PRE_COMMIT_CLI_INTEGRATION_TESTS[@]}" -gt 0 ]; then
      if [ "$PRE_COMMIT_CLI_FULL" = true ] || [ "$PRE_COMMIT_COMPILER_FULL" = true ]; then
        echo "  Test scope: full affected package(s)"
      else
        local scope_parts=()
        local item
        for item in ${PRE_COMMIT_COMPILER_FILTERS[@]+"${PRE_COMMIT_COMPILER_FILTERS[@]}"}; do
          scope_parts+=("compiler:${item}")
        done
        for item in ${PRE_COMMIT_CLI_INTEGRATION_TESTS[@]+"${PRE_COMMIT_CLI_INTEGRATION_TESTS[@]}"}; do
          scope_parts+=("cli:--test ${item}")
        done
        for item in ${PRE_COMMIT_CLI_UNIT_FILTERS[@]+"${PRE_COMMIT_CLI_UNIT_FILTERS[@]}"}; do
          scope_parts+=("cli:--lib ${item}")
        done
        if [ "${#scope_parts[@]}" -gt 0 ]; then
          echo "  Test scope: ${scope_parts[*]+"${scope_parts[*]}"}"
        fi
      fi
    fi
  fi

  if [ "$need_rust" = true ]; then
    if ! command -v cargo &>/dev/null; then
      echo "⚠️  Warning: cargo not found. Skipping Rust checks."
    else
      run_rust_pre_merge_checks
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
