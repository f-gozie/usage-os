# Usage OS — Product Redesign: Dial + Recap + Local AI

**Task:** TBD (assign in workflow) · **Created:** 2026-06-22 · **Status:** 🔵 Planned

> Builds **on** the current `main` (v0.1.0 + the Tier-1 OSS-hygiene foundation). This is an evolution, not a restart. See `context/vision.md` (what/why) and `context/decisions.md` (D1–D25).

## Goal

Turn the existing tracker into a private, on-device **time mirror**: a fixed-24h **day-dial**, a daily **recap**, two-axis context × project, optional local AI — in a Bauhaus design. Cut gamification.

## What already exists (keep & build on)

From the shipped v0.1.0 + Tier-1 work, do **not** rebuild these:
- Tauri v2 shell, **rusqlite** + a versioned **migration system** (`schema_migrations`), settings table.
- Category **rules engine** (process/title matching, `ignore_title`), reprocess logs.
- **Tests** (24 Rust, 28 TS) and **GitHub Actions CI** (Linux/macOS/Windows), CONTRIBUTING, CHANGELOG, data **retention**.

## What changes (the redesign)

- **Capture:** `active_win_pos_rs` (app-name only; titles empty) → event-driven **AX titles + Automation URLs + NSWorkspace** events via `objc2`. _Still broken today — this is task #1._
- **Data model:** extend the migration chain (v5+) — add `projects`, `sites`, richer `contexts`, `recaps`, `embeddings`, `exclusions`, and columns on events (`url`, `site`, `project_id`, `is_private`).
- **UI:** the cyberpunk pie dashboard → the **Bauhaus day-dial + week + recap + linear timeline** (light & dark).
- **New deps:** `tauri-specta` (generated IPC), a thin **Swift Foundation Models sidecar** for the smart recap (template recap as fallback), on-device embeddings for categorization.

## Constraints

- Capture/AX/Automation can only be validated on macOS (Favour's machine). The CI/Linux box can build + run non-GUI Rust/TS tests but not the desktop capture path.
- Hard rules in `CLAUDE.md` are binding (no network in data path; generated IPC; no `unwrap` in prod; SQL only in the repo layer; design from the system).

## Phase 0 — De-risk & foundation

- [ ] **Native capture spike (isolated)** — prove AX titles + Automation URLs + NSWorkspace events across Cursor/iTerm/Chrome/Safari/Brave on the real Mac. _Make-or-break; nothing else matters if it fails._
- [ ] **Project-inference spike** — measure auto-tagging accuracy from titles/cwd/repos.
- [ ] **Ecosystem research** → write `context/standards/{rust,tauri-ipc,capture-and-permissions,testing-and-ci}.md`, grounded not from memory.
- [ ] **Wire tauri-specta** into the existing app (+ a binding-freshness check in CI alongside the current pipeline).
- [ ] **Design system (parallel track):** full Bauhaus system in Claude Design (needs `/design-login`), both themes, all states; reconcile `context/design-system.md`. UI build blocked until locked.

## Phase 1 — Capture → the dial ("I open it and see MY actual day")

- [ ] Replace the watcher with the event-driven capture impl (behind a `capture` trait, mockable).
- [ ] New migrations for projects/sites/contexts + event columns; repository functions + tests.
- [ ] Sensitive handling: exclusion list, per-app Private, incognito never recorded.
- [ ] Fixed-24h dial from real data, click-to-inspect. (Default contexts via the existing rules engine; projects auto-inferred.)

## Phase 2 — Enrichment

- [ ] Embedding-based categorization + corrections memory; contexts/rules editor in Settings.
- [ ] Week view (7 mini-dials) + linear timeline strip.

## Phase 3 — The recap

- [ ] `RecapFacts` computed in Rust; deterministic template recap first.
- [ ] Swift `usageos-ai` sidecar (Foundation Models, stdio, structured output) + availability check + fallback.
- [ ] Lazy compute on open; opt-in evening "your day is ready" ping.

## Phase 4 — Shell & polish

- [ ] Menubar launcher + window; primed onboarding + permission priming (run degraded if declined).
- [ ] Dark-mode parity (designed); performance pass (idle CPU).

## Phase 5 — Launch

- [ ] Notarized DMG + auto-update + Homebrew cask; rewrite README for the new product; Sponsor link; finalize name/domain.

## Open questions

- Project-inference accuracy (Phase 0 spike) may reshape the project axis.
- Foundation Models recap quality on a ~3B model — validate grounded prompts.
- How much of the existing cyberpunk UI / category model carries over vs is replaced.
