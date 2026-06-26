# Review — Phase 6: perf harness + read-path fix + churn + security

**Date:** 2026-06-25 · **Scope:** branch `phase6/perf-harness` (uncommitted vs `main`) · **Files:** 8 code
**Plan:** [plan.md](../plan.md) · **Impl-plan:** [2026-06-25-phase6-perf-harness.md](../impl-plans/2026-06-25-phase6-perf-harness.md)
**Codex:** ran (codex 0.x, read-only)

## Merge gates
| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy -D warnings (default **and** `--features perf`) | ✅ |
| cargo test | ✅ 127 passed / 1 ignored |
| tsc --noEmit | ✅ |
| vitest | ✅ 32 |
| bindings fresh | ✅ (no IPC surface touched) |
| cargo deny check | ✅ advisories/bans/licenses/sources ok |

## Findings
**Verification:** 4 raised · 4 verified · 0 dropped · 0 cross-model-confirmed (the one Critical was Codex-only; the Claude lanes saw the same code and judged it safe — adjudicated below).

### Critical — RESOLVED
- `db/events.rs` `get_activity_logs` — **[Lane D / Codex]** *Bounded scan drops valid overlapping spans.* The new `start_time >= start − MAX_SPAN_LOOKBACK_SECS` lower bound narrows the contract: a span overlapping the window but starting >2 days before it is dropped, and the write path only **assumed** (via the idle gate) that such spans can't exist — it didn't **enforce** it. Lanes A & B independently traced this and judged it safe for captured data (the idle gate closes spans in ≤600 s), but Codex is right that an assumed invariant on a repository API is a latent footgun (e.g. a future bulk-import of long spans).
  **Resolution (not just accepted):** made the invariant *provable*. The capture write path now **caps span length** at `capture::MAX_OPEN_SPAN_SECS` (12 h) — far below the 2-day read lookback — by splitting a marathon span in `on_tick`. The two pieces are contiguous and the same window, so **read-time segmentation re-coalesces them (zero UI change)**. New test `capture::tests::tick_caps_marathon_span_into_contiguous_pieces`; both the read-lookback doc and the cap doc cross-reference each other (D58). The bounded scan is now complete by construction, not heuristic.

### Warnings — FIXED
- `perf.rs` — **[Lane C]** `impl Default for SeedConfig` was dead code (all 3 call sites use struct literals; `::default()` never called). **Removed.**

### Info — FIXED / accepted
- `perf.rs:1` — **[Lane C]** module doc referenced a handoff slug (`handoff 2026-06-25-08`) — rot-prone decision-archaeology the project warns against. **Removed.**
- `perf.rs` `pick_app` / various — **[Lane C]** a few comments lightly over-explain; the dense perf-rationale comments are load-bearing and **kept by design** (they explain what the measurement isolates / why the scan is bounded).
- `SeedStats.days`/`.projects` echo inputs — **[Lane C, info]** mild over-reporting; left as-is (the symmetry reads fine and the bin prints all three).

### Verified clean (no finding)
- Hard rules 1–7 (Lane A): no network in the data path (`reqwest`/`hyper` resolved but **not in the build graph**); no new IPC/bindings; **no `unwrap`/`expect`/`panic` in the non-test `perf` code** (generator, `WriteProbe::feed`, `bulk_insert_events`, `seed_db` bin all use `?`/`Result`); all SQL in the repo layer (`bulk_insert_events` in `db/events.rs`; the test-only `COUNT`/`EXPLAIN` is consistent with existing test helpers); `WriteProbe` is `#[cfg(feature="perf")]`-gated, unreachable in the default build; no SQL injection (`format!` splices a const column list, values are bound params).
- Correctness (Lane B): read-fix param-binding order (`?1=scan_lo,?2=start,?3=end`), `saturating_sub`, the regression test arithmetic, the cross-midnight clip test, `bulk_insert_events` column/param order, the generator (no cross-day bleed, no modulo-by-zero, weighted pick), the churn test (no deadlock, `done` ordering, joins), and the `seed_db` arg parser — all traced and correct.

## Auto-fixes applied
- Removed dead `impl Default for SeedConfig`; dropped the handoff-slug doc reference; `cargo fmt`.
- **Hardening (manual, from the Critical):** write-path span cap (`MAX_OPEN_SPAN_SECS`) + `OpenSpan.start` + split in `on_tick` + test; honest contract docs on `MAX_SPAN_LOOKBACK_SECS` and the query.

## Manual TODO
- [ ] Open the PR (this review pairs with the impl-plan).
- [ ] (Optional, later) confirmatory release soak for the 2.13 GB question.

## Definition of Done
- [x] plan.md ticked for what landed (read fix, churn/R57, memory, security)
- [x] decisions.md ADR appended (**D58** — budgets ratified, bounded-scan fix, R57-deferred-with-data, cargo-deny gate)
- [x] impl-plan present; handoff written (`handoffs/2026-06-26-01-…`)
- [x] docs move with code (pre-push tripwire would not fire)

## Plan compliance
Alignment: **good** — scope matches the owner's "do all of Phase 6 this session" directive (measure → fix → stress → audit). The one scope addition (the write-path span cap) is a direct, in-scope hardening response to the cross-model Critical, not creep.
