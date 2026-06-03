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

  _pre_commit_dedupe_compiler_filters
  _pre_commit_dedupe_cli_unit_filters
}

pre_commit_cpu_count() {
  local n=4
  if command -v sysctl &>/dev/null; then
    n=$(sysctl -n hw.ncpu 2>/dev/null || echo 4)
  elif command -v getconf &>/dev/null; then
    n=$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)
  elif command -v nproc &>/dev/null; then
    n=$(nproc 2>/dev/null || echo 4)
  fi
  echo "$n"
}

# Drop narrower filters subsumed by a module-root filter (for example catalog::).
_pre_commit_dedupe_compiler_filters() {
  local has_catalog=false has_compiler=false has_parser=false has_runtime=false has_validator=false
  local filter
  local -a deduped=()

  for filter in ${PRE_COMMIT_COMPILER_FILTERS[@]+"${PRE_COMMIT_COMPILER_FILTERS[@]}"}; do
    case "$filter" in
      catalog::) has_catalog=true ;;
      compiler::) has_compiler=true ;;
      parser::) has_parser=true ;;
      runtime::) has_runtime=true ;;
      validator::) has_validator=true ;;
    esac
  done

  for filter in ${PRE_COMMIT_COMPILER_FILTERS[@]+"${PRE_COMMIT_COMPILER_FILTERS[@]}"}; do
    case "$filter" in
      catalog::*)
        if [ "$has_catalog" = true ] && [ "$filter" != "catalog::" ]; then
          continue
        fi
        ;;
      compiler::*)
        if [ "$has_compiler" = true ] && [ "$filter" != "compiler::" ]; then
          continue
        fi
        ;;
      parser::*)
        if [ "$has_parser" = true ] && [ "$filter" != "parser::" ]; then
          continue
        fi
        ;;
      runtime::*)
        if [ "$has_runtime" = true ] && [ "$filter" != "runtime::" ]; then
          continue
        fi
        ;;
      validator::*)
        if [ "$has_validator" = true ] && [ "$filter" != "validator::" ]; then
          continue
        fi
        ;;
    esac
    deduped+=("$filter")
  done

  PRE_COMMIT_COMPILER_FILTERS=()
  for filter in ${deduped[@]+"${deduped[@]}"}; do
    PRE_COMMIT_COMPILER_FILTERS+=("$filter")
  done
}

_pre_commit_dedupe_cli_unit_filters() {
  local has_commands=false
  local filter
  local -a deduped=()

  for filter in ${PRE_COMMIT_CLI_UNIT_FILTERS[@]+"${PRE_COMMIT_CLI_UNIT_FILTERS[@]}"}; do
    if [ "$filter" = "commands::" ]; then
      has_commands=true
      break
    fi
  done

  for filter in ${PRE_COMMIT_CLI_UNIT_FILTERS[@]+"${PRE_COMMIT_CLI_UNIT_FILTERS[@]}"}; do
    case "$filter" in
      commands::*)
        if [ "$has_commands" = true ] && [ "$filter" != "commands::" ]; then
          continue
        fi
        ;;
    esac
    deduped+=("$filter")
  done

  PRE_COMMIT_CLI_UNIT_FILTERS=()
  for filter in ${deduped[@]+"${deduped[@]}"}; do
    PRE_COMMIT_CLI_UNIT_FILTERS+=("$filter")
  done
}

pre_commit_default_parallel_jobs() {
  local num_tasks=$1
  local cpus
  cpus=$(pre_commit_cpu_count)
  if [ "$num_tasks" -lt "$cpus" ]; then
    echo "$num_tasks"
  else
    echo "$cpus"
  fi
}

pre_commit_test_threads_per_job() {
  local max_jobs=$1
  local cpus
  cpus=$(pre_commit_cpu_count)
  local threads=$((cpus / max_jobs))
  if [ "$threads" -lt 1 ]; then
    threads=1
  fi
  echo "$threads"
}

PRE_COMMIT_TEST_JOB_LABELS=()
PRE_COMMIT_TEST_JOB_SPECS=()

_pre_commit_queue_test_job() {
  PRE_COMMIT_TEST_JOB_LABELS+=("$1")
  PRE_COMMIT_TEST_JOB_SPECS+=("$2")
}

pre_commit_build_test_jobs() {
  PRE_COMMIT_TEST_JOB_LABELS=()
  PRE_COMMIT_TEST_JOB_SPECS=()

  if [ "$1" = true ]; then
    _pre_commit_queue_test_job \
      "cargo test -p controlpath-compiler (full package)" \
      "controlpath-compiler|full|"
  else
    local filter
    for filter in ${PRE_COMMIT_COMPILER_FILTERS[@]+"${PRE_COMMIT_COMPILER_FILTERS[@]}"}; do
      _pre_commit_queue_test_job \
        "cargo test -p controlpath-compiler ${filter}" \
        "controlpath-compiler|filter|${filter}"
    done
  fi

  if [ "$2" = true ]; then
    _pre_commit_queue_test_job \
      "cargo test -p controlpath-cli (full package)" \
      "controlpath-cli|full|"
  else
    local test_name filter
    for test_name in ${PRE_COMMIT_CLI_INTEGRATION_TESTS[@]+"${PRE_COMMIT_CLI_INTEGRATION_TESTS[@]}"}; do
      _pre_commit_queue_test_job \
        "cargo test -p controlpath-cli --test ${test_name}" \
        "controlpath-cli|integration|${test_name}"
    done
    for filter in ${PRE_COMMIT_CLI_UNIT_FILTERS[@]+"${PRE_COMMIT_CLI_UNIT_FILTERS[@]}"}; do
      _pre_commit_queue_test_job \
        "cargo test -p controlpath-cli --lib ${filter}" \
        "controlpath-cli|lib|${filter}"
    done
  fi
}

_pre_commit_run_one_test_job() {
  local spec=$1
  local test_threads=$2
  local pkg mode arg

  IFS='|' read -r pkg mode arg <<< "$spec"

  case "$mode" in
    full)
      cargo test -p "$pkg" -- --test-threads="$test_threads"
      ;;
    filter)
      cargo test -p "$pkg" "$arg" -- --test-threads="$test_threads"
      ;;
    integration)
      cargo test -p "$pkg" --test "$arg" -- --test-threads="$test_threads"
      ;;
    lib)
      cargo test -p "$pkg" --lib "$arg" -- --test-threads="$test_threads"
      ;;
    *)
      return 1
      ;;
  esac
}

_pre_commit_run_test_jobs_parallel() {
  local max_jobs=$1
  local test_threads=$2
  local total=${#PRE_COMMIT_TEST_JOB_SPECS[@]}
  local idx=0
  local failed=0
  local -a batch_pids=()
  local pid spec label

  while [ "$idx" -lt "$total" ]; do
    batch_pids=()
    local batch_count=0
    while [ "$batch_count" -lt "$max_jobs" ] && [ "$idx" -lt "$total" ]; do
      spec=${PRE_COMMIT_TEST_JOB_SPECS[$idx]}
      label=${PRE_COMMIT_TEST_JOB_LABELS[$idx]}
      echo "  [start] ${label} (--test-threads=${test_threads})"
      (
        _pre_commit_run_one_test_job "$spec" "$test_threads"
      ) &
      batch_pids+=($!)
      idx=$((idx + 1))
      batch_count=$((batch_count + 1))
    done

    for pid in ${batch_pids[@]+"${batch_pids[@]}"}; do
      if ! wait "$pid"; then
        failed=1
      fi
    done
  done

  return "$failed"
}

pre_commit_run_scoped_cargo_tests() {
  pre_commit_build_test_jobs "$1" "$2"

  local total=${#PRE_COMMIT_TEST_JOB_SPECS[@]}
  if [ "$total" -eq 0 ]; then
    die "Rust test scope collection produced no test targets"
  fi

  local max_jobs=1
  if [ "${PRE_COMMIT_SEQUENTIAL:-}" != "1" ]; then
    if [ -n "${PRE_COMMIT_TEST_JOBS:-}" ]; then
      max_jobs=$PRE_COMMIT_TEST_JOBS
    else
      max_jobs=$(pre_commit_default_parallel_jobs "$total")
    fi
  fi
  if [ "$max_jobs" -lt 1 ]; then
    max_jobs=1
  fi
  if [ "$max_jobs" -gt "$total" ]; then
    max_jobs=$total
  fi

  local test_threads
  test_threads=$(pre_commit_test_threads_per_job "$max_jobs")

  if [ "$max_jobs" -eq 1 ]; then
    echo "  Running ${total} scoped test job(s) sequentially (--test-threads=${test_threads})"
  else
    echo "  Running ${total} scoped test job(s) in parallel (jobs=${max_jobs}, --test-threads=${test_threads})"
  fi

  if ! _pre_commit_run_test_jobs_parallel "$max_jobs" "$test_threads"; then
    die "One or more scoped test jobs failed"
  fi
}
