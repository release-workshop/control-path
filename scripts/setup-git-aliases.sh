#!/bin/bash

# Copyright 2025 Release Workshop Ltd
# Licensed under the Elastic License 2.0; you may not use this file except in compliance with the Elastic License 2.0.
# See the LICENSE file in the project root for details.

# Setup git aliases for Control Path development workflow

set -e

echo "Setting up git aliases for Control Path..."

# Get the repository remote URL (assuming origin)
REMOTE_URL=$(git remote get-url origin 2>/dev/null || echo "")
if [ -z "$REMOTE_URL" ]; then
  echo "Warning: Could not determine remote URL. The pushmain alias will still work, but PR URL generation may be limited."
  REPO_OWNER=""
  REPO_NAME=""
else
  # Extract owner/repo from various remote URL formats
  if [[ "$REMOTE_URL" =~ github\.com[:/]([^/]+)/([^/]+)(\.git)?$ ]]; then
    REPO_OWNER="${BASH_REMATCH[1]}"
    REPO_NAME="${BASH_REMATCH[2]%.git}"
  fi
fi

# pushmain alias: push current main through validation → auto-merge (appears as direct push to main)
git config alias.pushmain '!f() {
  CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

  if [ "$CURRENT_BRANCH" != "main" ]; then
    echo "Error: pushmain must be run from the main branch (current: $CURRENT_BRANCH)."
    echo "Please switch to main: git checkout main && git pull --ff-only"
    exit 1
  fi

  # Ensure local main is up to date
  echo "Syncing with origin/main..."
  git fetch origin main:main 2>/dev/null || true

  echo "Rebasing local main onto origin/main..."
  git rebase origin/main || {
    echo "Error: Rebase failed. Please resolve conflicts and try again."
    exit 1
  }

  # Create a unique remote validation branch name
  SHORT_SHA=$(git rev-parse --short HEAD)
  USER_PART=$(git config user.username || git config user.name || echo "dev")
  TS_PART=$(date +%Y%m%d-%H%M%S)
  REMOTE_BRANCH="validation/${USER_PART}-${TS_PART}-${SHORT_SHA}"

  echo "Pushing to validation branch: ${REMOTE_BRANCH}..."
  git push origin HEAD:"refs/heads/${REMOTE_BRANCH}"

  echo ""
  echo "✓ Pushed to ${REMOTE_BRANCH}"
  echo ""
  echo "CI is running validation checks. If all checks pass, your changes will"
  echo "automatically merge into main (appearing as if you pushed directly)."
  echo ""
  echo "You can continue working on main locally. Check GitHub Actions for status."
}; f'

echo "✓ Git alias 'pushmain' configured successfully!"
echo ""
echo "Usage (for maintainers/trusted users with trunk-based development):"
echo "  git checkout main"
echo "  # ... make changes and commit directly on main ..."
echo "  git pushmain"
echo ""
echo "This will:"
echo "  - Sync and rebase your local main onto origin/main"
echo "  - Push to a validation/* branch (invisible to you)"
echo "  - CI validates your changes (TIA, coverage, lint, typecheck)"
echo "  - On success, automatically merges into main (appears as direct push)"
echo ""
echo "Note: Contributors should use Pull Requests instead of pushmain."

