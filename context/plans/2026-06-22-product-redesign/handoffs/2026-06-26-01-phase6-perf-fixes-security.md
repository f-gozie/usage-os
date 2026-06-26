# Handoff — 2026-06-26-01 · Phase 6 (perf + stress + security) done in one session: read-path **fixed** (~33–35×), write-churn measured, deps audited

**Supersedes handoff 09's framing.** Handoff `2026-06-25-09` recorded session-1 as measure-only with the fix "next session." The owner then directed *do all of Phase 6 this session* — so the fix, the write-path stress, the memory check, and the security/dependency audit all landed on **`phase6/perf-harness`** too. **All gates green. PR pending.**

Full results: `explorations/2026-06-25-perf-baseline.md`. Impl-plan: `impl-plans/2026-06-25-phase6-perf-harness.md`.

## What landed (one branch, off merged `main`)
1. **Perf/stress harness** behind a `perf` cargo feature (never ships): deterministic synthetic-history generator (`src-tauri/src/perf.rs` → repo-layer `db::bulk_insert_events`), `#[ignore]` read-timing + write-churn tests, a `seed_db` bin for an on-disk DB.
2. **Read path — diagnosed + FIXED.** It was O(total history) (the rollup build is flat ~1.2 ms — never the bottleneck); `get_activity_logs`'s `start_time < ?end` (only `idx_start_time`) scanned ~the whole table for a recent day. **Fix:** lower-bound the scan at `start − MAX_SPAN_LOOKBACK_SECS` (2 days) → bounded range scan, O(window). One change in `db/events.rs`, **no migration, planner-independent**, regression test added.
   - **Before → after @ 1.05M rows: `get_day` 52.97 → 1.59 ms (~33×), `get_week` 370.9 → 10.5 ms (~35×); now flat across all scales.** Every read is well under the proposed budgets at 1M+ rows, so the "single Week query"/"shared fetch" ideas are unneeded.
3. **Write path — churn + lock contention measured.** 45,000 writes/sec solo. Under a *tight-loop* concurrent reader the writer slows ~16× (a single `Arc<Mutex<Connection>>` serializes reads/writes), but real interaction rates never approach that → **R57 (dedicated writer thread) correctly deferred, with data**; the churn harness is its trigger.
4. **Memory (2.13 GB)** — live dev app RSS sampled at **~170 MB** (main 90 + WebKit 82), matching the earlier healthy range; combined with the tiny Rust read footprint, **strong evidence it's transient/system-pressure, not a product leak.** (Aside: this session's repeated release builds filled the disk — `ENOSPC` — a plausible source of the original "system-wide pressure"; freed by removing rebuildable `target/release` + `target/llvm-cov-target`.)
5. **Security / dependency audit** — `src-tauri/deny.toml` is now a committed gate (macOS-scoped). **2 real vulnerabilities fixed** (`bytes` 1.11.0→1.11.1 integer overflow; `time` 0.3.44→0.3.47 RFC-2822 DoS), **our crate now declares `license = "MIT"`** (was undeclared → cargo-deny flagged it), licenses all permissive/allow-listed, sources clean, unmaintained transitives (rust-unic/fxhash/mach/paste — no upstream fix) documented-ignored. No-network reaffirmed (`reqwest`/`hyper` resolved but **not in the build graph**). `cargo deny check` → all ok.
6. **Launch fix** — adding the `seed_db` bin made this a multi-binary package, which broke `tauri dev`'s bare `cargo run` (the user hit this live). Fixed with `default-run = "usage-os"` in `Cargo.toml`.

7. **Reviewed (`/usageos-review`) — clean, one Critical caught + resolved.** The Codex cross-model lane flagged that the read-path lower bound *narrows the overlap contract* (a span older than the lookback but still overlapping is dropped) and the write path only *assumed* that can't happen. **Resolved by enforcing it:** `capture::on_tick` now caps span length at `MAX_OPEN_SPAN_SECS` (12 h « the 2-day lookback) by splitting marathon spans into contiguous same-window pieces (read-time segmentation re-coalesces them — invisible in the UI). The bounded scan is now complete by construction. Plus the review removed dead code (`impl Default for SeedConfig`) and a rot-prone doc slug. Report: `reviews/2026-06-25-phase6-perf-harness.md`.

## Gates (all green)
`cargo fmt --check` · `cargo clippy --all-targets -D warnings` (default **and** `--features perf`) · `cargo test` **127 passed / 1 ignored** · `tsc` · `vitest` 32 · binding-freshness (no `bindings.ts` change) · **`cargo deny check` clean**.

## NEXT
- **Owner: ratify the perf budgets** (`get_day`<50, `timeline`/`recap`<100, `week`<150 ms) → write the ADR in `decisions.md`. They now pass with ~30–90× headroom at 1M rows.
- **Open the PR** for `phase6/perf-harness` (consider `/usageos-review` first). After merge, fork the next branch from `main`.
- **Optional/confirmatory:** a release soak (heavy DB + long Timeline navigation, RSS monitor) to definitively close the 2.13 GB question — `seed_db --months 24` makes the DB. Run when the dev server is stopped (the release build contends + the soak wants the GUI).
- **Smaller deferred hardening:** native `osascript` off the main run loop; migration `CHECK` constraints; idle-CPU profiling.

## Files touched (summary)
`src-tauri/Cargo.toml` (feature, bin, `default-run`, `license`), `Cargo.lock` (bytes/time bumps), `src-tauri/deny.toml` (new), `src-tauri/src/perf.rs` (new), `src-tauri/src/bin/seed_db.rs` (new), `src-tauri/src/db/events.rs` (read-path fix + `bulk_insert_events`), `src-tauri/src/db/mod.rs` (regression test), `src-tauri/src/capture/mod.rs` (`WriteProbe` + the `MAX_OPEN_SPAN_SECS` span cap from review), `src-tauri/src/lib.rs` (perf module decl). Docs: the findings doc, this handoff, the impl-plan, `plan.md`.

## Open / to confirm
- Budgets proposed, **not yet ratified** (no ADR yet).
- The 2 pre-existing untracked items (`reviews/…full-codebase-audit.claude-findings.json`, `design/logo/`) remain untracked.
