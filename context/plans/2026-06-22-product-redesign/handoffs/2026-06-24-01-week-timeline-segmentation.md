# Handoff — M2 Week + M3 Timeline + D34a segmentation (excursion-absorb), lock-time fix

_Written 2026-06-24 (after [`2026-06-23-06-agent-supervision-tracking.md`](2026-06-23-06-agent-supervision-tracking.md)). Read `CLAUDE.md` first, then this. A long session: shipped the **Week (M2)** and **Timeline (M3)** views, then **resolved D34a** (context-run segmentation / excursion-absorb) via a two-round Codex↔Opus debate, and fixed a real **lock-time capture bug** found by dogfooding. All on branch **`phase1/bauhaus-day`** → **[PR #13](https://github.com/f-gozie/usage-os/pull/13)** (open, not merged). HEAD = `227794e`._

## 1. Current state overview
- **Product:** UsageOS — a private, on-device macOS "calm rear-view mirror" of where time goes.
- **Phase:** Phase 1 complete; **Phase 2 well underway** — Week (M2) ✅ and Timeline (M3) ✅ both shipped this session, and the **D34a segmentation model is now resolved + tuned** (D40/D41). The dial / week mini-dials / timeline all share one read-time segmentation engine.
- **Branch/PR:** everything is committed + pushed to `phase1/bauhaus-day` (PR #13). Nothing uncommitted.

## 2. Key decisions (this session; full ADRs in `context/decisions.md`)
- **D39 — agent-supervision idle gate.** Per-app idle gate: default `GATE_SECS=120`, `PATIENT_GATE_SECS=600` for agent/dev apps (`PATIENT_APPS`), **never disabled** (cap bounds over-count). No agent *detection* (no privacy-safe signal separates watching from away). Removed the vestigial producer-side `is_idle` (consumer is the single idle source). Decided via Codex↔Opus debate. Claude desktop app → Deep work (migration `0004`). Day headline tile shows the **leading context** (was a hard-coded "Deep work" that read "14s").
- **D40 — context-run segmentation + excursion-absorb (unified read-model).** A sandwiched, contiguous non-X block whose **wall-clock span ≤ `ABSORB_SECS`** folds into the surrounding X-run; the detour stays a *segment* (in the Timeline expand, carrying its own context). **Two orthogonal guards:** local dominance (host active ≥ excursion) + accumulation cap (`MAX_ABSORB_FRACTION_PCT` of run wall). **Run headline numbers are host-context-only** (a "Deep · 52m" run = 52m of Deep; totals are structurally independent of segmentation). **Unified:** one `build_segmented_runs` is the single source; `ContextRun` is it minus `segments`; equivalence-tested. Post-process fixpoint merge (not lookahead). Rejected: sustained project sub-split, persisted sessions, hysteresis/S4, per-span absorb.
- **D41 — lock/away time isn't counted; absorb tuned 90→180s.** `loginwindow`/`ScreenSaverEngine` frontmost ⇒ **away** (the reliable lock signal; input-idle reads ~0 while locked, so the gate is blind) → routed through the `Excluded` path. **`ABSORB_SECS` raised 90→180** (first dogfood tune — real switches cluster at 90–300s). Migration `0005` clears historical phantom away-spans. Rejected: lock-state via `CGSSessionScreenIsLocked` (more native surface), treating `SecurityAgent` as away (it's a brief auth modal during use).

## 3. Changes implemented (files)
**Backend (Rust, `src-tauri/src/`):**
- `capture/mod.rs` — per-app `gate_secs_for` + `PATIENT_APPS` (D39); removed `FocusEvent.is_idle`; `AWAY_APPS`/`is_away_app` + away check in `resolve_focus` (D41).
- `capture/macos/mod.rs`, `capture/polling.rs` — removed the dead producer-side idle (`IDLE_THRESHOLD_SECS`, `is_user_idle`) (D39).
- `rollup.rs` — **the big one.** Week: `DaySlice`/`WeekView` + `build_day_slice`/`build_week_view`. Timeline + D40: `TimelineSegment` (now carries `context_slug`/`context_name`), `TimelineRun`, `TimelineView`; `SegRun`/`RawRunBuilder`; `raw_runs` → `absorb_excursions` (fixpoint) → `build_segmented_runs`; `build_runs` is now a **projection** of that; `build_timeline` returns it whole. Constants `ABSORB_SECS=180`, `MAX_ABSORB_FRACTION_PCT=15`, `IDLE_GAP_ENDS_RUN_SECS=300`. Deleted old `RunBuilder`/`project_of`.
- `lib.rs` — `get_week(day_starts, week_end)`, `get_timeline(start, end)` commands + registration.
- `migrations.rs` + `migrations/0004_recategorize_claude_as_deep.sql` (Claude→Deep + backfill) + `migrations/0005_drop_away_app_spans.sql` (delete loginwindow/ScreenSaverEngine rows). Migration chain now at **version 5**.
- `bindings.ts` regenerated (additive: WeekView/DaySlice/TimelineView/TimelineRun/TimelineSegment; `get_week`/`get_timeline`).
**Frontend (React, `src/`):**
- `lib/dates.ts` — `startOfWeek`, `weekDays`, `formatWeekRange`.
- `lib/tauri.ts` — `getWeek`/`getTimeline` wrappers + type re-exports.
- `hooks/useWeekData.ts`, `hooks/useTimelineData.ts` (mirror `useDayData`).
- `components/dial/MiniDial.tsx` (+ stories/test) — compact dial reusing the geometry.
- `components/timeline/TimelineRow.tsx` (+ stories/test) — collapsible run row; marks absorbed detours with a context dot.
- `views/WeekView.tsx` (+ test), `views/TimelineView.tsx` — the two new views.
- `App.tsx` — wired Week + Timeline (replaced placeholders); per-view header date.
- `components/shell/AppShell.tsx` — **invariant header height** (reserve the date slot) so the chrome doesn't shift between tabs; DayView headline tile → leading context.
**Docs:** `decisions.md` (D39/D40/D41), `plan.md` (ticked), `impl-plans/2026-06-23-week-view.md`, `impl-plans/2026-06-24-timeline-view.md`, this handoff.

## 4. Progress completed
- **Week view (M2)** — Sun→Sat mini-dial grid, summary (active / avg / deepest day), click-a-day → Day. Hover scales the dial.
- **Timeline view (M3)** — agenda of context-runs, click-to-expand into app-switch segments, Away/Now rows. This is the **session explorer**.
- **D34a resolved (D40/D41)** — unified segmentation + excursion-absorb; tuned to 180s on real data.
- **Lock-time bug fixed (D41)** — ~46 min/day of phantom "active" eliminated; `0005` cleans history.
- **Headline tile** + **Claude→Deep** + **invariant header chrome** fixed.
- **Gates green:** 87 Rust + 14 TS tests, clippy/fmt/tsc, vite + storybook builds.

## 5. Current blockers
- **None hard.** PR #13 is a large unmerged stack (D35→D41 + M2/M3) — review/merge when ready.
- Apple Developer enrollment (notarization/distribution) — off the critical path.

## 6. Work in progress
- **Nothing uncommitted.** Last action: pushed `0005`. The immediate next step was **the user reopening the app** to see the calmer dial + corrected total (today drops ~46 min → ~4h07m).

## 7. TODO (remaining)
1. **Reopen + eyeball** the dial (applies `0005` + the new segmentation to all past days at read time). If still too granular: nudge `ABSORB_SECS`→240 or revisit `MAX_ABSORB_FRACTION_PCT` (15%). The Timeline is the gauge.
2. **Dogfood a few more days** → lock `ABSORB_SECS` / cap / `GATE` / `PATIENT_GATE` against lived memory (D34a/D39 are still "tunable, not locked").
3. **M4 — Settings (Phase 2/4):** contexts/rules editor (+ user-editable exclusions & a patient-app allowlist), retention picker, **Export CSV** + **Delete-all**; retires the legacy `Category` IPC naming. (`usage-os`/`Nudge` stay uncategorized until then — user's call.)
4. **Phase 3 — recap:** Swift `usageos-ai` Foundation Models sidecar (stdio, structured output) + availability check; the deterministic template recap already exists.
5. **Review + merge PR #13.**
6. Phase 4 polish (menubar, onboarding/permission priming, dark-mode parity, day-start offset D14) → Phase 5 launch (DMG, auto-update, Homebrew).

## 8. Important context / gotchas
- **Migrations are immutable once applied** (FNV checksum drift guard → startup panic). **Never edit an applied migration** — add a new one. (This session: editing `0004` mid-dev-run crashed the app; recovery = `DELETE FROM schema_migrations WHERE version=N` on the dev DB, then it re-applies. Only do that for unshipped/local.)
- **Segmentation is read-time** (`rollup`): changing `ABSORB_SECS`/gaps re-segments **all** past days on next launch — no recapture needed. **Totals are independent of segmentation** (D34) — absorb never moves a second between context totals.
- **Tunable knobs** (all dogfood-tunable, none locked): `ABSORB_SECS=180`, `MAX_ABSORB_FRACTION_PCT=15`, `IDLE_GAP_ENDS_RUN_SECS=300` (rollup); `GATE_SECS=120`, `PATIENT_GATE_SECS=600`, `PATIENT_APPS`/`AWAY_APPS` (capture).
- **Live DB (for read-only dogfood diagnosis):** `~/Library/Application Support/com.favour.usage-os/usage.db`. Reconstructing runs in SQL (window fn over `activity_logs`, split on context-change or 5-min gap) is how this session profiled fragmentation — but post-absorb runs aren't easily reproduced in SQL (it's a fixpoint merge), so trust the Rust tests + reopen.
- **`/debate`** (Codex CLI + Opus, neutral brief, 2 rounds) drove D38/D39/D40 — the pattern for hard/core calls. Codex output can be large; pipe `| tail`.
- The `Grep`/`rg` tool occasionally garbles matched substrings in its echo (cosmetic) — read files directly to confirm exact text.

## 9. Testing status
- **Green:** 87 Rust tests (incl. 7 absorb tests, the `away_app` test, the `build_runs == project(build_timeline)` equivalence test, week/timeline rollup tests) + 14 TS tests (MiniDial, TimelineRow, WeekView RTL); clippy `-D warnings`, fmt, tsc, vite + storybook builds; bindings fresh.
- **Capture is mockable** (`FakeCapture`); on-device behaviour verified by dogfooding against the live DB.
- **Still needs validation:** the absorb thresholds against more real days (the point of the session-explorer) before locking D34a.

## 10. Next steps recommendation
1. **Reopen the app, look at the dial** (Day + Week + Timeline). Confirm: total drops ~46 min, gray lock arc gone, dial calmer. This is the cheap, high-signal check.
2. If granularity still bugs you → bump `ABSORB_SECS` (rollup.rs) toward 240 and/or relax the cap; re-look. One-line changes, read-time effect.
3. Otherwise pick the next build: **M4 Settings** (unlocks user-tuning of rules/exclusions/thresholds — natural follow-on to all the dogfood-tunable knobs) **or merge PR #13** first to shrink the stack.

**Read order:** `CLAUDE.md` → `context/plans/README.md` → this plan's `plan.md` + this handoff → `context/decisions.md` (D1–D41) → `npm run tauri dev` + `npm run storybook`. For hard calls, use **`/debate`**.
