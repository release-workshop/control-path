#!/bin/bash

# Copyright 2025 Release Workshop Ltd
# Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
# See the LICENSE file in the project root for details.

# Setup git aliases for Control Path development workflow

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PUSHMAIN_SCRIPT="${SCRIPT_DIR}/pushmain.sh"

echo "Setting up git aliases for Control Path..."

PRE_COMMIT_SCRIPT="${SCRIPT_DIR}/run-pre-commit-checks.sh"
for hook_script in "${PUSHMAIN_SCRIPT}" "${PRE_COMMIT_SCRIPT}"; do
  if [ ! -x "${hook_script}" ]; then
    chmod +x "${hook_script}"
  fi
done

HOOKS_DIR="$(cd "${SCRIPT_DIR}/../.githooks" && pwd)"
GIT_HOOKS="$(git rev-parse --git-path hooks)"
mkdir -p "${GIT_HOOKS}"
for hook in pre-commit commit-msg pre-push; do
  if [ -f "${HOOKS_DIR}/${hook}" ]; then
    cp "${HOOKS_DIR}/${hook}" "${GIT_HOOKS}/${hook}"
    chmod +x "${GIT_HOOKS}/${hook}"
  fi
done

# Resolve repo root at run time so the alias survives clone moves after re-setup.
git config alias.pushmain '!bash "$(git rev-parse --show-toplevel)/scripts/pushmain.sh"'

echo "✓ Git alias 'pushmain' configured successfully!"
echo ""
echo "✓ Git hooks installed (pre-commit: affected checks; commit-msg; pre-push)"
echo ""
echo "Usage (for maintainers/trusted users with trunk-based development):"
echo "  git checkout main"
echo "  # ... make changes and commit directly on main ..."
echo "  git pushmain"
echo ""
echo "This will:"
echo "  - Sync and rebase your local main onto origin/main"
echo "  - Push to a temporary validation/* branch"
echo "  - Wait until Merge into main succeeds (requires GitHub CLI: gh auth login)"
echo "  - On success, sync when ready: git pull --ff-only origin main"
echo ""
echo "Options:"
echo "  git pushmain --no-wait   # push validation branch without waiting for CI"
echo ""
echo "Note:"
echo "  - Direct pushes to main are blocked by the pre-push hook (use 'git pushmain')"
echo "  - Contributors should use pull requests instead of pushmain"
