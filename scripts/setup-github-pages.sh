#!/usr/bin/env bash
# One-time GitHub Pages setup for spice-lsp.
#
# Requires: gh CLI authenticated as a repository admin.
# The deploy workflow pushes site output to the gh-pages branch but cannot
# enable Pages or set the repository Website field (GITHUB_TOKEN lacks access).
#
# Usage:
#   ./scripts/setup-github-pages.sh
#   ./scripts/setup-github-pages.sh owner/repo   # override repository

set -euo pipefail

REPO="${1:-amirhosseindavoody/spice-lsp}"
OWNER="${REPO%%/*}"
NAME="${REPO##*/}"
PAGES_URL="https://${OWNER}.github.io/${NAME}/"

echo "Repository: ${REPO}"
echo "Pages URL:  ${PAGES_URL}"
echo

echo "Ensuring gh-pages branch has been deployed at least once..."
if ! gh api "/repos/${REPO}/branches/gh-pages" >/dev/null 2>&1; then
  echo "error: gh-pages branch not found. Merge docs and wait for the Deploy docs workflow." >&2
  exit 1
fi

echo "Configuring GitHub Pages (source: gh-pages branch, / root)..."
if gh api "/repos/${REPO}/pages" >/dev/null 2>&1; then
  gh api -X PUT "/repos/${REPO}/pages" \
    -f build_type=legacy \
    -f "source[branch]=gh-pages" \
    -f "source[path]=/"
else
  gh api -X POST "/repos/${REPO}/pages" \
    -f build_type=legacy \
    -f "source[branch]=gh-pages" \
    -f "source[path]=/"
fi

echo "Setting repository Website field..."
gh repo edit "${REPO}" --homepage "${PAGES_URL}"

echo
echo "Done. Pages may take a minute to build."
echo "Site: ${PAGES_URL}"
echo "Settings: https://github.com/${REPO}/settings/pages"
