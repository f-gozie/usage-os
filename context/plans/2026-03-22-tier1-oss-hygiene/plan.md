# UsageOS — Tier 1: OSS Hygiene & Foundation
**Task:** TASK-023 | **Workflow:** WF-026
**Branch:** `feat/tier1-oss-hygiene`
**Created:** 2026-03-22
**Status:** ✅ Complete

## Goal
Make UsageOS credible for open-source release: tests, CI, data retention, proper migrations, and cross-platform documentation.

## Constraints
- Server is headless Linux — can build and test Rust/TS but cannot launch the Tauri desktop GUI
- Visual validation done via Vite dev server + browser screenshots (mock data for Tauri IPC)
- Final desktop smoke test requires Favour on macOS

---

## Phase 1: Rust Backend Tests ✅
- [x] Add `#[cfg(test)]` module in `db.rs` with in-memory SQLite
  - [x] Test `init_database` creates schema correctly
  - [x] Test `insert_activity_log` + `get_activity_logs` round-trip
  - [x] Test coalescing: same process within 30s → updates `end_time`
  - [x] Test coalescing: same process after 30s gap → new entry
  - [x] Test coalescing: different process → new entry
  - [x] Test coalescing: idle state change → new entry
  - [x] Test `find_category` matches process name rules (case-insensitive)
  - [x] Test `find_category` matches window title rules
  - [x] Test `find_category` returns None when no rules match
  - [x] Test `reprocess_logs` clears and reapplies categories
  - [x] Test category CRUD (create, delete, cascade to rules)
  - [x] Test rule CRUD (create, delete)
- [x] Add `#[cfg(test)]` in `watcher.rs`
  - [x] Test `get_current_timestamp` returns reasonable value
  - [x] ~~Test `is_user_idle` returns bool~~ (skipped: segfaults on headless — needs X11)

## Phase 2: TypeScript Frontend Tests ✅
- [x] Set up Vitest + config
- [x] `stats.test.ts`
  - [x] Test `calculateDuration` excludes idle by default
  - [x] Test `calculateDuration` includes idle with option
  - [x] Test `calculateIdleDuration` sums only idle entries
  - [x] Test `groupByProcess` aggregation + sorting
  - [x] Test `groupByProcess` with idle toggle
  - [x] Test `groupByCategory` with uncategorized bucket
  - [x] Test `formatDuration` edge cases (0s, 59s, 1h 0m, etc.)
  - [x] Test `getTodayRange` / `getYesterdayRange` / `getWeekRange` return valid Unix ranges
  - [x] Test `getColorForProcess` returns valid HSL strings
- [x] `time.test.ts`
  - [x] Test `formatRelativeTime` output at various intervals

## Phase 3: Migration System ✅
- [x] Create `schema_migrations` table: `(version INTEGER, name TEXT, applied_at INTEGER)`
- [x] Refactor `init_database` to run migrations in order
- [x] Migration 001: initial schema (activity_logs, categories, rules, indexes)
- [x] Migration 002: add `category_id` to activity_logs (replaces inline ALTER TABLE check)
- [x] Migration 003: add settings table (key-value pairs)

## Phase 4: Data Retention ✅
- [x] Add `settings` table with key-value pairs (via migration 003)
- [x] Add `cleanup_old_data(conn, retention_days)` function in `db.rs`
  - [x] Deletes activity_logs older than N days
  - [x] Returns count of deleted rows
- [x] Call cleanup on app startup (after migrations, before watcher starts)
- [x] Add Tauri commands: `get_settings`, `update_setting`
- [x] Add retention setting to SettingsView UI (dropdown: Keep All / 30 / 60 / 90 / 180 / 365 days)
- [x] Test cleanup logic (delete correct rows, preserve recent)

## Phase 5: CI & Project Hygiene ✅
- [x] GitHub Actions workflow: `.github/workflows/ci.yml`
  - [x] Matrix: ubuntu-latest, macos-latest, windows-latest
  - [x] Steps: install Rust, install Node, `cargo test`, `npm ci`, Vitest, `cargo build`
- [x] Add `.github/CONTRIBUTING.md`
- [x] Update README.md
  - [x] Cross-platform setup (macOS, Windows, Linux permissions/deps)
  - [x] Development section (cargo test, npm test, dev mode)
  - [x] Architecture overview (what each file does)
  - [x] Screenshots section (placeholder until we have real ones)
- [x] Add `CHANGELOG.md` (retroactive entries for existing commits + v0.1.0)

## Phase 6: Quick Wins from Review ✅
- [x] Make chart top-N configurable (Settings, default 8 instead of 5)
- [x] Add per-app "ignore window title" option in rules (migration 004)
- [x] Watcher: surface persistent errors to frontend (error counter + health status command)

---

## Validation Results
- `cargo test`: 25 tests passing ✅
- `npx vitest run`: 28 tests passing ✅
- `cargo build`: compilation succeeds ✅
- CI: configured for Linux/macOS/Windows (will validate on push)
- Final desktop smoke test: awaiting Favour on macOS

## Out of Scope (Tier 2+)
- Timeline view
- Export (CSV/JSON)
- System tray icon
- XP/gamification
- Project tagging
