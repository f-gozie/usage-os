# Handoff — 2026-06-25-04 · Recap branch reviewed + merged to main; Phase 3 closed

**Where we are now:** the recap sidecar is **merged to `main`**. PR [#17](https://github.com/f-gozie/usage-os/pull/17) squash-merged as `c58767e`, the `phase3/recap-sidecar-impl` branch is deleted, and `main` is in sync with origin. **Phase 3 (the recap) is complete** — Foundation Models sidecar (D51) + persisted content-fingerprint cache (D52), with the always-on deterministic template floor. Only the deferred evening "your day is ready" ping remains in Phase 3.

## What happened this session
1. **Ran `/usageos-review` against the recap branch** (`branch` scope, `main...HEAD`). Panel: Claude Lanes A/B/C + an independent Codex cross-model lane; every Critical/Warning verified against real code (incl. reading the pinned `tauri-plugin-shell-2.2.1` source). Report: `reviews/2026-06-24-phase3-recap-sidecar.md` (paired to the impl-plan).
   - **Gates all green:** 123 Rust + 30 TS tests, `clippy -D warnings`, `cargo fmt`, `tsc`, `vitest`, bindings fresh.
   - **Verdict: 0 Critical, 0 hard-rule violations.** Re-verified the linchpins: `get_recap` drops the DB lock *before* the `.await` (no `MutexGuard` across await; clippy's `await_holding_lock` agrees), `get_recap` registered in `collect_commands!`, sidecar has empty entitlements / no network, `RecapProse` carries no numeric field (model phrases, never counts), template fallback intact on every non-`ok` status.
2. **Fixed the two Warnings pre-merge** (commit `b423d03`, gates re-run green):
   - **W1 — `useRecap` stale-response race:** added a monotonic request-id latch (mirrors `useViewData`). `DayView` is mounted unkeyed, so a slow prior-day narration could resolve late and paint the previous day's prose over the current day. Closed.
   - **W2 — wedged sidecar not killed on timeout:** verified against the plugin source that dropping `CommandChild` only closes stdin (EOF), which a child mid-inference ignores. Hoisted spawn+write out of the timed `read` future and added `child.kill()` on the timeout arm. (Also corrected Codex's Critical, whose "stdin is never closed" mechanism was wrong — the happy path was already clean.)
3. **Merged + reconciled docs.** Squash-merged #17; updated `plan.md` (Phase 3 ✅, CSP note now "unblocked on main"); wrote this handoff. Decisions D51/D52 were already in `decisions.md` from the branch.

## Info-level follow-ups (optional, non-blocking — from the review)
- `ai/sidecar.rs`: per-chunk `String::from_utf8_lossy` could corrupt a multi-byte char split across stdout pipe chunks (low probability; byte-accumulate then decode the whole line to fix).
- `useRecap.ts:9`: `refetch` typed `() => void` but is `async` (harmless fire-and-forget).
- The D52 cache rationale is narrated in three places (migration SQL + `recap.rs` docs + `get_recap` doc) — trim if touched.
- `DayView` narrates a recap even when `getDay` errors — harmless (content-addressed cache), optional micro-opt.

## Next steps (from the plan — Phase 3 done, so Phase 4 is up)
1. **Phase 4 — shell & polish** (the plan's stated next, and the gate to real install/dogfood):
   - Menubar launcher + window; **primed onboarding + permission priming** (Accessibility/Automation; run degraded if declined). This is the biggest unlock — capture needs these grants on a real install.
   - Dark-mode parity (designed); idle-CPU performance pass.
   - Day-start offset (night-owl "day starts at 4 AM", D14) — deferred-later.
2. **Small unblocked wins (can slot in anytime):**
   - **CSP** (`tauri.conf.json` still `csp: null`) — now unblocked on `main`; set strict `default-src 'self'` and verify no white-screen.
   - **`CategoryEditorModal` component tests** — the under-tested file where the D53 audit's near-misses clustered.
   - **`cargo-deny`** dependency audit (Phase 6 security).
3. **On-device session items (need Favour's Mac — D53 deferred):**
   - Move `osascript`/`pgrep` capture enrichment off the main run loop + producer full-identity dedupe (blind run-loop edits risk a UAF — D32/D33).
   - Owner call: the Chromium keep-title-when-private-unprovable posture (D33/R18).
4. **Phase 6 hardening (perf-gated, not speculative):** `Arc<Mutex<Connection>>` → R57 (dedicated WAL writer thread); migration `CHECK` constraints (needs a table-rebuild migration); **diagnose the 2.13 GB memory observation** (release-build long-run + heap snapshot before any fix).
5. **Phase 5 — launch:** notarized DMG + auto-update + Homebrew cask; README rewrite; finalize name/domain.

## Gotchas / housekeeping
- Stale local branches remain: `phase6/audit-fixes` (merged via #18) and `spike/foundation-models` (the spike) — safe to delete (`git branch -d`).
- The audit intermediate `reviews/*.claude-findings.json` (580 KB) is untracked and should stay uncommitted (it's a local artifact; not currently in `.gitignore`, so don't `git add -A` blindly).
- Migrations self-heal in dev since D54 — switching branches prints `rebaselining`/`ignoring`, not an error.
