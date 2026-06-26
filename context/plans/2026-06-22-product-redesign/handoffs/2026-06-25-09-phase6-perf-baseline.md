# Handoff — 2026-06-25-09 · Phase 6 Session 1 done: **scale + read-path perf baseline** (harness built, measured; no fixes — measure-first)

**Where we are now:** Phase 4 is **merged** ([PR #19](https://github.com/f-gozie/usage-os/pull/19), squash `96f8685` on `main`). Phase 6 is underway, led by stress/perf (owner's 2026-06-25 resequencing: harden + perf-test before the Phase-5 launch). **Session 1 is complete on branch `phase6/perf-harness`** (off `main`): a reusable load harness + the read-path scale baseline. **All merge gates green. PR pending.** Impl-plan: `impl-plans/2026-06-25-phase6-perf-harness.md`; findings: `explorations/2026-06-25-perf-baseline.md`.

## Correction to handoff 08 (append-only — not editing it)
Handoff `2026-06-25-08` line 24 says the DB has **"no WAL yet (R57)"** — that's **stale/wrong**. WAL `journal_mode` **is on** ([db/mod.rs:184](../../../../src-tauri/src/db/mod.rs)): `PRAGMA journal_mode = WAL` is set at `init_database`. What R57 actually defers is the **dedicated writer thread + separate read connections**, not WAL mode itself. (Tests open in-memory with `foreign_keys` only — no WAL — which is fine.) Carry this corrected statement forward.

## What landed this session
A measurement harness behind a `perf` cargo feature (never ships in the default binary or CI default lanes):
- **`db::bulk_insert_events`** (repo layer, hard rule 4) — prepared-statement batch seed; takes `(NewEvent, end_time)` since `insert_event` only models open spans.
- **`src-tauri/src/perf.rs`** — deterministic SplitMix64 generator: ~14-app catalog across the 5 seeded categories (+ 1 uncategorized), 3 projects, browser sites/urls, ~960 short clustered spans/day with idle gaps. Plus the in-crate `#[ignore]` `perf_read_baseline` timing test.
- **`src-tauri/src/bin/seed_db.rs`** (`required-features=["perf"]`) — writes a real on-disk WAL `usage.db` for the soak/webview session; refuses to clobber without `--force`. Verified end-to-end.

Gates: fmt ✓ · clippy `-D warnings` default **and** `--features perf` ✓ · `cargo test` 125✓/1 ignored · tsc ✓ · vitest 32 ✓ · bindings fresh ✓ (no IPC/specta surface touched).

## The findings (numbers that drive the rest of Phase 6)
Release, in-memory SQLite, median of 9, recent-day read:

```
tier            rows   get_day  timeline   recap     week    day_build
1mo            28269    2.91ms    2.85ms   2.87ms  18.54ms     1.18ms
6mo           174917   10.00ms    9.59ms   9.68ms  67.27ms     1.20ms
12mo          350865   18.63ms   18.63ms  18.72ms 129.71ms     1.20ms
24mo          702833   35.83ms   36.08ms  36.10ms 251.90ms     1.23ms
heavy-36mo   1055461   52.97ms   52.63ms  52.77ms 370.93ms     1.21ms
```
`EXPLAIN`: `SEARCH activity_logs USING INDEX idx_start_time (start_time<?)`.

**Headline: the read path is O(total history), and ~100% of that growth is the unbounded overlap query — the rollup build is flat ~1.2 ms (28k→1M rows) and is NOT a bottleneck.** `get_activity_logs`'s `start_time < ?end` (only `idx_start_time`) scans ~the whole table for a recent day, then row-checks `end_time > ?start`. `get_day`≈`timeline`≈`recap` at each tier (all dominated by the same scan). **`get_week` (7 scans) bends first** — breaches a 150 ms bar at **~15 months**; single-day reads breach 50 ms at **~33 months (~1M rows)**. Default retention is **"0" = forever** → unbounded. Corollary for the 2.13 GB question: the Rust read path holds only ~1,030 events/day in memory regardless of history — any real climb is **webview-side**.

## NEXT (in order)
1. **Owner: ratify the perf budgets** (proposed in the findings doc): `get_day`<50ms, `get_timeline`<100ms, `get_recap`<100ms, `get_week`<150ms (recent day, ~2yr history). Once ratified → an ADR in `decisions.md`. **The Week-at-~15-months breach is the gating problem.**
2. **Index/query fix session** (the first *fix* of Phase 6) — implement + **measure each** against this harness (re-run `perf_read`): **(A, recommended)** add an index on `end_time` so the planner drives off the *selective* `end_time > ?start` bound (a recent day → ~O(day)); **(B)** collapse the Week into one ranged query partitioned in Rust; **(C)** share one fetch per day-open across `get_day`/`get_recap`/Timeline. A is the headline; B/C additive. Bar: whole table back under budget at heavy-36mo. (A is an additive migration v8 + repo-layer query — no IPC change.)
3. **Soak/memory session** — use `seed_db --months 24` → a release build long-run + RSS monitor to settle the 2.13 GB observation (look webview-side). Harness is built once, reused.
4. **Capture-churn / R57** (writer-thread contention) and **recap/sidecar spawn load** — their own harnesses, later in Phase 6.

## Open / to confirm
- **Open PR** for `phase6/perf-harness` (consider `/usageos-review` first — it's the recommended pre-PR step). After merge, the fix branch (e.g. `phase6/read-path-index`) forks from `main`.
- The 2 pre-existing untracked items (`reviews/2026-06-25-full-codebase-audit.claude-findings.json`, `design/logo/`) remain untracked.
- Budgets are **proposed, not yet ratified** — no ADR written this session.
