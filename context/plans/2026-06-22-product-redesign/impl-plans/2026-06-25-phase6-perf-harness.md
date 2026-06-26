# Impl-plan — Phase 6: perf harness + read-path fix + churn + security (as built)

**Branch:** `phase6/perf-harness` (off merged `main`) · **Date:** 2026-06-25 · **Status:** built,
all gates green, PR pending. Findings: `../explorations/2026-06-25-perf-baseline.md`.

> **Scope note:** this began as a measure-only session-1 (harness + baseline). Per the owner's
> follow-up ("do all of Phase 6 this session"), it expanded to also **fix** the read path,
> **stress** the write path, and run the **security/dependency audit** — all in this branch. The
> sections below are the as-built; the "Expanded scope" section at the end covers the fix/churn/security.

## Goal

Build a reusable load harness; measure the read path at scale; then fix what it surfaces, stress
the write path, settle the memory question, and pass a dependency/security audit.

## What landed

- **`perf` cargo feature** (`src-tauri/Cargo.toml`) gating all of the below so nothing ships in
  the default binary or touches CI default lanes. Plus a `[[bin]] seed_db` with
  `required-features = ["perf"]` (default builds skip it cleanly).
- **`db::bulk_insert_events`** ([src-tauri/src/db/events.rs](../../../../src-tauri/src/db/events.rs),
  `#[cfg(feature = "perf")]`) — one prepared statement, caller-wrapped in a transaction. Takes
  `&[(NewEvent, end_time)]` because `insert_event` models an *open* span (`start == end`) and
  can't seed real durations. SQL stays in the repository layer (hard rule 4).
- **Synthetic generator** ([src-tauri/src/perf.rs](../../../../src-tauri/src/perf.rs)) —
  deterministic SplitMix64 PRNG (no `rand`/wall-clock), a ~14-app catalog across the five seeded
  categories (+ one uncategorized), 3 projects via `resolve_or_create_project`, browser
  sites/urls. ~960 short clustered spans/day across a morning→evening window with idle gaps. All
  catalog strings are `'static`, so every `NewEvent` borrows without allocation. Per-day
  transaction keeps memory flat; ~1.5 µs/row.
- **Read-path timing harness** — in-crate `#[ignore]` test (`perf::tests::perf_read_baseline`),
  in-crate so it can call the `pub(crate)` `rollup::build_recap_facts`. Seeds 1/6/12/24-month +
  a 36-month "heavy" (≥1M rows) tier; times `get_day`/`timeline`/`recap` **end-to-end** (each
  re-runs `get_activity_logs`, like the real command) + `get_week` + an isolated build-only
  column; prints `EXPLAIN QUERY PLAN`.
- **`seed_db` bin** ([src-tauri/src/bin/seed_db.rs](../../../../src-tauri/src/bin/seed_db.rs)) —
  writes a real on-disk WAL `usage.db` for the soak/webview session. Refuses to clobber without
  `--force`. Verified end-to-end (28,269 events / 30 days / 3 projects / 1-mo).

## Deltas from the approved plan

- The timing harness measures each read command **end-to-end with its own query** (plus a
  separate build-only column), rather than reusing one fetched Vec — this is how the real
  commands run (they each lock + query), and it makes the query-vs-build split explicit. (First
  run reused the Vec and mislabeled units as "m"; corrected to ms + per-command queries.)
- Budgets remain **proposed, pending owner ratification** (no ADR yet).

## Expanded scope (same session): fix + churn + security

- **Read-path fix** — `get_activity_logs` ([db/events.rs](../../../../src-tauri/src/db/events.rs))
  gains a lower bound `start_time >= start − MAX_SPAN_LOOKBACK_SECS` (2 days) so `idx_start_time`
  does a bounded range scan = O(window) instead of walking all history. One change, no migration,
  planner-independent. **Result: `get_day` 52.97→1.59 ms, `get_week` 370.9→10.5 ms @ 1.05M rows;
  flat across all scales.** Regression test added (`db::tests::get_activity_logs_lower_bounds_*`).
- **Write-path churn test** — a dev-only `capture::WriteProbe` ([capture/mod.rs], `#[cfg(perf)]`)
  exposes the private `on_focus` write path; `perf::tests::perf_write_churn` drives 50k switch
  events + a concurrent reader. **Result: 45k writes/sec; contention only under tight-loop
  concurrency → R57 deferred with data.**
- **Security/dependency audit** — `src-tauri/deny.toml` (committed gate, macOS-scoped). Fixed
  **2 vulns** (`bytes`→1.11.1, `time`→0.3.47 in `Cargo.lock`), declared `license = "MIT"` on our
  crate, allow-listed permissive licenses, documented-ignored the unmaintained transitives.
  `cargo deny check` → advisories/bans/licenses/sources **ok**.
- **Cargo.toml** also gained `default-run = "usage-os"` (the new `seed_db` bin made it a
  multi-binary package, which broke `tauri dev`'s bare `cargo run`).
- **Write-path span cap (from the `/usageos-review` cross-model Critical)** — `capture::on_tick`
  now splits a span active past `MAX_OPEN_SPAN_SECS` (12 h) into contiguous same-window pieces
  (`OpenSpan.start` added), so no stored span outlives the 2-day read lookback. This makes the
  read-path bound **complete by construction**, not an assumption. Read-time segmentation
  re-coalesces the pieces → invisible in the UI. Test:
  `capture::tests::tick_caps_marathon_span_into_contiguous_pieces`. See
  [reviews/2026-06-25-phase6-perf-harness.md](../reviews/2026-06-25-phase6-perf-harness.md).

## Gates (all green)

`cargo fmt --check` · `cargo clippy --all-targets -D warnings` (default **and** `--features
perf`) · `cargo test` **127** passed / 1 ignored · `tsc --noEmit` · `vitest` 32 · binding-freshness
(no `bindings.ts` change — no command/specta surface touched) · **`cargo deny check` clean**.

## Reproduce

```sh
cargo test --manifest-path src-tauri/Cargo.toml --release --features perf \
  perf_read -- --ignored --nocapture
cargo run --manifest-path src-tauri/Cargo.toml --features perf --bin seed_db -- \
  --months 24 --out /tmp/usage-24mo.db
```
