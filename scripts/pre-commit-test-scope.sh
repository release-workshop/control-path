#!/bin/bash
# Map staged paths to scoped cargo test invocations for pre-commit.
#
# Sourced by scripts/run-pre-commit-checks.sh — do not execute directly.

# Copyright 2025 Release Workshop Ltd
# Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
# See the LICENSE file in the project root for details.

# Populated by pre_commit_collect_test_scope:
#   PRE_COMMIT_CLI_FULL, PRE_COMMIT_COMPILER_FULL (true/false)
#   PRE_COMMIT_CLI_INTEGRATION_TESTS (array of integration_*.rs stem names)
#   PRE_COMMIT_CLI_UNIT_FILTERS (array of cargo test name filters)
#   PRE_COMMIT_COMPILER_FILTERS (array of cargo test name filters)
#   PRE_COMMIT_CLI_NEEDS_RUNTIME (true when workflow integration tests may run)

PRE_COMMIT_CLI_FULL=false
PRE_COMMIT_COMPILER_FULL=false
PRE_COMMIT_CLI_INTEGRATION_TESTS=()
PRE_COMMIT_CLI_UNIT_FILTERS=()
PRE_COMMIT_COMPILER_FILTERS=()
PRE_COMMIT_CLI_NEEDS_RUNTIME=false

# Bash 3.2 + set -u treats "${array[@]}" as unbound when array is empty.
_pre_commit_add_cli_integration() {
  local value=$1
  local existing
  for existing in ${PRE_COMMIT_CLI_INTEGRATION_TESTS[@]+"${PRE_COMMIT_CLI_INTEGRATION_TESTS[@]}"}; do
    if [ "$existing" = "$value" ]; then
      return 0
    fi
  done
  PRE_COMMIT_CLI_INTEGRATION_TESTS+=("$value")
  if [ "$value" = "integration_workflows" ]; then
    PRE_COMMIT_CLI_NEEDS_RUNTIME=true
  fi
}

_pre_commit_add_cli_unit_filter() {
  local value=$1
  local existing
  for existing in ${PRE_COMMIT_CLI_UNIT_FILTERS[@]+"${PRE_COMMIT_CLI_UNIT_FILTERS[@]}"}; do
    if [ "$existing" = "$value" ]; then
      return 0
    fi
  done
  PRE_COMMIT_CLI_UNIT_FILTERS+=("$value")
}

_pre_commit_add_compiler_filter() {
  local value=$1
  local existing
  for existing in ${PRE_COMMIT_COMPILER_FILTERS[@]+"${PRE_COMMIT_COMPILER_FILTERS[@]}"}; do
    if [ "$existing" = "$value" ]; then
      return 0
    fi
  done
  PRE_COMMIT_COMPILER_FILTERS+=("$value")
}

_pre_commit_mark_cli_full() {
  PRE_COMMIT_CLI_FULL=true
  PRE_COMMIT_CLI_INTEGRATION_TESTS=()
  PRE_COMMIT_CLI_UNIT_FILTERS=()
}

_pre_commit_mark_compiler_full() {
  PRE_COMMIT_COMPILER_FULL=true
  PRE_COMMIT_COMPILER_FILTERS=()
}

_pre_commit_map_cli_path() {
  local path=$1

  case "$path" in
    crates/cli/Cargo.toml | \
    crates/cli/src/lib.rs | \
    crates/cli/src/main.rs | \
    crates/cli/src/error.rs | \
    crates/cli/src/test_helpers.rs | \
    crates/cli/src/commands/mod.rs | \
    crates/cli/src/utils/mod.rs | \
    crates/cli/src/saas/mod.rs | \
    crates/cli/src/ops/mod.rs | \
    crates/cli/tests/integration_test_helpers.rs | \
    crates/cli/tests/ci_workflow_gates.rs)
      _pre_commit_mark_cli_full
      return 0
      ;;
    crates/cli/tests/integration_*.rs)
      local stem=${path##*/}
      stem=${stem%.rs}
      _pre_commit_add_cli_integration "$stem"
      return 0
      ;;
    crates/cli/src/commands/workflow.rs)
      _pre_commit_add_cli_integration integration_workflows
      _pre_commit_add_cli_unit_filter "commands::workflow::"
      ;;
    crates/cli/src/commands/watch.rs)
      _pre_commit_add_cli_integration integration_watch
      _pre_commit_add_cli_unit_filter "commands::watch::"
      ;;
    crates/cli/src/commands/debug.rs)
      _pre_commit_add_cli_integration integration_debug_ui
      _pre_commit_add_cli_unit_filter "commands::debug::"
      ;;
    crates/cli/src/commands/explain.rs)
      _pre_commit_add_cli_integration integration_explain
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_unit_filter "commands::explain::"
      ;;
    crates/cli/src/commands/validate.rs)
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_integration integration_error_cases
      _pre_commit_add_cli_unit_filter "commands::validate::"
      ;;
    crates/cli/src/commands/compile.rs)
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_unit_filter "commands::compile::"
      ;;
    crates/cli/src/commands/generate_sdk.rs)
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_unit_filter "ops::generate_sdk::"
      ;;
    crates/cli/src/commands/setup.rs | crates/cli/src/commands/init.rs)
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_unit_filter "commands::setup::"
      ;;
    crates/cli/src/commands/flag.rs)
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_integration integration_lifecycle
      _pre_commit_add_cli_unit_filter "commands::flag::"
      ;;
    crates/cli/src/commands/env.rs)
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_unit_filter "commands::env::"
      ;;
    crates/cli/src/commands/completion.rs)
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_unit_filter "commands::completion::"
      ;;
    crates/cli/src/commands/kill_switch.rs)
      _pre_commit_add_cli_integration integration_explain
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_unit_filter "commands::kill_switch::"
      ;;
    crates/cli/src/commands/ci.rs | crates/cli/src/commands/dev.rs)
      _pre_commit_add_cli_unit_filter "commands::"
      ;;
    crates/cli/src/utils/catalog_store.rs | crates/cli/src/utils/catalog.rs)
      _pre_commit_add_cli_integration integration_attributes
      _pre_commit_add_cli_integration integration_imports
      _pre_commit_add_cli_unit_filter "utils::catalog"
      ;;
    crates/cli/src/utils/config.rs)
      _pre_commit_add_cli_unit_filter "utils::config::"
      ;;
    crates/cli/src/utils/unified_config.rs)
      _pre_commit_add_cli_unit_filter "utils::unified_config::"
      ;;
    crates/cli/src/utils/environment.rs)
      _pre_commit_add_cli_unit_filter "utils::environment::"
      ;;
    crates/cli/src/utils/language.rs)
      _pre_commit_add_cli_unit_filter "utils::language::"
      ;;
    crates/cli/src/saas/*)
      _pre_commit_add_cli_integration integration_saas
      _pre_commit_add_cli_unit_filter "saas::"
      ;;
    crates/cli/src/ops/*)
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_unit_filter "ops::"
      ;;
    crates/cli/src/generator/*)
      _pre_commit_add_cli_integration integration_commands
      _pre_commit_add_cli_unit_filter "generator::"
      ;;
    crates/cli/tests/integration_assertions.rs)
      _pre_commit_add_cli_integration integration_assertions
      ;;
    crates/cli/tests/integration_legacy_prune.rs)
      _pre_commit_add_cli_integration integration_legacy_prune
      ;;
    crates/cli/*)
      _pre_commit_mark_cli_full
      ;;
  esac
}

_pre_commit_map_compiler_path() {
  local path=$1

  case "$path" in
    crates/compiler/Cargo.toml | \
    crates/compiler/src/lib.rs | \
    crates/compiler/src/error.rs | \
    crates/compiler/src/schemas.rs)
      _pre_commit_mark_compiler_full
      return 0
      ;;
    crates/compiler/src/catalog/base_attributes.rs)
      _pre_commit_add_compiler_filter "catalog::base_attributes::"
      ;;
    crates/compiler/src/catalog/validate.rs)
      _pre_commit_add_compiler_filter "catalog::validate::"
      _pre_commit_add_compiler_filter "catalog::tests::"
      ;;
    crates/compiler/src/catalog/model.rs)
      _pre_commit_add_compiler_filter "catalog::model::"
      _pre_commit_add_compiler_filter "catalog::tests::"
      ;;
    crates/compiler/src/catalog/parse.rs)
      _pre_commit_add_compiler_filter "catalog::parse::"
      _pre_commit_add_compiler_filter "catalog::tests::"
      ;;
    crates/compiler/src/catalog/namespace.rs)
      _pre_commit_add_compiler_filter "catalog::namespace::"
      _pre_commit_add_compiler_filter "catalog::tests::"
      ;;
    crates/compiler/src/catalog/compile.rs)
      _pre_commit_add_compiler_filter "catalog::compile::"
      _pre_commit_add_compiler_filter "catalog::compile_tests::"
      ;;
    crates/compiler/src/catalog/tests.rs)
      _pre_commit_add_compiler_filter "catalog::tests::"
      ;;
    crates/compiler/src/catalog/validation_mode_tests.rs)
      _pre_commit_add_compiler_filter "validation_mode_tests::"
      ;;
    crates/compiler/src/catalog/sdk.rs)
      _pre_commit_add_compiler_filter "catalog::sdk::"
      _pre_commit_add_compiler_filter "catalog::sdk_tests::"
      ;;
    crates/compiler/src/catalog/cdn.rs)
      _pre_commit_add_compiler_filter "catalog::cdn::"
      _pre_commit_add_compiler_filter "catalog::cdn_tests::"
      ;;
    crates/compiler/src/catalog/explain.rs)
      _pre_commit_add_compiler_filter "catalog::explain::"
      ;;
    crates/compiler/src/catalog/saas_environment.rs)
      _pre_commit_add_compiler_filter "catalog::saas_environment::"
      ;;
    crates/compiler/src/catalog/mod.rs)
      _pre_commit_add_compiler_filter "catalog::"
      ;;
    crates/compiler/src/catalog/*)
      _pre_commit_add_compiler_filter "catalog::"
      ;;
    crates/compiler/src/compiler/*)
      _pre_commit_add_compiler_filter "compiler::"
      ;;
    crates/compiler/src/parser/*)
      _pre_commit_add_compiler_filter "parser::"
      ;;
    crates/compiler/src/runtime/*)
      _pre_commit_add_compiler_filter "runtime::"
      ;;
    crates/compiler/src/validator/*)
      _pre_commit_add_compiler_filter "validator::"
      ;;
    crates/compiler/*)
      _pre_commit_mark_compiler_full
      ;;
    schemas/base-attributes.json)
      _pre_commit_add_compiler_filter "catalog::base_attributes::"
      _pre_commit_add_cli_integration integration_attributes
      ;;
    schemas/control-path.schema.v2.json | schemas/*)
      _pre_commit_add_compiler_filter "catalog::"
      _pre_commit_add_cli_integration integration_attributes
      _pre_commit_add_cli_integration integration_imports
      ;;
  esac
}

# Read staged paths from stdin; sets scope globals listed at top of file.
pre_commit_collect_test_scope() {
  PRE_COMMIT_CLI_FULL=false
  PRE_COMMIT_COMPILER_FULL=false
  PRE_COMMIT_CLI_INTEGRATION_TESTS=()
  PRE_COMMIT_CLI_UNIT_FILTERS=()
  PRE_COMMIT_COMPILER_FILTERS=()
  PRE_COMMIT_CLI_NEEDS_RUNTIME=false

  local path
  local saw_cli=false
  local saw_compiler=false

  while IFS= read -r path; do
    [ -z "$path" ] && continue
    case "$path" in
      crates/cli/*)
        saw_cli=true
        if [ "$PRE_COMMIT_CLI_FULL" != true ]; then
          _pre_commit_map_cli_path "$path"
        fi
        ;;
      crates/compiler/*)
        saw_compiler=true
        if [ "$PRE_COMMIT_COMPILER_FULL" != true ]; then
          _pre_commit_map_compiler_path "$path"
        fi
        ;;
      schemas/*)
        saw_compiler=true
        if [ "$PRE_COMMIT_COMPILER_FULL" != true ]; then
          _pre_commit_map_compiler_path "$path"
        fi
        ;;
    esac
  done

  if [ "$saw_cli" = true ] && [ "$PRE_COMMIT_CLI_FULL" != true ]; then
    if [ "${#PRE_COMMIT_CLI_INTEGRATION_TESTS[@]}" -eq 0 ] && [ "${#PRE_COMMIT_CLI_UNIT_FILTERS[@]}" -eq 0 ]; then
      _pre_commit_mark_cli_full
    fi
  fi

  if [ "$saw_compiler" = true ] && [ "$PRE_COMMIT_COMPILER_FULL" != true ]; then
    if [ "${#PRE_COMMIT_COMPILER_FILTERS[@]}" -eq 0 ]; then
      _pre_commit_mark_compiler_full
    fi
  fi
}

pre_commit_run_scoped_cargo_tests() {
  local ran=false

  if [ "$1" = true ]; then
    echo "  cargo test -p controlpath-compiler (full package)"
    cargo test -p controlpath-compiler
    ran=true
  elif [ "${#PRE_COMMIT_COMPILER_FILTERS[@]}" -gt 0 ]; then
    local filter
    for filter in ${PRE_COMMIT_COMPILER_FILTERS[@]+"${PRE_COMMIT_COMPILER_FILTERS[@]}"}; do
      echo "  cargo test -p controlpath-compiler ${filter}"
      cargo test -p controlpath-compiler "$filter"
      ran=true
    done
  fi

  if [ "$2" = true ]; then
    echo "  cargo test -p controlpath-cli (full package)"
    cargo test -p controlpath-cli
    ran=true
  else
    local test_name filter
    for test_name in ${PRE_COMMIT_CLI_INTEGRATION_TESTS[@]+"${PRE_COMMIT_CLI_INTEGRATION_TESTS[@]}"}; do
      echo "  cargo test -p controlpath-cli --test ${test_name}"
      cargo test -p controlpath-cli --test "$test_name"
      ran=true
    done
    for filter in ${PRE_COMMIT_CLI_UNIT_FILTERS[@]+"${PRE_COMMIT_CLI_UNIT_FILTERS[@]}"}; do
      echo "  cargo test -p controlpath-cli --lib ${filter}"
      cargo test -p controlpath-cli --lib "$filter"
      ran=true
    done
  fi

  if [ "$ran" = false ]; then
    die "Rust test scope collection produced no test targets"
  fi
}
