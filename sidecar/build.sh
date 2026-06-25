#!/usr/bin/env bash
#
# Build the usageos-ai Swift sidecar (release) and place it in the Tauri externalBin slot
# as `src-tauri/binaries/usageos-ai-$TARGET_TRIPLE` — the exact name `tauri build` /
# `app.shell().sidecar("binaries/usageos-ai")` resolve at bundle/spawn time.
#
# macOS-only (FoundationModels needs the macOS 26 SDK). Not part of `cargo`/CI: cross-
# platform CI stays green via the Rust FakeNarrator (C19); the real path is built here and
# in the separate non-blocking macOS Swift lane (C20). Run from anywhere:
#   ./sidecar/build.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$SCRIPT_DIR/usageos-ai"
OUT_DIR="$SCRIPT_DIR/../src-tauri/binaries"

# The triple Tauri expects in the externalBin filename. `--print host-tuple` is Rust 1.84+;
# fall back to parsing `rustc -vV` for older toolchains.
TRIPLE="$(rustc --print host-tuple 2>/dev/null || rustc -vV | sed -n 's/^host: //p')"
if [ -z "${TRIPLE:-}" ]; then
  echo "error: could not determine the host target triple (is rustc installed?)" >&2
  exit 1
fi

echo "[usageos-ai] swift build -c release"
( cd "$PKG_DIR" && swift build -c release )

SRC="$PKG_DIR/.build/release/usageos-ai"
DEST="$OUT_DIR/usageos-ai-$TRIPLE"
mkdir -p "$OUT_DIR"
cp "$SRC" "$DEST"
echo "[usageos-ai] -> $DEST"
