#!/usr/bin/env bash
# scripts/release.sh — Tag and trigger a Jura Trace release build.
#
# Usage:
#   ./scripts/release.sh v0.3.0
#
# What this does:
#   1. Validates the version argument follows vMAJOR.MINOR.PATCH[-suffix]
#   2. Checks the working tree is clean (no uncommitted changes)
#   3. Creates an annotated git tag
#   4. Pushes the tag to origin, triggering .github/workflows/release.yml
#
# Prerequisites:
#   - git is installed and configured
#   - You have push access to origin (github.com/juralabs/jura-archive)
#   - The GitHub "production" environment is configured with required reviewers
#     (Settings > Environments > production)

set -euo pipefail

VERSION="${1:-}"

# ── Validate argument ─────────────────────────────────────────────────────────
if [[ -z "$VERSION" ]]; then
  echo "Error: version argument is required."
  echo ""
  echo "Usage: ./scripts/release.sh v0.3.0"
  echo "       ./scripts/release.sh v0.3.0-beta.1"
  exit 1
fi

if [[ ! "$VERSION" =~ ^v[0-9]+\.[0-9]+\.[0-9]+(-[a-zA-Z0-9.]+)?$ ]]; then
  echo "Error: version '${VERSION}' does not match vMAJOR.MINOR.PATCH[-suffix]"
  echo "Examples: v0.3.0  v1.0.0-beta.1  v2.0.0-rc.1"
  exit 1
fi

# ── Verify the working tree is clean ─────────────────────────────────────────
if [[ -n "$(git status --porcelain)" ]]; then
  echo "Error: working tree has uncommitted changes."
  echo "Commit or stash your changes before releasing."
  git status --short
  exit 1
fi

# ── Verify we're on main ──────────────────────────────────────────────────────
CURRENT_BRANCH="$(git rev-parse --abbrev-ref HEAD)"
if [[ "$CURRENT_BRANCH" != "main" ]]; then
  echo "Warning: you are on branch '${CURRENT_BRANCH}', not 'main'."
  read -r -p "Continue anyway? [y/N] " CONFIRM
  if [[ "$CONFIRM" != "y" && "$CONFIRM" != "Y" ]]; then
    echo "Aborted."
    exit 1
  fi
fi

# ── Check the tag does not already exist ─────────────────────────────────────
if git rev-parse "$VERSION" >/dev/null 2>&1; then
  echo "Error: tag '${VERSION}' already exists locally."
  echo "Delete it first: git tag -d ${VERSION}"
  exit 1
fi

# ── Create and push the tag ───────────────────────────────────────────────────
echo "Creating annotated tag ${VERSION} ..."
git tag -a "$VERSION" -m "Release ${VERSION}"

echo "Pushing tag to origin ..."
git push origin "$VERSION"

echo ""
echo "Done. The release workflow is now running at:"
echo "https://github.com/juralabs/jura-archive/actions"
echo ""
echo "Once all platform builds complete, a reviewer must approve the"
echo "'production' environment gate before the release is published."
