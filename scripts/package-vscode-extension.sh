#!/usr/bin/env bash
# Assemble a VSIX with prebuilt spice-lsp binaries under editors/vscode/bin/.
#
# Usage:
#   ./scripts/package-vscode-extension.sh
#   ./scripts/package-vscode-extension.sh --from-artifacts ./artifacts
#
# With --from-artifacts, expects files named spice-lsp-<platform-id>[.exe] in that
# directory (as produced by the release-vscode GitHub Actions workflow).

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXT_DIR="$ROOT/editors/vscode"
BIN_DIR="$EXT_DIR/bin"
ARTIFACTS_DIR=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --from-artifacts)
      ARTIFACTS_DIR="${2:?missing artifacts directory}"
      shift 2
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
      ;;
  esac
done

rm -rf "$BIN_DIR"
mkdir -p "$BIN_DIR"

if [[ -n "$ARTIFACTS_DIR" ]]; then
  shopt -s nullglob
  for artifact in "$ARTIFACTS_DIR"/spice-lsp-*; do
    base="$(basename "$artifact")"
    platform="${base#spice-lsp-}"
    if [[ "$platform" == *.exe ]]; then
      platform="${platform%.exe}"
      dest="$BIN_DIR/$platform/spice-lsp.exe"
    else
      dest="$BIN_DIR/$platform/spice-lsp"
    fi
    mkdir -p "$(dirname "$dest")"
    cp "$artifact" "$dest"
    chmod +x "$dest"
    echo "Installed $dest"
  done
  shopt -u nullglob
else
  PLATFORM_ID="$(node -p "process.platform + '-' + process.arch")"
  RELEASE_BIN="$ROOT/target/release/spice-lsp"
  if [[ "$(uname -s)" == "MINGW"* || "$(uname -s)" == "MSYS"* || "$(uname -s)" == "CYGWIN"* ]]; then
    RELEASE_BIN="$ROOT/target/release/spice-lsp.exe"
  fi
  if [[ ! -f "$RELEASE_BIN" ]]; then
    echo "Release binary not found at $RELEASE_BIN" >&2
    echo "Run: pixi run build" >&2
    exit 1
  fi
  DEST="$BIN_DIR/$PLATFORM_ID/$(basename "$RELEASE_BIN")"
  mkdir -p "$(dirname "$DEST")"
  cp "$RELEASE_BIN" "$DEST"
  chmod +x "$DEST"
  echo "Bundled $RELEASE_BIN -> $DEST"
fi

if [[ -z "$(find "$BIN_DIR" -type f 2>/dev/null | head -1)" ]]; then
  echo "No binaries were bundled under $BIN_DIR" >&2
  exit 1
fi

cd "$EXT_DIR"
npm install
npm run compile
# Extension JS is esbuild-bundled (vscode-languageclient inlined). Pack without
# shipping node_modules.
npx vsce package --no-dependencies

VSIX="$(ls -t "$EXT_DIR"/*.vsix | head -1)"
# Avoid `grep -q` under `pipefail`: early exit SIGPIPEs unzip and falsely fails the check.
if ! unzip -l "$VSIX" | grep -F 'extension/out/extension.js' >/dev/null; then
  echo "Packaged VSIX is missing extension/out/extension.js: $VSIX" >&2
  exit 1
fi
if unzip -l "$VSIX" | grep -F 'extension/node_modules/' >/dev/null; then
  echo "Packaged VSIX unexpectedly contains node_modules (should be bundled): $VSIX" >&2
  exit 1
fi
if ! unzip -p "$VSIX" extension/out/extension.js | grep -F 'LanguageClient' >/dev/null; then
  echo "Bundled extension.js does not appear to include LanguageClient: $VSIX" >&2
  exit 1
fi
if ! unzip -l "$VSIX" | grep -F 'extension/out/terminateProcess.sh' >/dev/null; then
  echo "Packaged VSIX is missing terminateProcess.sh helper: $VSIX" >&2
  exit 1
fi

echo "VSIX written to $VSIX"
