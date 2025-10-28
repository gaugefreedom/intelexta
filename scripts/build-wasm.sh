#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_CRATE="$ROOT_DIR/apps/web-verifier/wasm-verify"
OUTPUT_DIR="$ROOT_DIR/apps/web-verifier/public/pkg"

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "Error: wasm-pack is not installed. See https://rustwasm.github.io/wasm-pack/installer/" >&2
  exit 1
fi

mkdir -p "$OUTPUT_DIR"

wasm-pack build "$WASM_CRATE" \
  --target web \
  --out-dir "$OUTPUT_DIR" \
  --release
