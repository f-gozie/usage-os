#!/usr/bin/env bash
#
# gen-licenses.sh — regenerate src-tauri/THIRD-PARTY-LICENSES.html, the bundled
# third-party notices (M5 distribution obligation). Covers:
#   • Rust crates compiled into the app  — via cargo-about (src-tauri/about.toml + about.hbs)
#   • npm packages bundled into the frontend — via scripts/gen-licenses-extra.mjs
#   • the OFL fonts (Anton / Jost)         — appended by the same helper
#
# The output is committed so `tauri build` (which validates bundled resources at compile
# time) always has it; the release script regenerates it to keep it fresh.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$REPO_ROOT/src-tauri/THIRD-PARTY-LICENSES.html"

command -v cargo-about >/dev/null 2>&1 || cargo install cargo-about --features cli

echo "[licenses] cargo-about → Rust crate notices"
( cd "$REPO_ROOT/src-tauri" && cargo about generate about.hbs -o "$OUT" )

echo "[licenses] npm + fonts → appended"
node "$REPO_ROOT/scripts/gen-licenses-extra.mjs" "$OUT"

echo "[licenses] wrote $OUT ($(wc -c < "$OUT" | tr -d ' ') bytes, $(grep -c '<h2>' "$OUT") sections)"
