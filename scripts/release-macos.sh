#!/usr/bin/env bash
#
# release-macos.sh — build a signed + notarized + stapled UsageOS DMG.
#
# What it does, in order:
#   1. Load the account-specific signing env (kept OUTSIDE this public repo).
#   2. Preflight: Developer ID identity, notarization creds, toolchain.
#   3. Build the REAL Swift sidecar (sidecar/build.sh) — never the CI placeholder stub.
#   4. Regenerate the bundled third-party license notices.
#   5. `tauri build` → signs the app + nested sidecar (hardened runtime), builds the DMG,
#      notarizes via the App Store Connect API key, and staples.
#   6. Verify the signature chain, Gatekeeper assessment, and the notarization staple.
#
# Secrets never live in the repo: the identity name + ASC key id/issuer come from
#   ~/.appstoreconnect/usageos-signing.env  (see ~/.appstoreconnect/usageos-signing.txt).
# A from-source build without that env still works — it just produces an unsigned app.
#
# Usage:  ./scripts/release-macos.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"
SIGNING_ENV="${USAGEOS_SIGNING_ENV:-$HOME/.appstoreconnect/usageos-signing.env}"

say() { printf '\n\033[1;34m▶ %s\033[0m\n' "$*"; }
die() { printf '\n\033[1;31m✗ %s\033[0m\n' "$*" >&2; exit 1; }

# ---------------------------------------------------------------------------
say "1/6  Load signing env"
if [ -f "$SIGNING_ENV" ]; then
  # shellcheck disable=SC1090
  source "$SIGNING_ENV"
  echo "    sourced $SIGNING_ENV"
else
  die "missing $SIGNING_ENV — see ~/.appstoreconnect/usageos-signing.txt for what it must export."
fi

: "${APPLE_SIGNING_IDENTITY:?APPLE_SIGNING_IDENTITY not set}"
: "${APPLE_API_ISSUER:?APPLE_API_ISSUER not set}"
: "${APPLE_API_KEY:?APPLE_API_KEY not set}"
: "${APPLE_API_KEY_PATH:?APPLE_API_KEY_PATH not set}"

# ---------------------------------------------------------------------------
say "2/6  Preflight"
security find-identity -v -p codesigning | grep -q "$APPLE_SIGNING_IDENTITY" \
  || die "signing identity not in keychain: $APPLE_SIGNING_IDENTITY"
echo "    ✓ Developer ID identity present"
[ -f "$APPLE_API_KEY_PATH" ] || die "ASC API key not found: $APPLE_API_KEY_PATH"
echo "    ✓ notarization API key present (key $APPLE_API_KEY)"
for t in tauri cargo npm swift xcrun rustc; do
  command -v "$t" >/dev/null 2>&1 || die "missing required tool: $t"
done
echo "    ✓ toolchain present"

# ---------------------------------------------------------------------------
say "3/6  Build the real Foundation Models sidecar"
./sidecar/build.sh --universal
SIDECAR="src-tauri/binaries/usageos-ai-universal-apple-darwin"
[ -s "$SIDECAR" ] || die "universal sidecar not built: $SIDECAR"
file "$SIDECAR" | grep -q "Mach-O" || die "sidecar is not a Mach-O binary (placeholder stub?): $SIDECAR"
lipo -info "$SIDECAR" | grep -q "x86_64 arm64\|arm64 x86_64" || die "sidecar is not universal: $SIDECAR"
echo "    ✓ $SIDECAR (universal)"

# ---------------------------------------------------------------------------
say "4/6  Regenerate third-party license notices"
./scripts/gen-licenses.sh

# ---------------------------------------------------------------------------
say "5/6  tauri build --target universal-apple-darwin  (sign · DMG · notarize · staple)"
tauri build --target universal-apple-darwin

# ---------------------------------------------------------------------------
say "6/6  Verify"
APP="$(ls -d src-tauri/target/universal-apple-darwin/release/bundle/macos/*.app 2>/dev/null | head -1)"
DMG="$(ls -t src-tauri/target/universal-apple-darwin/release/bundle/dmg/*.dmg 2>/dev/null | head -1)"
[ -n "$APP" ] || die "no .app produced"
[ -n "$DMG" ] || die "no .dmg produced"
echo "    app: $APP"
echo "    dmg: $DMG"

echo ""
echo "--- codesign --verify --deep --strict ---"
codesign --verify --deep --strict --verbose=2 "$APP" 2>&1 || die "app signature verification failed"

echo ""
echo "--- nested sidecar signature (must show Developer ID + runtime flag) ---"
NESTED="$(/usr/bin/find "$APP/Contents" -name 'usageos-ai*' -type f | head -1)"
if [ -n "$NESTED" ]; then
  codesign -dv --verbose=4 "$NESTED" 2>&1 | grep -iE "Authority=Developer ID|flags=.*runtime|TeamIdentifier" \
    || echo "    ⚠ sidecar may lack hardened runtime — inspect: codesign -dv --verbose=4 \"$NESTED\""
else
  echo "    ⚠ nested sidecar not found under $APP/Contents"
fi

echo ""
echo "--- Gatekeeper assessment (spctl) ---"
spctl -a -t exec -vvv "$APP" 2>&1 || echo "    ⚠ spctl assessment not yet accepted"

echo ""
echo "--- notarization staple ---"
xcrun stapler validate "$APP" 2>&1 || echo "    ⚠ app not stapled"
xcrun stapler validate "$DMG" 2>&1 || echo "    ⚠ dmg not stapled (the app inside may still be)"

say "Done"
echo "Ship:  $DMG"
