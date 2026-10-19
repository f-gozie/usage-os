# Review — watch coverage, Calendar, day-shape recap (`feat/watch-coverage`)

**Date:** 2026-10-19 · **Scope:** branch (`main...HEAD`, 14 commits: D72, D73, D74, D75, jump-to-latest, Calendar/hover fixes) · **Files:** 42 code files (54 incl. docs/boards)
**Plan:** [plan.md](../plan.md) · **Impl-plan:** none for this work — see Definition of Done
**Codex:** ran (`codex-cli 0.153.4`, 5 findings, "patch is incorrect")

## Merge gates

| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy --all-targets --all-features -D warnings | ✅ |
| cargo test | ✅ 174 passed |
| tsc --noEmit | ✅ |
| vitest | ✅ 93 passed |
| bindings fresh | ✅ |

Re-run after the auto-fixes below: all still green.

## Findings

**Verification:** 4 lanes (hard rules, correctness, simplify/copy, Codex). 14 Critical/Warning findings verified against the code; the correctness lane reproduced its recap findings with scratch tests on a copy of the crate. 3 cross-model confirmed (Codex + a Claude lane). Lane A found no hard-rule violations.

### Critical (must fix before merge) — all four fixed, see Follow-up

- `src/components/dial/Dial.tsx:122-139` — **[Codex]** A whole day UsageOS wasn't running draws no broken ring. `arcPath(…, 0, 1440)` starts and ends at the same point, so the SVG arc is empty and the dial shows the solid "you were away" ring — the exact lie D72 exists to remove. Fix: draw a full-day gap as a circle (or two half arcs) and add a test for `0 → 1440`.
- `src-tauri/src/db/settings.rs:45-54` — **[Codex]** "Delete all data" leaves `watch_coverage` behind, so the history of when the app ran survives the wipe and past gaps keep drawing. Fix: delete coverage rows in the same transaction, then open a fresh session (the live session id is held by capture, so it must be re-created, not just deleted). Migration-adjacent logic: manual.
- `src-tauri/src/rollup.rs:995-1006` — **[correctness, reproduced]** The recap's "active from … until …" counts only runs of 5+ minutes, while the parts of the day count everything. An evening of 4-minute switches is outside the range but still described: "active from around 9 in the morning until around 2 in the afternoon… The evening went to Messaging and Browsing." The break search stops at the same edge, so it reported 4h 30m when the real longest break was 5h 50m. Fix: take the range from the first and last active events after the overnight tail.
- `src-tauri/src/rollup.rs:1002` — **[correctness, reproduced]** After-midnight time counts toward the 1-hour minimum for a range, so a 10-minute day after a 3-hour night reads "You were active from around 10 in the morning until around 10 in the morning, 3h 10m in all, counting 3h 0m just after midnight." Fix: apply the minimum to time inside the range, and drop the range when both ends round to the same time.

### Warnings (should fix)

- `src-tauri/src/rollup.rs:1005`, `clock_phrase`, `PARTS` — **[Codex + correctness, cross-model]** Local time is computed as `t - day_start` and "still going" as `now < day_start + 86_400`. On the two daylight-saving days clock phrases are an hour off and the ongoing check is wrong for one hour. The pattern predates this branch (`time_of_day_phrase`), but D74 leans on it much more. Fix: pass the real day end from `get_day`; derive clock time with `localtime_r` (`libc` is already a dependency).
- `src/views/CalendarView.tsx:88,106-110,151` with `useViewData` — **[Codex + correctness, cross-model]** Stale data shows during range changes: stepping months shows the previous month under the new heading, and switching Year ↔ Month shows zeros (the hidden rung fetches with an empty range and overwrites its data). Fix: an `enabled` flag on `useViewData`, and treat a range change as a fresh load.
- `src-tauri/src/rollup.rs:1218` — **[correctness + copy, cross-model]** A finished empty day still says "No activity tracked today yet." — now reachable from every empty Month cell. Fix: "Nothing was tracked on this day." once the day has ended, plus the unwatched sentence when it applies.
- `src/index.css:176,225` — **[Codex]** The Calendar's entrance animations used `both`, holding `transform: none` after they end, which overrides the hover lift and press. **Auto-fixed** (`backwards`).
- Recap wording (`rollup.rs` `render_template_recap`) — **[copy]**
  - "2h 0m" reads mechanically in sentences ("10h 0m in all", "The longest break was 3h 0m"): use a prose formatter that drops "0m".
  - "counting 1h 9m just after midnight" → "including 1h 9m just after midnight".
  - "It came in short pieces" has no clear subject after the places sentence → "The time came in short pieces; the longest ran …".
  - "Most of it came in long stretches, the longest 3h in the morning" has no verb → "…long stretches; the longest ran 3h in the morning."
  - "The longest break was 3h, from around 11 in the morning." → "…starting around 11 in the morning."
  - "Mostly on usageos." now follows "mostly on github.com" and reads like another site → e.g. "Most of it was for usageos."
- `src/views/CalendarView.tsx:38`, `src/components/calendar/yearModel.ts:11`, `TimelineView.tsx:156`, `Glance.tsx:87` — **[simplify]** The 23-hour "not recorded" threshold is copied four times in TS while Rust owns it. Fix: Rust returns an `unrecorded` flag (rule 6), or one exported constant.
- `src/views/CalendarView.tsx:342-372` — **[simplify]** `MonthGrid` re-derives `dayState`/`dayLabel`, the date formatter and the click delegation already in `yearModel.ts`/`YearGrid.tsx`.
- `src/views/CalendarView.tsx:463-470` — **[simplify]** The Calendar legend hardcodes category names and colours, ignoring renames (D47); `TimelineView` already builds its legend from `CANONICAL_CATEGORIES` + `categoryDisplayName`.
- `src-tauri/src/lib.rs:241,278` — **[simplify]** `get_week` and `get_month` repeat the same per-day loop; extract one helper.
- `src-tauri/src/rollup.rs:959-1075` — **[simplify]** `compute_recap_facts` is ~116 lines; the tail/span, break and texture parts want their own helpers like `day_parts` and `places_of`.

### Info

- `src-tauri/src/lib.rs:126` / `db/coverage.rs:77` — after wake from sleep, the live session reads up to its last heartbeat, so for up to ~20s the day can show a false multi-hour gap. Extend the live session to `now` in `day_coverage_gaps`.
- `src-tauri/src/db/coverage.rs:135` — the 5-minute gap minimum is checked before clipping to the day, so a sub-5-minute gap can appear at a day edge.
- `src-tauri/src/rollup.rs:1126` — a browser is only recognised by having recorded a site that day, so Firefox or a private-only day gives "Browsing was mostly in Firefox". Use the capture browser list.
- `src-tauri/src/rollup.rs:1109` — non-adjacent parts merge ("The morning and evening went to Work"); reads fine, but the comment says neighbours.
- `src-tauri/src/lib.rs:1034` — a second normal launch closes the first instance's open coverage record at its last heartbeat; a false gap follows if the first is later SIGKILLed.
- Dead code: `ai::sidecar::prewarm` (kept with the D74 seam), `startOfYear` in `dates.ts`; `YearLayouts`' `onOpenDay` is always `openMonth`.
- Backstory comments in `yearModel.ts`, `YearLayouts.tsx`, `CalendarView.tsx`, `TimelineView.tsx`, `DateStepper.tsx` could shrink to the rule plus a D-number.
- Copy: "Tracked this month" on a past month; "Week of 12–18 October 2026" back label is redundant; `useCalendarData` error strings use straight apostrophes (like their siblings); the refresh tooltip promises 30s polling, which Year doesn't do.
- `scripts/dev-with-restore.sh:49,61` — reverts only 0009's table (a future 0010 would be left half-reverted), and copies the DB without a WAL checkpoint first.
- `YearLayouts.tsx:147` — `text-[7.5px]` is below anything on `main`; check legibility.

## Auto-fixes applied

- `src/index.css` — `.cal-bar` and `.cal-daycell` entrance animations `both` → `backwards`, so hover and press transforms apply (with a one-line comment).
- `src-tauri/src/lib.rs` — the run-loop closure's `_app_handle`/`_event` renamed to `app_handle`/`event` and the stale comment removed (the Exit arm uses both on every target).
- `src/components/calendar/YearGrid.tsx` — removed re-exports nothing imports.

## Manual TODO

- [x] Full-day gap draws as a broken circle on the Day dial (+ test).
- [x] "Delete all data" clears `watch_coverage`; the live session keeps its row, restarted at the wipe (+ test).
- [x] Recap range from first/last activity, short end groups ignored; minimum measured inside the range; no "from X until X" (+ 3 tests).
- [x] Past empty day: "Nothing was tracked on this day." (+ unwatched sentence, + test).
- [x] Recap wording: prose durations without "0m", "including", "The time came…; the longest ran …". "starting around" and the project sentence done on `fix/review-followups`.
- [x] Real day end for "still going"; local clock time for phrases (DST).
- [x] `useViewData` stale data on range change / hidden rung.
- [x] Simplify: shared threshold, `MonthGrid` reuse of `yearModel`, legend from `CANONICAL_CATEGORIES`, shared per-day loop, split `compute_recap_facts` (the last done on the branch; the rest on `fix/review-followups`).

## Definition of Done

- [x] plan.md ticked for what landed (Calendar, D74, D75 sections)
- [x] decisions.md ADRs appended (D72–D75)
- [ ] impl-plan present — **none** in `impl-plans/` for this branch's work
- [ ] handoff to follow at session end (the last is `2026-09-20-02-calendar-built.md`)
- [x] docs move with code (pre-push tripwire would not fire)

## Plan compliance

Alignment: partial — scope matches plan.md's Watch coverage, Calendar and Daily recap sections and their ADRs, but the work has no impl-plan, and four correctness findings (two in D74's recap, the full-day gap ring and Delete all data) need fixing before this branch is PR-ready.

## Follow-up (same day)

The four Criticals and the recap wording were fixed on the branch, each with a test:

- `src/lib/geometry.ts` — `arcPath` draws a span of a whole day as two half arcs; `Dial.test.tsx` pins the full-day gap.
- `src-tauri/src/db/settings.rs` — `delete_all_data(conn, now)` deletes closed coverage rows and restarts the open one at the wipe, so the heartbeat still lands and the past reads as unknown.
- `src-tauri/src/rollup.rs` — `active_bounds` (groups an hour apart, end groups under 5 minutes ignored) sets the range and the break search; the 1-hour minimum counts time inside the range; equal rounded ends say nothing; `night_tail_len`, `longest_break` and `texture_of` split out of `compute_recap_facts`; `prose_secs` replaces `human_secs`; a finished empty day reads "Nothing was tracked on this day."
- While this was being fixed the owner asked whether "counting 2h 0m just after midnight" was accurate: it was 2h of activity spread from midnight to 2:54am. The sentence now says when it stopped: "including 2h between midnight and around 3 at night".

Gates after the fixes: fmt ✅ · clippy `--all-features` ✅ · cargo test ✅ 180 · tsc ✅ · vitest ✅ 94 · bindings fresh ✅ (regenerated: `delete_all_data`'s doc comment). Checked against a copy of the real database for Oct 14 and Oct 19.

Then the two remaining behaviour Warnings:

- **Daylight saving** — `rollup::DayFrame { start, end, now, clock }` replaces the bare `day_start`/`now`: "still going" and "day over" use the real day end from `get_day`/`get_recap`; clock phrases, the parts of the day and the night's end read local clock time through the new `local_clock::secs_of_day` (libc `localtime_r` on macOS, UTC elsewhere), which the tray hour now shares. Test: a 23-hour spring-forward day phrases 8am as "around 8" and its parts by the clock, and is over an hour into the next day.
- **Stale Calendar data** — `useViewData` keeps each result with the range it was fetched for and reports `stale`, and takes an `enabled` flag; the hidden rung no longer fetches an empty range, and the Calendar dims its grid and makes it inert while the previous range is still showing (`aria-busy`). The Month grid remounts on its data's month, not the requested one. Two hook tests.

Gates after: fmt ✅ · clippy `--all-features` ✅ · cargo test ✅ 181 · tsc ✅ · vitest ✅ 96 · bindings fresh ✅.

Still open from the Warnings: the duplicated 23-hour threshold, `MonthGrid` reuse, the hardcoded legend, the shared per-day loop, and the Info items.

## Second follow-up (`fix/review-followups`, after v0.1.4 shipped)

Every remaining Warning and the Info items with a user-visible effect, each tested: the Calendar legend follows renamed categories; "Tracked in {month}"; coverage reads a running app to now (wake from sleep), a heartbeat reopens a session a second launch closed, no edge slivers; browsers never named as places; break and project wording; curly apostrophes; the refresh tooltip; the week back-link label; one Rust-owned `unrecorded` flag; `MonthGrid` reuses `yearModel`; one `onOpen` for the Year layouts; `day_slices` shared by `get_week`/`get_month`; dead code and backstory comments; a WAL-safe dev backup. Left as is, deliberately: parts of the day that agree merge even when not adjacent ("The morning and evening went to Work"), which reads better than repeating the category; the comment now says so. Not addressed: `text-[7.5px]` in `YearLayouts.tsx` (legibility, needs eyes on device).

