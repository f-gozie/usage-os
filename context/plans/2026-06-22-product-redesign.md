# Usage OS — Product Redesign: Dial + Recap + Local AI

**Task:** TBD (assign in workflow) · **Created:** 2026-06-22 · **Status:** 🟡 In progress — Phase 0 (feasibility audit complete → `context/feasibility/2026-06-22-feasibility-audit.md`, verdict **GO-WITH-CAVEATS**; **native capture gate complete — spikes ①–④ all ✅ PASS** — leans **GO**; project-inference spike + tauri-specta wiring next)

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
- [~] **Native capture spike (isolated)** — on the real Mac. _Make-or-break._ **Spike order (audit §4):**
  - [x] ① **AX titles with Screen Recording OFF (R2–R4, R7) — ✅ PASS** ([PR #6](https://github.com/f-gozie/usage-os/pull/6), `spikes/ax-titles/`). Real focused-window titles for Chromium (Chrome, Brave), Electron editors/apps (Cursor, Claude, Notion, Figma, WhatsApp, Spotify), native (Finder), terminals (iTerm2) — Accessibility only, no Screen Recording. **R4 retired.** Side-findings: a run loop is required (confirms R6); system-wide `AXFocusedApplication` errors from a CLI (use per-pid); titles already carry project/page signal (helps D6) **and** sensitive content (D8 is load-bearing).
  - [x] ② **run-loop/threading model + NSWorkspace activation + AXObserver — ✅ PASS** (`spikes/ax-observer/`). The real event-driven architecture works end-to-end, Accessibility-only: the `NSWorkspace` activation **block** (`block2::RcBlock`) fires on every switch (R8/R9), a **per-PID `AXObserver`** delivers `AXFocusedWindowChanged` through the run-loop source and is rebuilt per app (R10), a dedupe debounce coalesces chatty titles (R11, observed `coalesced 3`), and every event marshals to a **Tokio** consumer over a `Send` channel with idle **0.0% CPU** (R6). Built + ran on **arm64** (R13). **Key findings:** (a) **`AXObserver` lives in `objc2-application-services`** — `accessibility-sys` not needed; (b) **no `NSApplication` required** — a bare main run loop delivers activation events (Tauri's loop is a superset); (c) **`AXTitleChanged` must be registered on the focused *window*** and re-pointed on window change. Residual manual check: pure in-place `AXTitleChanged` delivery (browser-navigation pass). R12 (idle detection) is out of scope (already shipped, orthogonal).
  - [x] ③ **browser URL + incognito exclusion — ✅ core PASS** (`spikes/browser-url/`). Chromium active-tab **URL reads** via Apple Events (Chrome, Brave; R15), and an **incognito window is excluded** because `mode of front window` is read **first** and any non-`"normal"` mode skips the URL — proven **live** (normal→incognito→normal; R17, D8 enforced). osascript latency **~140–160 ms** warm, fine for event-driven (R21); `NSAppleScript` not needed. Automation grants are per-(client,target) and persist (R19); the TCC client is the responsible process (the `.app` bundle in production, R20). **Residual manual checks:** Safari URL + private-detection (R16/R18 — Safari wasn't running; safe-default = never emit a Safari URL we can't prove non-private), Arc (R17 window/space model), and the `-1743` deny fallback (coded, not live-triggered). Zero-dep pure-std crate shelling `/usr/bin/osascript` (C7).
  - [x] ④ **`proc_pidinfo` cwd read — ✅ PASS** (`spikes/proc-cwd/`). A **non-root, unsandboxed** process reads another process's cwd via `proc_pidinfo(PROC_PIDVNODEPATHINFO)` — **no `EPERM`** (R22 was the kill-switch). Verified against interactive zsh shells it didn't spawn, matching `lsof` exactly (a shell in `…/projects/usage_os` → that path; `…/projects/nudge` → that). Hardened-runtime GUI apps (Cursor, Finder) read fine too — same-uid is the only gate; **no TCC grant needed**. So the terminal-cwd branch is viable and **R24's iTerm2-`path`/Terminal-tty fallbacks aren't needed for the common case**. Remaining = pid *selection* (front-tab shell pid via `proc_listchildpids` + tty/recency, or iTerm2 `path`) — mechanism, not feasibility. `libc`-only crate.
  - _Note: dev-build trust held across rebuilds via `codesign --force --sign -` (R14); for the spike, the binary read cross-app titles fine once trusted._
- [x] **Project-inference spike — ✅ PASS (snapshot)** (`spikes/project-inference/`). On a corpus of real signals (live-resolved terminal cwds + real browser tabs + editor titles), the heuristic made **zero false assignments** — identified the 3 active projects (`eyemark_frontend`, `usage_os`, `nudge`) and abstained on everything else (22 signals → 8 assigned, 14 abstained). **`cwd → git remote` is the anchor** (canonical `owner/repo`, no FPs); `github-url` corroborates (R26). **Abstain threshold set** (R27): assign only on HIGH/MED unambiguous signals, else `unassigned` — and **`ambiguous` (localhost/dev-dashboards) is a distinct third state** for Phase-2 temporal correlation. **Headline finding: project identity needs canonicalization** — same project arrived as `f-gozie/usage-os` *and* `usage_os`; the `projects` table must key on the git remote with folder/title/URL as aliases (→ D30). _Snapshot measures precision + abstain, not multi-day recall — re-measure once Phase-1 persists data._
- [x] **Hard-rule-3 cleanup + missing merge gates** ([PR #5](https://github.com/f-gozie/usage-os/pull/5)) — removed all production `.expect()` (`lib.rs`, `watcher.rs`, `db.rs` → shared `now_unix()` helper), added `#![cfg_attr(not(test), deny(clippy::unwrap_used, expect_used, panic))]` to both crate roots, and wired `clippy -D warnings` + `cargo fmt --check` + `tsc --noEmit` into `ci.yml`. (R82) _Binding-freshness gate deferred until tauri-specta is wired._
- [ ] **`recharts` stays for now** — audit §M / R77 called it unused, but it's imported by `src/components/ActivityChart.tsx` (the current dashboard). Remove only when the Bauhaus dial replaces that dashboard (Phase 1+).
- [x] **Wire tauri-specta — ✅ DONE** (`phase1/tauri-specta-ipc`). All 11 commands `#[specta::specta]` with a typed `AppError`; `get_watcher_status` (`serde_json::Value`→`WatcherStatus`) + `get_settings` (tuples→`Setting`) rewritten to named structs; `tauri_specta::Builder` replaces `generate_handler!`; `BigIntExportBehavior::Number` (timestamps export as `number`); `export_bindings` `#[test]` → generated `src/bindings.ts` (deterministic); frontend `src/lib/tauri.ts` re-exports generated types + unwraps `Result`; Linux-only binding-freshness CI gate. **Key finding (D27): the trio is `rc.20`, not the standard's provisional `rc.24`** — rc.24's specta dropped `#[specta(rename)]` on containers, which tauri 2.9.3 still uses, so rc.24 doesn't build. Commands-only stands (#211). Gates green: build, 26 Rust tests, clippy -D, fmt, tsc, 28 TS tests.
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
