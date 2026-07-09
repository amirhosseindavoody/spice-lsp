#!/usr/bin/env bash
# Zig C compiler/linker wrapper targeting glibc 2.31 (Ubuntu 20.04+).
# Used by CI Linux release builds so Marketplace binaries do not require
# the newer glibc from ubuntu-latest (currently 2.39 / GLIBC_2.34).
set -euo pipefail

SCRIPT_PATH="${BASH_SOURCE[0]}"
while [[ -L "$SCRIPT_PATH" ]]; do
  SCRIPT_PATH="$(readlink "$SCRIPT_PATH")"
done
ROOT="$(cd "$(dirname "$SCRIPT_PATH")/.." && pwd)"

ZIG="${ZIG:-$ROOT/.pixi/envs/default/bin/zig}"
if [[ ! -x "$ZIG" ]]; then
  ZIG="$(command -v zig || true)"
fi
if [[ -z "${ZIG}" || ! -x "$ZIG" ]]; then
  echo "zig not found; run: pixi add zig && pixi install" >&2
  exit 127
fi

args=()
for arg in "$@"; do
  case "$arg" in
    --target=x86_64-unknown-linux-gnu|--target=x86_64-linux-gnu|--target=x86_64-unknown-linux-musl)
      ;;
    -target)
      skip_next=1
      ;;
    *)
      if [[ "${skip_next:-0}" == "1" ]]; then
        skip_next=0
      else
        args+=("$arg")
      fi
      ;;
  esac
done

exec "$ZIG" cc -target x86_64-linux-gnu.2.31 "${args[@]}"
