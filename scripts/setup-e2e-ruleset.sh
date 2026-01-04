#!/bin/bash
# Script to create a GitHub Ruleset that requires E2E tests to pass before merging release PRs
#
# Usage:
#   GITHUB_TOKEN=your_token REPO=owner/repo ./scripts/setup-e2e-ruleset.sh
#
# Or set environment variables:
#   export GITHUB_TOKEN=your_token
#   export REPO=owner/repo
#   ./scripts/setup-e2e-ruleset.sh

set -euo pipefail

# Check for required environment variables
if [ -z "${GITHUB_TOKEN:-}" ]; then
  echo "❌ Error: GITHUB_TOKEN environment variable is required"
  echo "   Get a token from: https://github.com/settings/tokens"
  echo "   Required scopes: repo, admin:repo"
  exit 1
fi

if [ -z "${REPO:-}" ]; then
  # Try to infer from git remote
  if command -v git >/dev/null 2>&1; then
    REMOTE_URL=$(git remote get-url origin 2>/dev/null || echo "")
    if [[ "$REMOTE_URL" =~ github.com[:/]([^/]+/[^/]+)\.git ]]; then
      REPO="${BASH_REMATCH[1]}"
      echo "📦 Inferred repository: ${REPO}"
    else
      echo "❌ Error: REPO environment variable is required"
      echo "   Format: owner/repo (e.g., controlpath/control-path)"
      exit 1
    fi
  else
    echo "❌ Error: REPO environment variable is required"
    echo "   Format: owner/repo (e.g., controlpath/control-path)"
    exit 1
  fi
fi

API_URL="https://api.github.com/repos/${REPO}/rulesets"

echo "🔧 Creating ruleset to require E2E tests for release PRs..."

# Create the ruleset JSON
RULESET_JSON=$(cat <<EOF
{
  "name": "Release PR - Require E2E Tests",
  "target": "branch",
  "enforcement": "active",
  "conditions": {
    "ref_name": {
      "include": ["release-please--branches--main"]
    }
  },
  "rules": [
    {
      "type": "required_status_checks",
      "parameters": {
        "strict_required_status_checks_policy": true,
        "required_status_checks": [
          {
            "context": "Run E2E Tests (Post-Merge Verification)"
          }
        ]
      }
    },
    {
      "type": "required_signatures"
    },
    {
      "type": "pull_request",
      "parameters": {
        "required_approving_review_count": 0,
        "dismiss_stale_reviews_on_push": false,
        "require_code_owner_review": false,
        "require_last_push_approval": false
      }
    }
  ]
}
EOF
)

# Create the ruleset
RESPONSE=$(curl -s -w "\n%{http_code}" \
  -X POST \
  -H "Accept: application/vnd.github+json" \
  -H "Authorization: Bearer ${GITHUB_TOKEN}" \
  -H "X-GitHub-Api-Version: 2022-11-28" \
  -d "${RULESET_JSON}" \
  "${API_URL}")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [ "$HTTP_CODE" = "201" ]; then
  RULESET_ID=$(echo "$BODY" | jq -r '.id')
  echo "✅ Ruleset created successfully!"
  echo "   Ruleset ID: ${RULESET_ID}"
  echo "   Target branch: release-please--branches--main"
  echo "   Required check: Run E2E Tests (Post-Merge Verification)"
  echo ""
  echo "📋 View ruleset: https://github.com/${REPO}/settings/rules"
elif [ "$HTTP_CODE" = "422" ]; then
  ERROR_MSG=$(echo "$BODY" | jq -r '.message // .errors[0].message // "Validation error"')
  echo "❌ Error: Validation failed"
  echo "   ${ERROR_MSG}"
  echo ""
  echo "   Full response:"
  echo "$BODY" | jq '.'
  exit 1
elif [ "$HTTP_CODE" = "404" ]; then
  echo "❌ Error: Repository not found or you don't have access"
  echo "   Repository: ${REPO}"
  echo "   Check that your token has 'repo' and 'admin:repo' scopes"
  exit 1
else
  echo "❌ Error: Failed to create ruleset (HTTP ${HTTP_CODE})"
  echo "   Response:"
  echo "$BODY" | jq '.' 2>/dev/null || echo "$BODY"
  exit 1
fi

echo ""
echo "🎉 Setup complete! Release PRs will now require E2E tests to pass."

