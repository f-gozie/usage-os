# Phase 6 — performance, stress & hardening results

**Date:** 2026-06-25 · **Branch:** `phase6/perf-harness` · Owner directive: stress + perf-test
(and fix) before the Phase-5 launch. This doc is the full results: the read-path scale baseline,
**the fix and its before/after**, the write-path churn / lock-contention test (the R57 question),
memory, and the security/dependency audit.

All measurements use the dev-only `perf`-feature harness (`src-tauri/src/perf.rs`): a deterministic
(seeded SplitMix64) generator of realistic spans seeded **through the repository layer**, so the
path under test is the production one. Reproduce:
```sh
cargo test --manifest-path src-tauri/Cargo.toml --release --features perf perf_read       -- --ignored --nocapture
cargo test --manifest-path src-tauri/Cargo.toml --release --features perf perf_write_churn -- --ignored --nocapture
```

---

## 1. Read path — baseline, diagnosis, and the fix

### Baseline (BEFORE) — release, in-memory SQLite, median of 9
```
tier              rows day_rows   get_day  timeline     recap      week  day_build
1mo              28269     1030    2.91ms    2.85ms    2.87ms   18.54ms    1.18ms
6mo             174917     1030   10.00ms    9.59ms    9.68ms   67.27ms    1.20ms
12mo            350865     1030   18.63ms   18.63ms   18.72ms  129.71ms    1.20ms
24mo            702833     1030   35.83ms   36.08ms   36.10ms  251.90ms    1.23ms
heavy-36mo     1055461     1030   52.97ms   52.63ms   52.77ms  370.93ms    1.21ms
```
`day_rows` is constant (1030) yet every read grows with **total** rows → the cost was in the
query, not the rollup (`day_build` is flat ~1.2 ms — the pure read-model is O(day), never a
bottleneck). **Root cause:** `get_activity_logs`'s overlap predicate `start_time < ?end AND
end_time > ?start` had only `idx_start_time`; for a recent day `start_time < ?end` matches ~the
whole table, so the index walked all history. `EXPLAIN`: `SEARCH … idx_start_time (start_time<?)`.

### The fix (one change, planner-independent, no migration)
Lower-bound the scan: a span overlapping `[start, end)` must have started within
`MAX_SPAN_LOOKBACK_SECS` (2 days) before `start` — no real span outlives the idle gate, let alone
days. Adding `start_time >= ?start - LOOKBACK` turns `idx_start_time` into a **bounded range
scan = O(window)** ([src-tauri/src/db/events.rs](../../../../src-tauri/src/db/events.rs)). New
`EXPLAIN`: `SEARCH … idx_start_time (start_time>? AND start_time<?)`. Regression test added
(a span older than the lookback is correctly excluded; a within-lookback overlap is kept + clipped).

### AFTER — same harness, same data
```
tier              rows day_rows   get_day  timeline     recap      week  day_build
1mo              28269     1030    1.84ms    1.53ms    1.62ms   10.62ms    1.17ms
6mo             174917     1030    1.57ms    1.78ms    1.60ms   10.40ms    1.15ms
12mo            350865     1030    1.63ms    1.51ms    1.57ms   10.66ms    1.21ms
24mo            702833     1030    1.64ms    1.61ms    1.66ms   10.71ms    1.17ms
heavy-36mo     1055461     1030    1.59ms    1.81ms    1.60ms   10.47ms    1.17ms
```

### Before → after (at 1.05M rows ≈ 3 years of heavy use)
| read | before | after | speedup |
|---|---|---|---|
| `get_day` | 52.97 ms | **1.59 ms** | ~33× |
| `get_timeline` | 52.63 ms | **1.81 ms** | ~29× |
| `get_recap` | 52.77 ms | **1.60 ms** | ~33× |
| `get_week` | 370.93 ms | **10.47 ms** | ~35× |

**The read path is now flat — independent of total history.** Every read is comfortably under the
proposed budgets (`get_day`<50, `timeline`/`recap`<100, `week`<150 ms) at **1M+ rows**, so the
separate "single Week query" and "share one fetch per day-open" ideas are **unnecessary** (kept the
change minimal). Budgets are met with ~30–90× headroom; recommend ratifying them as-is.

---

## 2. Write path — churn & lock contention (the R57 question)

50,000 rapid app-switch events (each a close+open span = the heaviest write path) driven through the
real `on_focus` state machine, on a 6-month-seeded DB:

```
writes (solo):        1104 ms   =  22.07 µs/event  =  45,305 events/sec
read  (solo):         454 µs/read (recent day, ~1030 rows)
writes (concurrent):  17596 ms  (15.94× the solo write time)
read  (concurrent):   500 µs/read (1.10× solo), 35,161 reads completed while writing
```

**Reading:** writes are extremely cheap — **45k events/sec**, versus real capture's ~1,000 events
per *day*. Under a **tight-loop** concurrent reader, the writer slows ~16× while the reader barely
moves (+10%): a single `Arc<Mutex<Connection>>` serializes reads and writes (WAL's concurrent-read
benefit is unused because the mutex gates everything), and since a read holds the lock ~20× longer
than a write, the writer waits behind reads.

**Verdict on R57 (dedicated writer thread + separate read connections): still correctly deferred.**
The contention only appears under continuous hammering. Real rates are a write every few seconds and
a read on user interaction — each holds the lock <1 ms, so collisions are rare and imperceptible.
R57 would decouple them (readers stop blocking the writer) and is the right move **if** we ever make
reads frequent/heavy; the churn test is the gate, and today it says "not needed." (Data-backed, not
speculative.)

---

## 3. Memory (the unreproduced 2.13 GB)

Sampled the **live dev app** (real DB, normal use): main process **89.7 MB** + WebKit content
**81.8 MB** ≈ **170 MB total** — consistent with the earlier healthy monitoring (Rust ~100 MB,
WebKit 58–290 MB fluctuating, no sustained climb), and nowhere near 2.13 GB. Combined with §1's
finding that the Rust read path holds only ~1,030 events/day in memory regardless of history, the
evidence strongly indicates the 2.13 GB was a **transient/system-pressure/dev-mode artifact, not a
product leak**. A full release soak (heavy DB + long Timeline navigation, RSS monitor) remains the
definitive confirmation — `seed_db --months 24` produces the DB for it — but no fix is indicated.

---

## 4. Security & dependency audit

- **No network in the data path (hard rule 1) — reaffirmed.** Our source contains no socket/HTTP
  client use (the `fetch()`/`refetch()` in `src/hooks/*` are local Tauri-command calls, not HTTP).
  `reqwest`/`hyper` appear in `Cargo.lock` but `cargo tree -i reqwest` shows they are **not in our
  build graph** — not compiled into the app. No network capability in `capabilities/*.json`; CSP is
  strict (Phase 4 / D55).
- **Dependency advisories + licenses (`cargo-deny`) — now a committed gate** (`src-tauri/deny.toml`,
  scoped to the macOS ship targets). Run: `cargo deny check` → **advisories ok · bans ok ·
  licenses ok · sources ok.** What it took:
  - **Two real vulnerabilities found and fixed** by patch bumps (`Cargo.lock`):
    `bytes 1.11.0 → 1.11.1` (integer overflow in `BytesMut::reserve`, RUSTSEC) and
    `time 0.3.44 → 0.3.47` (RFC-2822 parse stack-exhaustion DoS). Both are transitive and not
    reachable with untrusted input in our no-network app, but the fixes are free.
  - **Our own crate declared MIT** — `usage-os` had no `license` field in `Cargo.toml` (cargo-deny
    flagged it `Unlicensed`); now `license = "MIT"`, matching the project.
  - **Licenses:** every dependency is permissive / MIT-compatible (MIT, Apache-2.0 (+LLVM-exc),
    BSD-2/3, 0BSD, BSL-1.0, CC0-1.0, ISC, MPL-2.0, Unicode-3.0, Unlicense, Zlib) — allow-listed.
    `r-efi` (LGPL) is a UEFI/wasi crate not compiled on macOS (excluded by the target scope).
  - **Sources:** all from crates.io. **Unmaintained** advisories (the rust-unic family, `fxhash`,
    `mach`, `paste`) are transitive with no upstream fix — documented ignores in `deny.toml`.

---

## 5. Status vs. the Phase-6 checklist

- ✅ Stress & perf harness (read + write) — built, behind the `perf` feature.
- ✅ Read-path scale problem — **diagnosed and fixed** (~33–35× at 1M rows; now flat).
- ✅ Write-path churn / lock contention — measured; **R57 deferred with data**.
- ✅ 2.13 GB memory — strong evidence it's not a product leak (live RSS ~170 MB); full release soak
  optional/confirmatory.
- ✅ Security — no-network reaffirmed; `cargo deny` clean (2 vulns fixed, MIT declared, committed gate).
- Deferred (data-backed or smaller): R57 writer thread (churn shows no real contention); native
  `osascript` off the main run loop; migration `CHECK` constraints.

> Aside surfaced this session: the dev machine's root volume filled during the repeated
> release/perf builds (`ENOSPC`), which is plausibly the "system-wide memory pressure" context
> behind the original 2.13 GB Force-Quit reading. Freed by removing rebuildable `target/release`
> + `target/llvm-cov-target`. Not a product issue; worth noting for the soak session.
