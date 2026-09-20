#!/usr/bin/env bash
# Run `tauri dev` against the live database, then put the machine back.
#
# Why this exists: dev shares the installed app's bundle id, so it shares its database — and
# applies any migration newer than the installed release. A release build then REFUSES to start
# ("database was migrated by a newer version; do not downgrade"), and because the login item
# launches it `--hidden`, that failure is completely silent: no window, no tray, no recording.
# Forgetting one SQL revert after a dev session therefore costs days of tracking. This script
# makes the revert impossible to forget.
#
# It never touches /Applications and never signs anything.
set -euo pipefail

APP_SUPPORT="$HOME/Library/Application Support/com.usageos.app"
DB="$APP_SUPPORT/usage.db"
INSTALLED="/Applications/UsageOS.app"
REPO="$(cd "$(dirname "$0")/.." && pwd)"

# The newest migration the INSTALLED release ships. Anything above this gets reverted on exit.
# Bump this when a release containing newer migrations is installed.
INSTALLED_SCHEMA=8

restore() {
  echo
  echo "── restoring ─────────────────────────────────────────────"
  # Stop only our own dev binary, matched by exact process name — never by argv.
  pkill -x usage-os 2>/dev/null || true
  # `tauri dev` can leave its Vite server behind on exit, which blocks the next run with
  # "Port 1420 is already in use". Stop it only if it is this repo's Vite.
  local vite_pid
  vite_pid="$(lsof -nP -tiTCP:1420 -sTCP:LISTEN 2>/dev/null || true)"
  if [[ -n "$vite_pid" ]] && ps -p "$vite_pid" -o command= | grep -q "$REPO/node_modules"; then
    kill "$vite_pid" 2>/dev/null || true
  fi
  sleep 1

  if [[ -f "$DB" ]]; then
    local now
    now="$(sqlite3 "$DB" 'SELECT max(version) FROM schema_migrations;')"
    sqlite3 "$DB" 'PRAGMA wal_checkpoint(TRUNCATE);' >/dev/null 2>&1 || true
    if (( now > INSTALLED_SCHEMA )); then
      # 0009 is purely additive (one table), so reverting it loses only dev-session coverage rows.
      sqlite3 "$DB" "DROP TABLE IF EXISTS watch_coverage;
                     DELETE FROM schema_migrations WHERE version > $INSTALLED_SCHEMA;"
      echo "schema $now → $(sqlite3 "$DB" 'SELECT max(version) FROM schema_migrations;')"
    fi
    echo "integrity: $(sqlite3 "$DB" 'PRAGMA integrity_check;')"
  fi

  if [[ -d "$INSTALLED" ]]; then
    open -a "$INSTALLED" --args --hidden
    echo "installed UsageOS relaunched (menu bar) — tracking resumes"
  fi
}
trap restore EXIT

# Quit the installed app first: two processes must not write the same database.
pkill -x usage-os 2>/dev/null || true
sleep 1

cp "$DB" "$DB.pre-dev.bak"
echo "backup: $DB.pre-dev.bak"
echo "starting dev — quit the app or press Ctrl-C here when you're done; everything is restored."
echo

cd "$REPO"
npm run tauri dev
