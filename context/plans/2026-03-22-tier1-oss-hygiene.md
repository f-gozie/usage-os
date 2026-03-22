# Usage OS — Tier 1: OSS Hygiene & Foundation
**Task:** TASK-023 | **Workflow:** WF-026
**Branch:** `feat/tier1-oss-hygiene`
**Created:** 2026-03-22

## Goal
Make Usage OS credible for open-source release: tests, CI, data retention, proper migrations, and cross-platform documentation.

## Constraints
- Server is headless Linux — can build and test Rust/TS but cannot launch the Tauri desktop GUI
- Visual validation done via Vite dev server + browser screenshots (mock data for Tauri IPC)
- Final desktop smoke test requires Favour on macOS

---

## Phase 1: Rust Backend Tests
- [ ] Add `#[cfg(test)]` module in `db.rs` with in-memory SQLite
  - [ ] Test `init_database` creates schema correctly
  - [ ] Test `insert_activity_log` + `get_activity_logs` round-trip
  - [ ] Test coalescing: same process within 30s → updates `end_time`
  - [ ] Test coalescing: same process after 30s gap → new entry
  - [ ] Test coalescing: different process → new entry
  - [ ] Test coalescing: idle state change → new entry
  - [ ] Test `find_category` matches process name rules (case-insensitive)
  - [ ] Test `find_category` matches window title rules
  - [ ] Test `find_category` returns None when no rules match
  - [ ] Test `reprocess_logs` clears and reapplies categories
  - [ ] Test category CRUD (create, delete, cascade to rules)
  - [ ] Test rule CRUD (create, delete)
- [ ] Add `#[cfg(test)]` in `watcher.rs`
  - [ ] Test `is_user_idle` returns bool (unit, not integration)
  - [ ] Test `get_current_timestamp` returns reasonable value

## Phase 2: TypeScript Frontend Tests
- [ ] Set up Vitest + config
- [ ] `stats.test.ts`
  - [ ] Test `calculateDuration` excludes idle by default
  - [ ] Test `calculateDuration` includes idle with option
  - [ ] Test `calculateIdleDuration` sums only idle entries
  - [ ] Test `groupByProcess` aggregation + sorting
  - [ ] Test `groupByProcess` with idle toggle
  - [ ] Test `groupByCategory` with uncategorized bucket
  - [ ] Test `formatDuration` edge cases (0s, 59s, 1h 0m, etc.)
  - [ ] Test `getTodayRange` / `getYesterdayRange` / `getWeekRange` return valid Unix ranges
  - [ ] Test `getColorForProcess` returns valid HSL strings
- [ ] `time.test.ts`
  - [ ] Test `formatRelativeTime` output at various intervals

## Phase 3: Migration System
- [ ] Create `migrations` table: `(version INTEGER, name TEXT, applied_at INTEGER)`
- [ ] Refactor `init_database` to run migrations in order
- [ ] Migration 001: initial schema (activity_logs, categories, rules, indexes)
- [ ] Migration 002: add `category_id` to activity_logs (replaces inline ALTER TABLE check)
- [ ] Migration 003: add `data_retention_days` to a new `settings` table (default: 0 = keep all)

## Phase 4: Data Retention
- [ ] Add `settings` table with key-value pairs (via migration 003)
- [ ] Add `cleanup_old_data(conn, retention_days)` function in `db.rs`
  - [ ] Deletes activity_logs older than N days
  - [ ] Returns count of deleted rows
- [ ] Call cleanup on app startup (after migrations, before watcher starts)
- [ ] Add Tauri commands: `get_settings`, `update_setting`
- [ ] Add retention setting to SettingsView UI (dropdown: Keep All / 30 / 60 / 90 / 180 / 365 days)
- [ ] Test cleanup logic (delete correct rows, preserve recent)

## Phase 5: CI & Project Hygiene
- [ ] GitHub Actions workflow: `.github/workflows/ci.yml`
  - [ ] Matrix: ubuntu-latest, macos-latest, windows-latest
  - [ ] Steps: install Rust, install Node, `cargo test`, `npm ci`, `npm run lint`, Vitest
  - [ ] Tauri build step (validate it compiles on all platforms)
- [ ] Add `.github/CONTRIBUTING.md`
- [ ] Update README.md
  - [ ] Cross-platform setup (macOS, Windows, Linux permissions/deps)
  - [ ] Development section (cargo test, npm test, dev mode)
  - [ ] Architecture overview (what each file does)
  - [ ] Screenshots section (placeholder until we have real ones)
- [ ] Add `CHANGELOG.md` (retroactive entries for existing commits + v0.1.0)

## Phase 6: Quick Wins from Review
- [ ] Make chart top-N configurable (Settings, default 8 instead of 5)
- [ ] Add per-app "ignore window title" option in rules
- [ ] Watcher: surface persistent errors to frontend (not just stderr)

---

## Validation Criteria
1. `cargo test` passes all Rust tests
2. `npx vitest run` passes all TS tests
3. `cargo build --release` succeeds on Linux (CI covers macOS/Windows)
4. CI green on all 3 platforms
5. Favour confirms: app launches, tracks activity, settings work, data retention works (macOS smoke test)

## Out of Scope (Tier 2+)
- Timeline view
- Export (CSV/JSON)
- System tray icon
- XP/gamification
- Project tagging
