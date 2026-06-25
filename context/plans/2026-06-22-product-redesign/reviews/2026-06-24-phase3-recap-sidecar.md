# Review — Phase 3 recap sidecar (Foundation Models) + recap cache

**Date:** 2026-06-25 · **Scope:** `branch` (`git diff main...HEAD`, `phase3/recap-sidecar-impl`) · **Files:** 37 (≈1,199 insertions; ~16 source files)
**Plan:** [plan.md](../plan.md) · **Impl-plan:** [2026-06-24-phase3-recap-sidecar.md](../impl-plans/2026-06-24-phase3-recap-sidecar.md)
**Codex:** ran (`codex exec`, read-only, output-schema) — 2 findings, both verified + one mechanism refuted below.

> Scope note: `main...HEAD` is anchored at the post-audit `main` (merge-base `28c10d6`), so this review covers **only** the recap-sidecar feature this branch adds on top of main (D51 sidecar + D52 cache + the merge resolutions) — the audit fixes (D53/D54) were reviewed under `2026-06-25-full-codebase-audit.md` and are excluded.

## Merge gates
| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy --all-targets --all-features -D warnings | ✅ |
| cargo test | ✅ 123 passed (1 ignored) |
| tsc --noEmit | ✅ |
| vitest | ✅ 30 passed (10 files) |
| bindings fresh (`export_bindings` + `git diff --exit-code src/bindings.ts`) | ✅ |

All gates green. `cargo clippy -D warnings` passing is itself evidence for the most important invariant here: no `std` `MutexGuard` is held across an `.await` (the `await_holding_lock` lint would deny it).

## Findings
**Verification:** 6 verified · 2 dropped/downgraded · 2 cross-model (both Codex findings overlapped a Lane-B finding; Codex's Critical *mechanism* was refuted on verification, see W2)

### Critical (must fix before merge)
None. No hard-rule violation; no correctness defect that breaks the feature.

### Warnings (should fix)

- **`src/hooks/useRecap.ts:24-36` — [Lane B / Codex, cross-model] stale-response race: a slow prior-day recap can render under the new day.**
  `fetchRecap` does `setRecap(await getRecap(...))` with **no request-id latch**. `getRecap` latency is highly variable — a cache hit returns instantly, a miss can take up to the 20 s model timeout. On fast day-stepping (Day A misses → step to Day B), Day A's slow call can resolve *after* Day B's and overwrite state. `DayView` is mounted **without a `key`** ([App.tsx:42](../../../../src/App.tsx)), so the `useRecap` instance persists across date changes and the same `setRecap` receives both — `DayView` then renders `(aiRecap ?? data.recap)` ([DayView.tsx:114](../../../../src/views/DayView.tsx)), i.e. **Day A's prose over Day B's dial/ledger**. The sibling [`useViewData.ts:31-49`](../../../../src/hooks/useViewData.ts) already implements exactly this latch with a comment citing this precise hazard — `useRecap` is the one fetch hook that omits it. *Verified: latch present in useViewData, absent in useRecap; DayView unkeyed.*
  **Fix:** add the monotonic `latest` ref pattern from `useViewData` — `const token = ++latest.current` before the await; only `setRecap(...)` if `token === latest.current`.

- **`src-tauri/src/ai/sidecar.rs:108-111` — [Lane B / Codex, cross-model] the per-call timeout abandons the wedged sidecar instead of killing it.**
  `CommandChild` (verified in the pinned `tauri-plugin-shell-2.2.1`, `process/mod.rs:65-87`) has **no `Drop` impl**: dropping it drops `stdin_writer` (a `PipeWriter` → closes the child's stdin → EOF) and decrements an `Arc<SharedChild>` refcount (the waiter thread holds another clone) — so a drop closes stdin but does **not** kill the process. On the **happy path this is clean**: the Swift sidecar's `while let line = readLine()` loop ([main.swift:160](../../../../sidecar/usageos-ai/Sources/usageos-ai/main.swift)) gets EOF and exits. But on the **timeout path** the child is mid-`await session.respond(...)`, not in `readLine`, so EOF doesn't interrupt it — the process keeps using the Neural Engine until inference finishes (it self-terminates then, via the EOF it already received). Worst case (a truly hung inference) it lingers. *Verified against the plugin source.*
  **Fix (optional hardening, not a leak-on-every-recap):** hoist the `CommandChild` out of the `read` future and `let _ = child.kill();` on the timeout arm so a wedged call frees the model immediately. Concurrency change → manual, re-run `cargo test` after.

### Info

- **`src-tauri/src/ai/sidecar.rs:87` — [Lane B] per-chunk `String::from_utf8_lossy` can corrupt a multi-byte char split across stdout pipe chunks.** Each `Stdout(bytes)` chunk is decoded independently before being appended to `buf`; a smart-quote/em-dash straddling a chunk boundary becomes `�`. Low probability (response lines are short; the JSON framing is ASCII so the protocol is unaffected — only prose content). Fix if touched: accumulate raw bytes in a `Vec<u8>`, split on `0x0A`, decode the complete line once.
- **`src/hooks/useRecap.ts:9` — [Lane C] `refetch: () => void` is typed sync but the impl is `async` (`Promise<void>`).** Harmless (callers fire-and-forget), mildly dishonest type. Leave as-is or type `() => Promise<void>`.
- **`src/views/DayView.tsx:36` — [Codex, downgraded] the recap narrates even when the day load (`useDayData`) errored.** `useRecap` runs unconditionally; if `getDay` fails, `ErrorState` renders and the prose is never shown. **Not a bug:** the cache is content-addressed by the facts fingerprint, so if `getRecap` computed valid facts the cached prose is correct and becomes an instant hit on retry; and since both share the DB, `getRecap` usually fails too → swallowed. Optional micro-optimization (gate the hook on loaded data), not correctness.
- **`db/recap.rs` doc + `0007_recap_cache.sql` comment + `lib.rs:121-125` `get_recap` doc — [Lane C] the same cache rationale is narrated in three places.** Each is individually a terse one-liner-plus-`(D52)` (the desired style), but collectively it restates D52's "content key = free invalidation / only model recaps cached" three times. Optional: trim the `recap.rs` fn docs to one line; keep the schema comment as the SQL's home.

### Dropped / downgraded
- **Codex Critical "sidecar stdin is never closed → child outlives the call → `rx.recv()` never returns `None`" — mechanism refuted.** Dropping `CommandChild` **does** close stdin (the `PipeWriter` field drops → EOF), and the happy path `return`s on the first stdout line without ever waiting for `None`. The valid kernel (timeout path doesn't proactively kill) is captured as **W2** at Warning, not Critical.
- **Lane C "`build_day_slice` re-implements the active-sum loop" — out of scope.** Pre-existing in `rollup.rs`, untouched by this diff.

## Fixes applied (this session, gates re-run green)
Both Warnings are concurrency/lifecycle changes (never machine-auto-fixed) — applied by hand, then the affected gates were re-run:
- [x] **W1 — `useRecap` request-id latch** ([useRecap.ts](../../../../src/hooks/useRecap.ts)): added the monotonic `latest` ref mirroring `useViewData`; a superseded narration no longer writes state. Re-ran `tsc` ✅ + `vitest RecapCard` ✅.
- [x] **W2 — `child.kill()` on the sidecar timeout arm** ([sidecar.rs:107-118](../../../../src-tauri/src/ai/sidecar.rs)): hoisted spawn+write out of the `read` future so the `CommandChild` survives into the timeout arm and a wedged inference is killed (not just sent EOF). Re-ran `cargo fmt`/`clippy -D warnings`/`cargo test ai::` ✅.

`cargo fmt` / `cargo clippy` were already clean before these — no machine-applicable auto-fixes existed.

## Manual TODO (optional, not blocking)
- [ ] I1 — byte-accumulate stdout reassembly in `sidecar.rs` (low-probability multi-byte split).
- [ ] I2 / I4 — `refetch` type-honesty + trim the thrice-narrated cache rationale (taste).

## Definition of Done
- [x] plan.md ticked for what landed — Phase 3 chunks A–D marked ✅ (D51), lazy `get_recap` ✅; reads true against the code.
- [x] decisions.md ADRs appended — D51 (sidecar end-to-end) + D52 (recap cache, via `/debate`).
- [x] impl-plan present (`2026-06-24-phase3-recap-sidecar.md`) with an as-built section · handoff `2026-06-25-03` records current state; a new handoff to follow at session end.
- [x] docs move with code — the diff touches `context/decisions.md` + `context/plans/**`, so the `.githooks/pre-push` tripwire would not fire.

## Plan compliance
Alignment: **good** — the diff is exactly Phase 3's recap-sidecar chunks B–D plus the D52 cache decided by the recorded `/debate`; no scope creep. The one IPC addition (`get_recap`) is registered and reflected in fresh `bindings.ts`. Hard rules hold: no network in the data path (empty sidecar entitlements; only on-device `FoundationModels`), generated IPC, no panics in prod paths, SQL confined to `db/**`, `Narrator` mockable behind the trait, and the model phrases pre-computed numbers only (`RecapProse` carries no numeric field; template fallback intact on every non-`ok` status).

## Verdict
**Ship-ready.** Zero Critical, zero hard-rule violations, all gates green. W1 (the `useRecap` stale-response race) and W2 (kill the wedged sidecar on timeout) are both fixed in this session with gates re-run green; only optional Info polish remains. The branch is ready to merge into `main`.
