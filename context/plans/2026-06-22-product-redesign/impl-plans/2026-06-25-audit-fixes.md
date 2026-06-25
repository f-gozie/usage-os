# Impl-plan — Resolve the full-codebase audit (Phase 6 cleanup)

**Date:** 2026-06-25 · **Branch:** `phase6/audit-fixes` · **Review:** [reviews/2026-06-25-full-codebase-audit.md](../reviews/2026-06-25-full-codebase-audit.md)

Resolve everything the audit reported, in verified phases (gates green after each). Excludes items **in-flight in PR #17** (the AI seam, the `"fm"` literal, the untracked-binary hygiene, the old Swift spike) and **defers** two items that need a product/perf decision (see Phase D).

Gates after every phase: `cargo fmt --check`, `cargo clippy --all-targets -D warnings`, `cargo test`, `tsc --noEmit`, `vitest run`, bindings fresh.

## Status — checkpoint 2026-06-25 (all gates green: 113 Rust + 20 TS, clippy/fmt/tsc, bindings fresh)

**Landed & verified (Batch 1 — all the data-correctness bugs + safe hardening/hygiene):**
- ✅ **A1** slugless/user categories no longer collapse into Uncategorized (carry id-key + hex `color` through `CategorySlice`/`CategoryRun`/`TimelineRun`/`TimelineSegment`; frontend renders it) — IPC additive, +regression test.
- ✅ **A2 (consumer)** capture idle-inflation: `on_focus` gates on the open/last window's idle gate so background title-churn can't inflate or spawn phantom spans; `same_window` now includes `project_id` (terminal `cd` opens a new span). +2 tests.
- ✅ **A3** day-boundary clipping: `get_activity_logs` selects overlapping spans (half-open) and clips them to the window — a midnight-crosser is counted once per day. +test.
- ✅ **A4** `reprocess_logs` runs in one transaction (atomic) and matches with escaped LIKE = literal substring, identical to live `find_category` (no `%`/`_` wildcard divergence). +2 tests.
- ✅ **A8** enrich: `github.com` exact/subdomain match (no look-alike hosts); `127.0.0.1:port` treated as local; browser tab titles no longer mint fake projects (title signal skipped when a URL is present).
- ✅ **A9 (backend)** `leading_project` requires a clear lead (no tie narrated as "mostly on X"); icon `cache_key` is collision-free (FNV suffix).
- ✅ **A6a** `CANON_ORDER` includes `personal`.
- ✅ **A9fe (theme)** ThemeProvider initial load can't clobber a user theme toggle made mid-load.
- ✅ **B2** empty/whitespace patterns skipped in `find_category` + `match_exclusion` (no match-all blackout). +test.
- ✅ **B1 (opener)** dropped over-broad `opener:default` (only reveal-in-dir is used).
- ✅ **C2 (partial)** `load_lookup_maps` helper kills the 3× duplicated lookup-map block.
- ✅ **C7 (partial)** real `Cargo.toml` author/description + `package.json` name (was `tauri-app`).

**Remaining (Batch 2 — frontend refactors + simplification sweep; CI-verifiable, lower data-risk):**
- [ ] **A5** collapse `useDayData`/`useTimelineData`/`useWeekData` → one `useViewData<T>` with a stale-response guard (fixes the wrong-range race) + RTL test.
- [ ] **A6b** new-category-from-Uncategorized seeds the selected app; **A7** CategoryEditorModal moved-app toggle un-queues the delete + title-drawer copy; **A9fe** DayView Retry also re-checks capture health; `categories.ts` names vs DB; `runs.ts` inspector no-project.
- [ ] **B3** migration hardening (0006 first-run gate, `CHECK` constraints, drift on missing migration); **B5** a11y (Modal focus-trap/scroll, Select keyboard, LedgerRow).
- [ ] **C1** decision-archaeology comment sweep (~40, + 2 stale comments); **C3** dead `FocusEvent.bundle_id`/`.pid` + single-use indirection; **C4** hardcoded colors → tokens; **C5** shared `ErrorState`/`useCaptureHealth`/`DayStepper`; **C6** split `db.rs`.

**Deferred to PR #17 coordination:** CSP (`tauri.conf.json`) + `tokio` feature trim (#17's sidecar timeout needs `tokio::time`; #17 owns these files). **Deferred to on-device:** the native A2c/B4 items. **Owner decision:** Chromium-keep-title privacy posture; `Arc<Mutex>`→R57.

---

## Phase A — Correctness (real bugs; each with a test)

- [ ] **A1 `rollup.rs:164` — slugless/user categories collapse into "Uncategorized".** `category_of` maps `slug=None`→`OTHER_SLUG`, and `build_day_view`/`build_runs` aggregate keyed on slug → all custom categories merge into one gray arc and lose their color. Fix: give a slugless category a stable identity (its own `category_id`-derived key) and carry its `color` through to `CategorySlice`; only true `category_id=None` is "Uncategorized". Add `color: Option<String>` to `CategorySlice`/`CategoryMeta` (IPC additive). + test with a slugless category.
- [ ] **A2 Capture honesty cluster.** (a) `capture/mod.rs:282` route same-window re-fires through the idle gate (read `current_idle_secs` on event arrival; no-op `set_span_end` when end doesn't advance). (b) `capture/mod.rs:147` add `project_id` to `same_window()`. (c) `capture/macos/mod.rs:191` dedupe on full identity (title+url+cwd+private), not title only. + tests for each.
- [ ] **A3 Day-boundary clipping** (`db.rs:264` + `rollup.rs:156`): clip span durations to the queried `[start, end)` so a span crossing midnight is split across days, not counted whole on the start day. + test.
- [ ] **A4 `reprocess_logs`** (`db.rs:420`/`:429`): wrap in a transaction (atomic); unify its matching with live `find_category` (one substring primitive, not SQL `LIKE`). + test that reprocess == live categorization.
- [ ] **A5 `useViewData<T>`**: collapse `useDayData`/`useTimelineData`/`useWeekData` into one generic hook with an AbortController/request-id stale-guard (fixes the wrong-range race) + dedup. + RTL test for out-of-order resolution.
- [ ] **A6 Settings**: `SettingsView.tsx:33` add `personal` to `CANON_ORDER`; `:150` new-category-from-Uncategorized keeps the selected app; `categories.ts:12` reconcile hard-coded canonical names with DB names.
- [ ] **A7 `CategoryEditorModal`**: `:142` toggling a just-moved app off must un-queue its `movedIds` delete; `:54` make the "match by window title" drawer save bare tokens as title rules (or fix the copy to require `title:`).
- [ ] **A8 enrich**: `project.rs:73` abstain on browser titles for non-repo pages (no fake "YouTube"/"GitHub" projects); `:159` host equality/suffix-with-dot for `github.com`; `:176` treat `127.0.0.1:port` like `localhost:port`.
- [ ] **A9 smaller**: `rollup.rs:757` tie-break/margin in `leading_project`; `ThemeProvider.tsx:57` don't let initial load overwrite a user theme toggle; `DayView.tsx:76` Retry re-runs the health check; `runs.ts:14` inspector keeps mixed no-project time; `apps.rs:189` unique icon cache key.

## Phase B — Hardening / privacy

- [ ] **B1** `tauri.conf.json:25` set a strict CSP (`default-src 'self'`); `capabilities/default.json:9` scope `opener` to the reveal/open-file perms used (coordinate with PR #17 — these files overlap).
- [ ] **B2** Guard empty patterns at the repository layer: `create_rule`/`create_exclusion` reject empty/whitespace; `find_category`/`match_exclusion` skip empty patterns (defense-in-depth for the `contains("")` match-all).
- [ ] **B3** `migrations/0006:12` gate "fresh-install-only" on a real first-run marker, not `COUNT(activity_logs)=0`; `migrations.rs:97` surface a stored-but-missing migration (downgrade detection); `0001` add `CHECK` constraints to the enum-like columns.
- [ ] **B4** `capture/macos/mod.rs:214` move `osascript`/`pgrep` off the main run loop (worker + channel) so a wedged scripted app can't freeze the UI.
- [ ] **B5** a11y: `Modal` focus-trap + restore + max-height/scroll; `Select` keyboard nav; `LedgerRow` keyboard reachable.

## Phase C — Simplification (the dominant theme)

- [ ] **C1 Decision-archaeology comment sweep** (the #1 finding, 40+): cut narration to a one-line *why* + bare cross-ref across `apps.rs`, `browser.rs`, `terminal.rs`, `migrations.rs`, `project.rs`, `ruleMatch.ts`, `appIcons.ts`, `main.swift`, the migration SQL headers, the constant docs, `Cargo.toml`, etc. Keep genuine invariant/UAF comments. **Fix the 2 stale comments** (`0001` rename, `migrations/README.md` chain).
- [ ] **C2 Rust micro-dedup**: `load_lookup_maps(conn)` for the 3 command handlers; `query_one_opt` helper; merge `RawRunBuilder`→`SegRun` (+ `TimelineRun` via struct-update/`From`); `best_icns` → `max_by_key`; reuse `ERROR_THRESHOLD` (not bare `6`); de-dup `parse_site`/`parse_url` port-strip; unify `DbConnection`/`DbState` to one alias.
- [ ] **C3 Dead code**: delete `FocusEvent.bundle_id`/`.pid`; remove single-use indirection (`DetailInspector`) + any unused primitives; `ignore_title` dead field check.
- [ ] **C4 Design tokens**: sweep hardcoded colors → tokens (`TitleBar`, `ThemeSwitcher`, `CategoryEditorModal` PALETTE, `TimelineRow`, `RadioGroup`, `Modal` overlay, `index.css` shimmer) and raw `[px]` → scale.
- [ ] **C5 Frontend dedup**: shared `ErrorState` + `useCaptureHealth` + one `DayStepper`; `Promise.all` for independent rule writes; fix `getWeek` doc + `dates.ts` half-open bounds (pairs with A3/A5); microcopy ("switches" vs segments; stale `ctx` naming).
- [ ] **C6 `db.rs` split** into `db/{events,projects,categories,exclusions,settings,mod}.rs` (organizational; re-export; no behavior change).
- [ ] **C7 Config hygiene**: real `Cargo.toml authors`/description + `package.json name`; drop unused `tokio` features (`time`/`sync` — verify).

## Phase D — Deferred

### Native-only (verify on-device, not CI — D32/D33)
The `objc2`/AX/run-loop code is verified on a real Mac, not in CI (0% CI coverage by design). These need an on-device session; making blind run-loop/threading edits risks a UAF/capture regression CI can't catch:
- [ ] **A2c + B4** — move `emit_with_title`'s `osascript`/`pgrep` enrichment **off the main run loop** (worker + channel), which also lets the producer dedupe on full identity (title+url+cwd+private) instead of title-only, so same-title URL/cwd navigation isn't dropped. (Done together — A2c's cheap full-identity dedupe depends on B4's off-loop enrichment.)
- [ ] AX-notification registration errors ignored (`macos/mod.rs:261`); Chrome Canary → stable Chrome AppleScript name (`browser.rs:32`). _(info)_

### Need a decision (NOT in this cleanup)

- [ ] **D-perf** `Arc<Mutex<Connection>>` → R57 (dedicated writer thread + separate WAL read connections). Correct today; upgrade gated on real perf data (big-history rollup contention). Stays a Phase-6 hardening item.
- [ ] **D-privacy** Chromium-keep-title-when-Automation-denied (D33/R18). A product privacy posture — surface to the owner; do not change unilaterally.

## Sequencing / DoD
Phases A→B→C, gates green after each, logical commits. Update `plan.md` (Phase 6), append an ADR if a decision is made (e.g. the `CategorySlice.color`/IPC change, or adopting the targeted coverage standard), write a handoff. `bindings.ts` regenerated via the export test (A1/A5 are IPC-additive).
