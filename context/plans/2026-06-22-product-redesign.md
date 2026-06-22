# Usage OS — Product Redesign: Dial + Recap + Local AI

**Task:** TBD (assign in workflow) · **Created:** 2026-06-22 · **Status:** 🟡 In progress — Phase 0 (feasibility audit complete → `context/feasibility/2026-06-22-feasibility-audit.md`, verdict **GO-WITH-CAVEATS**; native spike next)

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

- [x] **Ecosystem research + whole-project feasibility audit** — wrote `context/standards/{rust,tauri-ipc,capture-and-permissions,testing-and-ci,foundation-models}.md` + `context/feasibility/2026-06-22-feasibility-audit.md` (verdict **GO-WITH-CAVEATS**; risk register R1–R83; spike plan §4). Desk-grounded from current sources; **native/version claims provisional until the spike confirms them.**
- [ ] **Native capture spike (isolated)** — prove AX titles + Automation URLs + NSWorkspace events across Cursor/iTerm/Chrome/Safari/Brave on the real Mac. _Make-or-break; nothing else matters if it fails._ **Spike order (audit §4):** ① AX titles with Screen Recording OFF across all apps (R2–R4, R7) → ② run-loop/threading model + NSWorkspace + AXObserver (R6, R8–R13) → ③ browser URL + incognito exclusion (R15–R21) → ④ `proc_pidinfo` cwd read, non-root/unsandboxed (R22, R24). **Stabilize the dev-build signing identity FIRST (R14)** or every result is suspect.
- [ ] **Project-inference spike** — measure precision/recall + false-positive rate on the dev's real data; abstain threshold for low-confidence (R23, R26, R27). Depends on spikes ① and ③.
- [ ] **Hard-rule-3 cleanup (prerequisite for the `clippy -D warnings` gate)** — convert production `.expect()` to typed `Result`: `lib.rs` (×4), `watcher.rs`, `db.rs` (R82). Small separate PR.
- [ ] **Remove the unused `recharts` dependency** from `package.json` — honor "no chart library" (R77 / audit §M). Small separate PR.
- [ ] **Wire the missing merge gates** into `ci.yml` (hard rule 8): `clippy -D warnings`, `cargo fmt --check`, standalone `tsc`, and the tauri-specta binding-freshness check — on the right OS legs (R51, R82).
- [ ] **Wire tauri-specta** into the existing app — exact-pin the RC trio (D27); migrate `get_watcher_status` (returns `serde_json::Value` → named struct) + the 11 `Result<_, String>` commands → a `thiserror` `AppError` (R48); `BigIntExportBehavior::Number` for i64 timestamps (R49); commands-only first (#211, R50).
- [ ] **Design system (parallel track):** full Bauhaus system in Claude Design (needs `/design-login`), both themes, all states; reconcile `context/design-system.md`. UI build blocked until locked. **Blocker the audit found:** the Comms yellow `#F2BC0C` fails WCAG non-text contrast (1.47:1) and color is the only context channel — fix the palette (or add arc outlines + a guaranteed non-color cue) and re-run contrast in both themes before locking (R77).

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
