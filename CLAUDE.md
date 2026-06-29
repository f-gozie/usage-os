# UsageOS

A private, on-device app that records where your time goes and tells you the story of your day — a calm rear-view mirror, not a productivity coach. All data stays on the machine. _(Positioning is platform-agnostic — **macOS is the first platform, not the brand**, D60. v1 still ships macOS-only; see "Stack" and "What this product is NOT".)_

> **Working name:** UsageOS · **domain:** usageos.app (leaning) · **license:** MIT (public from day one)

This file is the contract every agent (and human) reads first. If something here conflicts with code, the code is wrong — fix the code, not this file. If this file is stale, fix this file.

---

## Read order for a new session

1. This file (`CLAUDE.md`) — rules + map.
2. `context/vision.md` — what we're building and why.
3. `context/decisions.md` — the locked decisions and their rationale (don't relitigate without reason).
4. `context/architecture.md` — system shape, layer boundaries, data model.
5. `context/plans/README.md` — the **plans registry**. Pick the `active` plan, then read its `plan.md` (roadmap) + the **newest file in its `handoffs/`** (current state). That latest handoff is the fastest "where are we now."
6. `context/design-system.md` — the visual contract (mirrored in the Claude Design project).
7. `context/standards/*` — detailed conventions (drafted in Phase 0 from grounded desk research; native/version claims are **provisional until the spike confirms them**).
8. `context/feasibility/` — the whole-project feasibility audit (verdict, the R1–R83 risk register, and the spike plan).

---

## Stack

- **Shell:** Tauri v2 (Rust backend + web frontend), macOS-first.
- **Backend:** Rust. Native macOS access via `objc2` (NSWorkspace activation events, Accessibility/AX window titles). Categorization is a deterministic rules engine in Rust — on-device embeddings were trialled and shelved (measured below the rules baseline, D47) — which keeps Swift = Foundation-Models-only (D26). SQLite via `rusqlite` behind a typed repository. _(WAL is enabled today; a dedicated writer thread is the remaining Phase-1 target — R57/D58; the connection is `Arc<Mutex<Connection>>`.)_
- **AI sidecar:** a small Swift helper binary — **Foundation Models only** (a Swift-only framework). Talks to Rust over stdio. This is the only Swift in the project.
- **IPC:** `tauri-specta` generates the TypeScript client + types from Rust commands. The frontend cannot call a command with the wrong shape.
- **Frontend:** React + TypeScript + Vite + Tailwind. Custom SVG for the dial (no chart library). Bauhaus design system.

**Already in place** (shipped v0.1.0 + Tier-1 hygiene — keep, don't rebuild): rusqlite + versioned migrations (`schema_migrations`), the v0.1.0 test baseline of ~25 Rust + 28 TS tests (the suite is much larger now; the TS suite is pure-logic, React Testing Library is net-new for the redesign UI), GitHub Actions CI (Linux + macOS; Windows dropped — macOS-only product, and the specta IPC stack won't link there), data retention, the category rules engine. **New in the redesign:** tauri-specta IPC, objc2 event-driven capture, the Swift Foundation Models sidecar, the Bauhaus dial UI. **We evolve this codebase — we do not restart it.**

## Hard rules (non-negotiable — drift here is a build failure, not a judgment call)

1. **Nothing leaves the machine.** No network calls in the data path, ever. This is auditable in the open source and is the product's whole promise. The only permitted network is an explicit, user-initiated update check.
2. **The IPC contract is generated, never hand-written.** All Rust↔TS types/clients come from `tauri-specta`. Editing generated bindings by hand is forbidden.
3. **No `unwrap()` / `expect()` / `panic!` in production paths.** Errors are typed and propagated (`Result`). Panics are for truly-impossible invariants only, with a comment proving it.
4. **All SQL lives in the repository layer.** No raw SQL or DB handles leak into command handlers or business logic. Repository functions are typed in, typed out.
5. **The native + AI surface stays minimal and isolated.** Capture lives behind a `capture` trait; the Swift AI sidecar behind an `ai` trait. Both must be mockable so the rest of the app is testable without macOS permissions or a model.
6. **The smart layer narrates, it never counts.** Recap models receive pre-computed aggregates and may only phrase them. Numbers are computed in Rust. A deterministic template recap is always available as fallback.
7. **Build against the design system, not vibes.** Tokens and components come from `context/design-system.md` / the Claude Design project. No ad-hoc colors, fonts, or spacing.
8. **Every merge is gated:** `cargo clippy -D warnings`, `cargo fmt --check`, `cargo test`, `tsc`, and a binding-freshness check must pass. Red = not merged.

## What this product is NOT

No gamification (XP, streaks, goals, "boss fights" — all cut). No real-time interruptions or nags. No cloud, no account, no telemetry. No Windows/Linux in v1. No Mac App Store (its sandbox forbids the Accessibility + Automation we require).

## Dev workflow

- Branch from `main`; never commit the working direction straight to `main` without review.
- **Plans, handoffs & decisions — three artifacts, three lifecycles (don't conflate them):**
  - A **plan** is a multi-session body of work, one folder under `context/plans/<date>-<slug>/`, registered in `context/plans/README.md`. It contains `plan.md` (the roadmap — the one *living* doc; check off / annotate as work lands, don't rewrite), `handoffs/`, and `impl-plans/`.
  - **Handoffs are append-only history.** At the **end of a plan session** (or `/handoff`), write a **new** file `handoffs/YYYY-MM-DD-NN-slug.md` — **never overwrite** an existing one. At the **start** of a session, read the active plan's `plan.md` + the **newest** handoff. The chain of handoffs is the project's narrative; preserving it is the point.
  - **Impl-plans:** when a task's plan-mode plan is approved, save it as `impl-plans/YYYY-MM-DD-<task>.md` (the as-built detail behind its PR).
  - **Decisions** go in `context/decisions.md` (ADR-style, append-only — D1…). The *why*; cross-reference from plans/handoffs.
  - Process docs (plans/handoffs/decisions/registry) are committed to `main` directly (they're cross-cutting, not feature code). Not every session maps to a plan — one-off tasks need none.
- **Definition of Done — docs move in lockstep with code (NO drift, NO staleness).** The plan/handoff/decisions are part of the change, not an afterthought. **Before opening or updating a PR, and before `/handoff`**, reconcile them against the **actual diff** (re-read what changed — not what you intended):
  1. **plan.md** — tick/annotate the active plan's checkboxes for exactly what landed; add newly-discovered items; note what's deferred. It must read true against the current code.
  2. **decisions.md** — append any new ADR (append-only).
  3. **handoff** — at session end / `/handoff`, write a **new** `handoffs/` entry for the current state + next steps (never overwrite).
  A PR/push is not "done" until these reflect the code as it now is. A **non-blocking `.githooks/pre-push`** tripwire reminds when a push changes `src/`/`src-tauri/src/` without touching `context/plans/` or `decisions.md` — a backstop, not a substitute for the reconciliation above. (One-time per clone: `git config core.hooksPath .githooks`.)
- Mockups before UI changes (the `mockup` skill). **Review every PR with `/usageos-review`** — the project's own review skill: it checks the diff against the hard rules, runs the real merge gates, gets an independent Codex cross-model second opinion, verifies findings against the code, and writes the report into the active plan's `reviews/` (paired to the impl-plan). It's the recommended pre-PR step, not a CI gate. (The generic `code-review` skill / `/code-review` is still fine for a quick diff-only pass.)
- Distribution: notarized DMG + auto-update + Homebrew cask. Needs the Apple Developer cert.
