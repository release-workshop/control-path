#!/bin/bash
# Setup script to install git hooks
# This replaces husky with standard git hooks

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
GITHOOKS_DIR="$REPO_ROOT/.githooks"
GIT_HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "🔧 Setting up git hooks..."

# Check if .git directory exists
if [ ! -d "$REPO_ROOT/.git" ]; then
  echo "❌ Error: Not a git repository. Run this from the repository root."
  exit 1
fi

# Ensure .git/hooks directory exists
mkdir -p "$GIT_HOOKS_DIR"

# Install hooks
for hook in pre-commit commit-msg pre-push; do
  if [ -f "$GITHOOKS_DIR/$hook" ]; then
    # Copy hook to .git/hooks
    cp "$GITHOOKS_DIR/$hook" "$GIT_HOOKS_DIR/$hook"
    chmod +x "$GIT_HOOKS_DIR/$hook"
    echo "  ✓ Installed $hook hook"
  fi
done

echo "✅ Git hooks installed successfully!"
echo ""
echo "Installed hooks:"
echo "  - pre-commit: Runs cargo check, clippy, fmt, and TypeScript build"
echo "  - commit-msg: Validates Conventional Commits format"
echo "  - pre-push: Blocks direct pushes to main branch"

