#!/usr/bin/env bash
#
# Build the usageos-ai Swift sidecar (release) into the Tauri externalBin slot(s) as
# `src-tauri/binaries/usageos-ai-$TARGET_TRIPLE` — the exact name `tauri build` /
# `app.shell().sidecar("binaries/usageos-ai")` resolve at bundle/spawn time.
#
#   ./sidecar/build.sh              # host arch only (dev / a host-target release)
#   ./sidecar/build.sh --universal  # arm64 + x86_64 slices, for `tauri build --target universal-apple-darwin`
#
# macOS-only (FoundationModels needs the macOS 26 SDK; it links — and degrades at runtime —
# on x86_64 too, so a Universal build covers Intel). Not part of `cargo`/CI: cross-platform CI
# stays green via the Rust FakeNarrator (C19); the real path is built here and in the separate
# non-blocking macOS Swift lane (C20).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PKG_DIR="$SCRIPT_DIR/usageos-ai"
OUT_DIR="$SCRIPT_DIR/../src-tauri/binaries"
mkdir -p "$OUT_DIR"

if [ "${1:-}" = "--universal" ]; then
  echo "[usageos-ai] swift build -c release --arch arm64 --arch x86_64 (universal)"
  ( cd "$PKG_DIR" && swift build -c release --arch arm64 --arch x86_64 )
  FAT="$PKG_DIR/.build/apple/Products/Release/usageos-ai"
  # `tauri build --target universal-apple-darwin` builds each arch separately (each needs its
  # per-arch externalBin) THEN assembles the universal app (needs the universal one) — so emit
  # all three: the two thin slices + the fat binary.
  lipo "$FAT" -thin arm64  -output "$OUT_DIR/usageos-ai-aarch64-apple-darwin"
  lipo "$FAT" -thin x86_64 -output "$OUT_DIR/usageos-ai-x86_64-apple-darwin"
  cp   "$FAT"              "$OUT_DIR/usageos-ai-universal-apple-darwin"
  echo "[usageos-ai] -> $OUT_DIR/usageos-ai-{aarch64,x86_64,universal}-apple-darwin"
else
  # The triple Tauri expects in the externalBin filename. `--print host-tuple` is Rust 1.84+;
  # fall back to parsing `rustc -vV` for older toolchains.
  TRIPLE="$(rustc --print host-tuple 2>/dev/null || rustc -vV | sed -n 's/^host: //p')"
  if [ -z "${TRIPLE:-}" ]; then
    echo "error: could not determine the host target triple (is rustc installed?)" >&2
    exit 1
  fi
  echo "[usageos-ai] swift build -c release ($TRIPLE)"
  ( cd "$PKG_DIR" && swift build -c release )
  cp "$PKG_DIR/.build/release/usageos-ai" "$OUT_DIR/usageos-ai-$TRIPLE"
  echo "[usageos-ai] -> $OUT_DIR/usageos-ai-$TRIPLE"
fi
