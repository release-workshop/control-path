#!/bin/bash
# Push local main through validation/* CI, wait for auto-merge, sync origin/main.
#
# Phase A: still uses .github/workflows/auto-merge-validation.yml to land on main.
# Requires: git, GitHub CLI (gh) authenticated for this repository.

# Copyright 2025 Release Workshop Ltd
# Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
# See the LICENSE file in the project root for details.

set -euo pipefail

readonly VALIDATION_WORKFLOW_FILE="auto-merge-validation.yml"
readonly WAIT_RUN_ATTEMPTS=45
readonly WAIT_RUN_SLEEP_SECS=2

usage() {
  cat <<'EOF'
Usage: git pushmain [--no-wait]

  Push local main to a temporary validation/* branch, run pre-merge CI, and on
  success wait for auto-merge into origin/main then fast-forward local main.

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
  echo "Validation workflow did not succeed (conclusion: ${conclusion:-unknown})." >&2
  if [ -n "$url" ]; then
    echo "Run: $url" >&2
  fi
  echo "" >&2
  echo "Failed jobs:" >&2
  gh run view "$run_id" --json jobs --jq '.jobs[] | select(.conclusion != "success" and .conclusion != "skipped") | "  - \(.name): \(.conclusion)"' 2>/dev/null >&2 || true
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
  echo "Watching run: ${run_url}"

  if ! gh run watch "$run_id" --exit-status; then
    print_failed_run_summary "$run_id"
    exit 1
  fi

  echo ""
  echo "Workflow succeeded. Checking that ${land_sha:0:7} landed on origin/main..."
  git fetch origin main

  if ! git merge-base --is-ancestor "$land_sha" origin/main; then
    die "CI finished but your commits are not on origin/main yet.
This often happens for docs-only or path-filtered changes where auto-merge does not run.
Branch: ${remote_branch}
Run: ${run_url}"
  fi

  echo "Fast-forwarding local main from origin/main..."
  git checkout main >/dev/null 2>&1
  git pull --ff-only origin main

  echo ""
  echo "✓ Landed on origin/main at $(git rev-parse --short origin/main)"
  echo "  Your commits: $(git rev-parse --short "$land_sha")"
}

main "$@"
