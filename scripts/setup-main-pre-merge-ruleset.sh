#!/bin/bash
# Create or update a GitHub Ruleset so PRs to main require Main CI jobs (including E2E smoke).
#
# Usage:
#   GITHUB_TOKEN=your_token REPO=owner/repo ./scripts/setup-main-pre-merge-ruleset.sh
#
# Job names must match workflow `name:` fields in .github/workflows/main-ci.yml.
# Confirm in GitHub → Actions after the first run on a PR.
#
# Release PRs (head: release-please--branches--main): Main CI skips those jobs by design.
# Use scripts/setup-e2e-ruleset.sh for release PRs; do not require Main CI contexts on them.

set -euo pipefail

RULESET_NAME="Main - Require pre-merge CI"

if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "❌ Error: GITHUB_TOKEN environment variable is required"
  exit 1
fi

if [ -z "${REPO:-}" ]; then
  if command -v git >/dev/null 2>&1; then
    REMOTE_URL=$(git remote get-url origin 2>/dev/null || echo "")
    if [[ "$REMOTE_URL" =~ github.com[:/]([^/]+/[^/.]+) ]]; then
      REPO="${BASH_REMATCH[1]}"
      echo "📦 Inferred repository: ${REPO}"
    else
      echo "❌ Error: REPO environment variable is required (owner/repo)"
      exit 1
    fi
  else
    echo "❌ Error: REPO environment variable is required (owner/repo)"
    exit 1
  fi
fi

API_BASE="https://api.github.com/repos/${REPO}/rulesets"
AUTH_HEADERS=(
  -H "Accept: application/vnd.github+json"
  -H "Authorization: Bearer ${GITHUB_TOKEN}"
  -H "X-GitHub-Api-Version: 2022-11-28"
)

RULESET_JSON=$(cat <<'EOF'
{
  "name": "Main - Require pre-merge CI",
  "target": "branch",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "include": ["~DEFAULT_BRANCH"]
    }
  },
  "rules": [
    {
      "type": "required_status_checks",
      "parameters": {
        "strict_required_status_checks_policy": true,
        "required_status_checks": [
          { "context": "Run Rust tests and clippy" },
          { "context": "Build CLI binary" },
          { "context": "Run TypeScript tests" },
          { "context": "Lint and typecheck" },
          { "context": "E2E smoke (pre-merge)" }
        ]
      }
    }
  ]
}
EOF
)

echo "🔧 Applying ruleset: ${RULESET_NAME}"
echo ""
echo "⚠️  Before enabling: release-please PRs skip Main CI jobs."
echo "   Keep scripts/setup-e2e-ruleset.sh for release-please--branches--main."
echo "   If release PRs stall, add a bypass for the release-please bot or do not"
echo "   require these contexts on that head branch in GitHub → Rules → Settings."
echo ""

EXISTING_ID=$(
  curl -s "${AUTH_HEADERS[@]}" "${API_BASE}" |
    jq -r --arg name "${RULESET_NAME}" '.[] | select(.name == $name) | .id' |
    head -n1
)

if [ -n "${EXISTING_ID}" ] && [ "${EXISTING_ID}" != "null" ]; then
  echo "↻ Updating existing ruleset (ID: ${EXISTING_ID})..."
  RESPONSE=$(curl -s -w "\n%{http_code}" \
    -X PUT \
    "${AUTH_HEADERS[@]}" \
    -d "${RULESET_JSON}" \
    "${API_BASE}/${EXISTING_ID}")
else
  echo "＋ Creating ruleset..."
  RESPONSE=$(curl -s -w "\n%{http_code}" \
    -X POST \
    "${AUTH_HEADERS[@]}" \
    -d "${RULESET_JSON}" \
    "${API_BASE}")
fi

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [ "$HTTP_CODE" = "201" ] || [ "$HTTP_CODE" = "200" ]; then
  RULESET_ID=$(echo "$BODY" | jq -r '.id')
  echo "✅ Ruleset applied (ID: ${RULESET_ID})"
  echo "   Required: Run Rust tests and clippy, Build CLI binary,"
  echo "             Run TypeScript tests, Lint and typecheck, E2E smoke (pre-merge)"
  echo "📋 https://github.com/${REPO}/settings/rules"
else
  echo "❌ Failed to apply ruleset (HTTP ${HTTP_CODE})"
  echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
  exit 1
fi
