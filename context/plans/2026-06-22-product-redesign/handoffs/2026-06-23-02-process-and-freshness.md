# Handoff — process restructure + docs-freshness mechanism

_Written 2026-06-23 at the end of the **process session** (same day as, and after, [`2026-06-23-01-phase1-capture`](2026-06-23-01-phase1-capture.md)). Read `CLAUDE.md` first, then this. **Project/code state is UNCHANGED this session** — for the full engineering state read the prior handoff; this one covers only the workflow changes._

## 1. Where we are (TL;DR — project state, unchanged)

UsageOS (private on-device Mac time-tracker; Tauri + Rust + React; Bauhaus redesign). **Phase 0 ✅ and all of Phase 1's backend capture pipeline ✅** — built, CI-green, **on-device verified** (real titles incl. Electron, browser URLs, incognito-safe, terminal→project via D30 canonicalization). **Everything remaining in Phase 1 is UI** (the dial + settings), **gated on the design system** — now the critical-path blocker. `main` is clean, no open PRs. 58 Rust + 28 TS tests, CI green (Ubuntu + macOS). **Full detail: [`2026-06-23-01-phase1-capture`](2026-06-23-01-phase1-capture.md).**

## 2. What changed this session (process only — no code)

Two commits on `main` (`7a8e1ea`, plus this handoff's commit):

**A. Plans + handoffs restructured into per-plan, append-only history** (was: one overwritten `context/handoff.md`):
```
context/plans/
  README.md                                  ← registry (plans + status) = the entry point
  2026-03-22-tier1-oss-hygiene/plan.md       (done)
  2026-06-22-product-redesign/               (active)
    plan.md                                  ← living roadmap (check off / annotate)
    handoffs/  YYYY-MM-DD-NN-slug.md          ← append-only, one per session, NEVER overwrite
    impl-plans/  YYYY-MM-DD-<task>.md         ← approved plan-mode plans (as-built)
context/handoff.md                           ← static redirect breadcrumb → registry
context/decisions.md                         ← unchanged (append-only ADRs, D1–D33)
```
- The full handoff chain (5 sessions: vision → phase0-spikes → spike-2 → native-spikes-ipc → phase1-capture) was **reconstructed from git** and lives in `handoffs/`.
- The 3 Phase-1 impl-plans are in `impl-plans/` (1.2b verbatim; 1.1/1.2a as-built from PR bodies).

**B. Docs-freshness mechanism (anti-drift)** — codified in `CLAUDE.md`:
- **Definition of Done** (Dev workflow): before any PR and before `/handoff`, reconcile **against the actual diff** — tick/annotate `plan.md`, append `decisions.md`, write a new `handoffs/` entry. A PR isn't "done" until the docs read true against the current code. **This is the primary guarantee** (it works because CLAUDE.md is always loaded).
- **Backstop:** `.githooks/pre-push` — a **non-blocking** tripwire that reminds if a push changes `src/`/`src-tauri/src/` without touching `context/plans/` or `decisions.md`. Verified both ways (fires on code-only, quiet when docs updated alongside).

## 3. Active workflow rules (READ — these now govern every session)

- **Start of session:** `CLAUDE.md` → `context/plans/README.md` (pick the `active` plan) → its `plan.md` + the **newest** `handoffs/` file.
- **As you work / at PR time:** keep `plan.md` checkboxes + `decisions.md` current with what actually landed (Definition of Done).
- **End of session / `/handoff`:** write a **NEW** numbered handoff in the active plan's `handoffs/` — never overwrite.
- **Approved plan-mode plan** → save to `impl-plans/`.

## 4. Gotchas / setup

- **`git config core.hooksPath .githooks`** is set on this clone; a fresh clone must re-run it for the pre-push reminder to work. (It's non-blocking either way.)
- The reconstructed handoffs reference *old* paths (e.g. the flat `…product-redesign.md`) — that's intentional: they're **immutable history**, true as of when written. Don't "fix" them.
- Everything else from the prior handoff's §8 still holds (dev-TCC responsibility attribution, Windows-dropped-from-CI, objc2 0.6.4/0.3.2 + specta =rc.20 pins, `gh pr edit` broken → use `gh api`, zsh no word-splitting).

## 5. Next steps (unchanged from the prior handoff)

1. **Design system** (now the critical-path blocker for the rest of Phase 1) — full Bauhaus, both themes, all states; **fix R77** (Comms-yellow contrast); the Claude Design `/design-login` push is the user's (not available in CLI). Then:
2. **Phase 1.4 — the dial** from the now-real capture data (the product's soul, D3).
3. **Phase 1.3 UI tail** — settings to manage exclusions / per-app Private (backend already done).
- Off critical path: deferred capture refinements (heartbeat timer for idle, Terminal.app cwd).
- External, start early: **Apple Developer enrollment** (gates Phase 3 + 5 + real TCC/signing).

**Read order:** `CLAUDE.md` → `context/plans/README.md` → active plan's `plan.md` + newest handoff → `context/decisions.md` (D1–D33) → `context/design-system.md` / `context/standards/*` as needed.
