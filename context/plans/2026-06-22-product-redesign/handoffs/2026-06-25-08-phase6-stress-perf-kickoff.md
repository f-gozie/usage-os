# Handoff — 2026-06-25-08 · Phase 4 shipped (PR #19); next up is **Phase 6 — stress & performance testing** (sequenced before Phase 5 launch)

**Where we are now:** Phase 4 (shell & onboarding) is **done, verified on-device, reviewed, and in review as [PR #19](https://github.com/f-gozie/usage-os/pull/19)** (branch `phase4/shell-onboarding`, base `main`). The promptless-skip behavior (D57) was re-verified on-device — works. See **handoff 07** for the full Phase-4 completion detail and **`reviews/2026-06-25-phase4-shell-onboarding.md`** for the review.

**Owner directive (2026-06-25): do Phase 6 before Phase 5.** We **harden + stress/performance-test** the app *before* we ship it. So the order is now **Phase 6 (optimization/perf/stability/security) → then Phase 5 (launch)**. `plan.md` is updated to reflect this (Phase 5 marked deferred-until-after-6; Phase 6 marked NOW NEXT with a new lead item).

## NEXT — Phase 6, led by **stress & performance testing**
The goal of the first session(s): **build a load/soak harness and find where the app bends before real users do** — produce numbers + a ranked bottleneck list that drives the rest of Phase 6. **Don't build fixes before reproducing/measuring** (especially the memory item — see below). Keep it simple; no premature optimization.

Four workstreams to design tests for:
1. **Scale (large synthetic history).** Seed many months → millions of `activity_logs` (+ categories/rules/sites/projects), then measure the read path at scale: `get_day` / `get_week` / `get_timeline` / `rollup` cost, and webview render of very long Timelines/days. Where does the dial/week/timeline start to lag? _(A seeding script is step one — a Rust test-util or a SQL generator against the repo layer.)_
2. **Soak (long-running stability + memory).** Run a **release** build (`tauri build`, not `tauri dev`) for a long session with an RSS/CPU monitor. This is also how we settle the **unreproduced 2.13 GB memory** observation: flat RSS ⇒ dev-mode bloat (no product bug); sustained climb ⇒ real → take a WebKit heap snapshot to name what's retained before choosing any fix (maybe Timeline virtualization, maybe not).
3. **Capture churn.** Rapid app/window switching, idle→resume, multi-display, agent-watching (PATIENT_GATE) — stress the single-writer consumer thread and the `Arc<Mutex<Connection>>` write path. This is where **R57** (a dedicated WAL writer thread + separate read connections) earns its keep if the lock contends; today's `Arc<Mutex<Connection>>` is the documented interim.
4. **Recap/sidecar.** Repeated `get_recap` spawns (one-shot Swift sidecar) under load — spawn cost, timeout behavior, cache hit-rate (D52 fingerprint cache).

### Deferred items that fold into Phase 6 (already in `plan.md`)
- **R57** — WAL + dedicated writer thread (perf-gated; do it if churn testing shows lock contention, not speculatively).
- **`cargo-deny`** dependency audit (the one remaining piece of the D53 security pass).
- Native **`osascript`/`pgrep` off the main run loop** + producer full-identity dedupe (on-device session — D32/D33).
- **Idle-CPU** profiling (pulled from Phase 4; glance/permissions poll on focus, not a timer — confirm cost).
- Migration `CHECK` constraints (needs a table-rebuild migration).

## Architecture notes for the perf work (so the next session doesn't re-derive them)
- **Write path:** capture producer (main run loop, D29) → `std::mpsc` → a **single consumer/writer thread** (`capture::consume`) that owns the open span and self-ticks via `recv_timeout` (TICK=20s / GATE=120s, PATIENT=600s for agent/dev apps — D38/D39). DB is `Arc<Mutex<Connection>>`, `foreign_keys` on, **no WAL yet** (R57).
- **Read path:** pure `rollup` read-model → `get_day`/`get_week`/`get_timeline`; numbers computed in Rust (hard rule 6). Recap is lazy/after-the-dial (D11), cached by content fingerprint (D52).
- All SQL is in the repository layer (`db.rs`/`migrations.rs`) — hard rule 4 — so a seeding util goes through (or beside) that, not raw SQL in handlers.

## Open / to confirm
- **Merge PR #19** (Phase 4) — once green/approved. After merge, the next branch (e.g. `phase6/perf-harness`) forks from `main`.
- The 2 pre-existing untracked items (`reviews/2026-06-25-full-codebase-audit.claude-findings.json`, `design/logo/`) remain untracked — not part of any branch.
- Phase-6 perf **targets/budgets are TBD** — the first session sets them (what "fast enough" means for a multi-month history, an acceptable RSS ceiling, etc.).
