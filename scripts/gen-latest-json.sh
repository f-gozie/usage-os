#!/usr/bin/env bash
#
# gen-latest-json.sh — build the Tauri updater manifest (latest.json) from the signed updater
# artifact that `tauri build` produced (createUpdaterArtifacts + TAURI_SIGNING_PRIVATE_KEY).
#
# The updater fetches this file from the GitHub release, compares its `version` to the running
# build, and (if newer) downloads the `.app.tar.gz` and verifies the ed25519 `signature` before
# installing. We point both macOS arches at the one universal artifact under a STABLE asset name
# so the URL never changes across versions.
#
# Usage:  ./scripts/gen-latest-json.sh ["release notes"]
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

BUNDLE="src-tauri/target/universal-apple-darwin/release/bundle/macos"
TARBALL="$(ls -t "$BUNDLE"/*.app.tar.gz 2>/dev/null | head -1)"
[ -n "$TARBALL" ] && [ -f "$TARBALL" ] \
  || { echo "✗ no updater artifact (*.app.tar.gz) in $BUNDLE — is bundle.createUpdaterArtifacts set and TAURI_SIGNING_PRIVATE_KEY exported at build time?" >&2; exit 1; }
[ -f "$TARBALL.sig" ] \
  || { echo "✗ no signature $TARBALL.sig — TAURI_SIGNING_PRIVATE_KEY was not set when building" >&2; exit 1; }

VERSION="$(node -p "require('./src-tauri/tauri.conf.json').version")"
SIG="$(cat "$TARBALL.sig")"
NOTES="${1:-A new version of UsageOS is available. See the release page for what changed.}"
PUB_DATE="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
URL="https://github.com/f-gozie/usage-os/releases/latest/download/UsageOS.app.tar.gz"

OUT_DIR="src-tauri/target/universal-apple-darwin/release/bundle"
# Stage the tarball under the stable, version-independent name the manifest URL points at.
cp "$TARBALL" "$OUT_DIR/UsageOS.app.tar.gz"

cat > "$OUT_DIR/latest.json" <<JSON
{
  "version": "$VERSION",
  "notes": "$NOTES",
  "pub_date": "$PUB_DATE",
  "platforms": {
    "darwin-aarch64": { "signature": "$SIG", "url": "$URL" },
    "darwin-x86_64": { "signature": "$SIG", "url": "$URL" }
  }
}
JSON

echo "    ✓ $OUT_DIR/latest.json (v$VERSION)"
echo "    ✓ $OUT_DIR/UsageOS.app.tar.gz (upload this + latest.json to the release)"
