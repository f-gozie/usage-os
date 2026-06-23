# Handoff — start here for the next session

_Written 2026-06-22 at the end of the vision/planning session. Read this, then `CLAUDE.md`, then the redesign plan._

## Where we are

The product was **rethought end-to-end** and the plan is locked. This session produced the vision/decision/design docs + a clickable prototype. **No redesign code is written yet** — the next session starts Phase 0.

Crucially: this builds **on the existing, healthy `main`** — it is an evolution, not a restart.

- **Branch:** the redesign docs are on `main` (pushed). Build the redesign on a feature branch off `main`, following the existing PR workflow.
- **Already shipped on `main`** (v0.1.0 + Tier-1 OSS hygiene): the tracker + rules engine + dashboard, **rusqlite + versioned migrations**, **24 Rust + 28 TS tests**, **GitHub Actions CI** (Linux/macOS/Windows), data retention, CONTRIBUTING/CHANGELOG. Keep all of it.
- **Discarded:** only the uncommitted XP/goals/gamification WIP from the stale local clone. (That clone was ~10 commits behind origin — don't trust old local snapshots; always fetch.)
- **Not done:** README still describes the old cyberpunk/gamified product (Phase 5 rewrite). Name/domain not finalized.

## What's locked (don't relitigate without reason)

Read `context/decisions.md` (D1–D25). Shape: private on-device Mac time mirror; **pure observer** (not a coach); fixed-24h **day-dial** is the soul; two-axis **context × project**; capture via **Accessibility + Automation, never Screen Recording**; **Apple Foundation Models** via a thin Swift sidecar (template recap always as fallback); **Bauhaus** design (Anton + Jost, primary inks on paper, light+dark); Rust core (objc2) + tauri-specta IPC + the existing rusqlite/migrations; MIT, free, notarized direct distribution.

## The one thing that can sink this

**Window-title capture does not work today.** Origin's current `watcher.rs` still uses `active_win_pos_rs`, which falls back to CGWindowList (needs Screen Recording → titles come back empty; verified in the live DB: ~every app had blank titles). The entire product depends on getting titles + browser URLs. So **Phase 0's first task is an isolated native capture spike** — prove AX titles + Automation URLs + NSWorkspace events across Cursor/iTerm/Chrome/Safari/Brave on the real Mac. If it doesn't hold, stop and rethink before building.

## Start sequence (see `context/plans/2026-06-22-product-redesign.md`)

1. **Native capture spike** (isolated) — make-or-break.
2. **Project-inference spike** — accuracy from titles/cwd/repos.
3. **Ground the standards** — research Tauri v2 / objc2-AX / FoundationModels / tauri-specta, then write `context/standards/*`.
4. **Wire tauri-specta** into the existing app; add a binding-freshness check to the existing CI.
5. **Design track (parallel):** full Bauhaus system in Claude Design — needs `/design-login` — then reconcile `context/design-system.md`. UI build blocked until locked.

## Things the next session must know that aren't in the code

- The owner **steers architecture but doesn't write Rust** — the guardrails (typed IPC, clippy-as-error, the existing tests/CI, hard rules in `CLAUDE.md`) are the trust mechanism. Prefer boring, explicit, auditable code.
- This repo has an **active agent/PR workflow** (see `context/plans/2026-03-22-tier1-oss-hygiene.md` — task/workflow IDs, headless-Linux build box). Follow its conventions; assign a task/workflow id to the redesign plan.
- **Design must be fully done in Claude Design before any UI code** (owner's explicit ask — zero drift).
- rusqlite + migrations is what's already there (D18 just confirms it). Don't introduce sqlx/an ORM.
- Open logistics for the owner: rewrite `README.md`; finalize name/domain; the stale `claude/frontend-design-TwfKi` and my `claude/redesign-foundation` branches can be deleted.
