# Review — remaining review findings (`fix/review-followups`)

**Date:** 2026-10-19 · **Scope:** branch (`main...HEAD`) · **Files:** 30
**Plan:** [plan.md](../plan.md) ("Review follow-ups for 0.1.5") · **Impl-plan:** none — the brief was the open items in [2026-10-19-watch-coverage-calendar-recap.md](2026-10-19-watch-coverage-calendar-recap.md)
**Codex:** ran — no findings, "patch is correct"

## Merge gates

| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy --all-targets --all-features -D warnings | ✅ |
| cargo test | ✅ 185 passed |
| tsc --noEmit | ✅ |
| vitest | ✅ 97 passed |
| bindings fresh | ✅ |

## Findings

**Verification:** 1 Claude lane (hard rules + correctness + simplify/copy, the diff being small) and Codex. 6 findings, all verified against the code, all fixed. No Criticals.

### Critical
None. No hard-rule violations: no network, no new commands, no prod `unwrap`, new SQL only in `db/coverage.rs` with bound parameters, tokens only.

### Warnings
- `src-tauri/src/rollup.rs` — "Most of it was on the usageos project." overstated a project that leads at 40%, and after the break sentence "it" read as the break. **Fixed:** "The usageos project took the most time." (board script too).

### Info (all fixed)
- `rollup.rs` parts-merge comment overclaimed ("adjacent or not"); now says parts merge unless a different one sits between them.
- `rollup.rs` `render_template_recap` compared `MOSTLY_UNWATCHED_SECS` directly; now `is_unrecorded`.
- `db/coverage.rs` `CoverageSpan` doc still said an open span ends at the last heartbeat.
- `lib.rs` `get_year` read `now` twice; now once.
- `CalendarView.tsx` double blank line; `YearGrid.test.tsx` mocks renamed to `onOpen`.

### Verified safe (the lane's checks worth keeping)
- Open-session-to-now: startup reconciles dead sessions inside `setup` before any command can read; Delete all keeps the live row open at the wipe; Glance reads through the same path.
- A late heartbeat after the exit close reopens the row for seconds and is reconciled to its `last_seen` on the next launch; `AUTOINCREMENT` ids mean a stale id can't reopen another row.
- `coverage_gaps` edge fix only changes the year-wide read, and correctly.
- `day_slices`, the `unrecorded` flag, `MonthGrid` reuse and the single `onOpen` behave as before.
- `sqlite3 .backup` works on a WAL database under a path with a space.

## Definition of Done
- [x] plan.md: "Review follow-ups for 0.1.5"
- [x] decisions.md: D72 addendum (coverage reads, heartbeat reopen, `unrecorded` flag)
- [ ] impl-plan: none (brief was the previous review's open list)
- [x] docs move with code

## Plan compliance
Alignment: good — exactly the open items from the previous review, plus the six fixes above. One item deliberately kept as is (parts that agree merge across a quiet part) and one not addressed (`text-[7.5px]` legibility, needs eyes on device).
