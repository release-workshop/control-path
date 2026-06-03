#!/bin/bash
# Push local main through validation/* CI and wait until land completes or fails.
#
# Success: "Merge into main" job succeeded. Does not wait for post-merge workflows on main.
# Phase A: still uses .github/workflows/auto-merge-validation.yml to land on main.
# Requires: git, GitHub CLI (gh) authenticated for this repository.

# Copyright 2025 Release Workshop Ltd
# Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
# See the LICENSE file in the project root for details.

set -euo pipefail

readonly VALIDATION_WORKFLOW_FILE="auto-merge-validation.yml"
readonly MERGE_JOB_NAME="Merge into main"
readonly WAIT_RUN_ATTEMPTS=45
readonly WAIT_RUN_SLEEP_SECS=2
readonly POLL_LAND_INTERVAL_SECS=5
# GitHub may mark a run completed before every job conclusion is visible in the API.
readonly MERGE_JOB_GRACE_ATTEMPTS=12

usage() {
  cat <<'EOF'
Usage: git pushmain [--no-wait]

  Push local main to a temporary validation/* branch, run pre-merge CI, and wait
  until the "Merge into main" job succeeds or the validation run fails.

Options:
  --no-wait   Push validation branch and exit (legacy fire-and-forget behavior)
  -h, --help  Show this help
EOF
}

die() {
  echo "❌ $*" >&2
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "Missing required command: $1"
}

repo_root() {
  git rev-parse --show-toplevel 2>/dev/null || die "Not inside a git repository."
}

current_branch() {
  git rev-parse --abbrev-ref HEAD 2>/dev/null || echo ""
}

sanitize_user_part() {
  local raw="${1:-dev}"
  raw="$(echo "$raw" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9-]/-/g' | sed 's/--*/-/g')"
  raw="${raw#-}"
  raw="${raw%-}"
  if [ -z "$raw" ]; then
    echo "dev"
  else
    echo "$raw"
  fi
}

validation_branch_name() {
  local short_sha user_part ts_part
  short_sha="$(git rev-parse --short HEAD)"
  user_part="$(sanitize_user_part "$(git config user.username 2>/dev/null || git config user.name 2>/dev/null || echo dev)")"
  ts_part="$(date +%Y%m%d-%H%M%S)"
  echo "validation/${user_part}-${ts_part}-${short_sha}"
}

wait_for_workflow_run_id() {
  local branch="$1"
  local attempt run_id

  for attempt in $(seq 1 "$WAIT_RUN_ATTEMPTS"); do
    run_id="$(
      gh run list \
        --branch "$branch" \
        --workflow "$VALIDATION_WORKFLOW_FILE" \
        --limit 1 \
        --json databaseId \
        --jq '.[0].databaseId // empty' 2>/dev/null || true
    )"
    if [ -n "$run_id" ]; then
      echo "$run_id"
      return 0
    fi
    sleep "$WAIT_RUN_SLEEP_SECS"
  done
  return 1
}

print_failed_run_summary() {
  local run_id="$1"
  local url conclusion

  url="$(gh run view "$run_id" --json url --jq .url 2>/dev/null || true)"
  conclusion="$(gh run view "$run_id" --json conclusion --jq .conclusion 2>/dev/null || true)"

  echo "" >&2
  echo "Validation did not land on main (run conclusion: ${conclusion:-unknown})." >&2
  if [ -n "$url" ]; then
    echo "Run: $url" >&2
  fi
  echo "" >&2
  echo "Failed jobs:" >&2
  gh run view "$run_id" --json jobs --jq '.jobs[] | select(.conclusion != "success" and .conclusion != "skipped") | "  - \(.name): \(.conclusion)"' 2>/dev/null >&2 || true
}

merge_job_conclusion() {
  local run_id="$1"
  gh run view "$run_id" --json jobs --jq \
    --arg name "$MERGE_JOB_NAME" \
    '.jobs[] | select(.name == $name) | .conclusion' 2>/dev/null | head -1 || true
}

landed_on_origin_main() {
  local land_sha="$1"
  git fetch origin main >/dev/null 2>&1 || true
  git merge-base --is-ancestor "$land_sha" origin/main 2>/dev/null
}

# Exit 0 when MERGE_JOB_NAME succeeds; exit 1 on failed/cancelled gate or merge jobs.
wait_for_merge_into_main() {
  local run_id="$1"
  local run_url="$2"
  local land_sha="$3"
  local completed_grace_attempts=0

  while true; do
    local merge_conclusion run_status
    merge_conclusion="$(merge_job_conclusion "$run_id")"

    case "$merge_conclusion" in
      success)
        return 0
        ;;
      failure | cancelled)
        print_failed_run_summary "$run_id"
        return 1
        ;;
      skipped)
        echo "" >&2
        echo "Workflow finished but \"${MERGE_JOB_NAME}\" was skipped." >&2
        echo "Run: ${run_url}" >&2
        echo "This often happens when path filters exclude your changes from auto-merge." >&2
        return 1
        ;;
    esac

    if gh run view "$run_id" --json jobs --jq \
      '.jobs[] | select(.conclusion == "failure" or .conclusion == "cancelled") | .name' 2>/dev/null |
      grep -q .; then
      print_failed_run_summary "$run_id"
      return 1
    fi

    run_status="$(gh run view "$run_id" --json status --jq .status 2>/dev/null || true)"
    if [ "$run_status" = "completed" ]; then
      if [ -z "$merge_conclusion" ]; then
        completed_grace_attempts=$((completed_grace_attempts + 1))
        if [ "$completed_grace_attempts" -le "$MERGE_JOB_GRACE_ATTEMPTS" ]; then
          sleep "$POLL_LAND_INTERVAL_SECS"
          continue
        fi
      fi

      if landed_on_origin_main "$land_sha"; then
        return 0
      fi

      echo "" >&2
      echo "Workflow finished but \"${MERGE_JOB_NAME}\" did not succeed (merge job: ${merge_conclusion:-not run})." >&2
      echo "Run: ${run_url}" >&2
      echo "This often happens when path filters exclude your changes from auto-merge." >&2
      return 1
    fi

    completed_grace_attempts=0
    sleep "$POLL_LAND_INTERVAL_SECS"
  done
}

main() {
  local no_wait=false
  local arg

  for arg in "$@"; do
    case "$arg" in
      -h | --help)
        usage
        exit 0
        ;;
      --no-wait)
        no_wait=true
        ;;
      *)
        die "Unknown argument: $arg (see --help)"
        ;;
    esac
  done

  require_cmd git
  cd "$(repo_root)"

  if [ "$(current_branch)" != "main" ]; then
    die "pushmain must be run from the main branch (current: $(current_branch)). Try: git checkout main"
  fi

  if [ "$no_wait" = false ]; then
    require_cmd gh
    gh auth status >/dev/null 2>&1 || die "GitHub CLI is not authenticated. Run: gh auth login"
  fi

  echo "Syncing with origin/main..."
  git fetch origin main 2>/dev/null || true

  echo "Rebasing local main onto origin/main..."
  if ! git rebase origin/main 2>/dev/null; then
    die "Rebase failed. Resolve conflicts, then run pushmain again."
  fi

  if git diff --quiet origin/main HEAD 2>/dev/null; then
    echo "No committed changes to push. Local main matches origin/main."
    exit 0
  fi

  local land_sha remote_branch
  land_sha="$(git rev-parse HEAD)"
  remote_branch="$(validation_branch_name)"

  echo "Pushing to validation branch: ${remote_branch}..."
  git push origin "HEAD:refs/heads/${remote_branch}"

  if [ "$no_wait" = true ]; then
    echo ""
    echo "✓ Pushed to ${remote_branch}"
    echo "CI is running (not waiting). Check: gh run list --branch ${remote_branch}"
    exit 0
  fi

  echo ""
  echo "Waiting for workflow ${VALIDATION_WORKFLOW_FILE} on ${remote_branch}..."
  local run_id
  if ! run_id="$(wait_for_workflow_run_id "$remote_branch")"; then
    die "Timed out waiting for a workflow run on ${remote_branch}. Check GitHub Actions."
  fi

  local run_url
  run_url="$(gh run view "$run_id" --json url --jq .url)"
  echo "Waiting for \"${MERGE_JOB_NAME}\" (run: ${run_url})..."

  if ! wait_for_merge_into_main "$run_id" "$run_url" "$land_sha"; then
    exit 1
  fi

  echo ""
  echo "✓ Merged into origin/main (${land_sha:0:7})"
  echo "  Main CI (smoke, TS tests, coverage) and post-merge E2E run on main — not waited on."
  echo "  Sync local main when ready: git pull --ff-only origin main"
}

main "$@"
