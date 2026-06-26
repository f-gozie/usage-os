# Handoff — 2026-06-26-02 · Phase 6 **merged**; clean slate on `main`; next session = **branding + landing page** (Phase 5 launch)

**Where we are now:** Phase 6 (performance, stress & security) is **merged** — [PR #20](https://github.com/f-gozie/usage-os/pull/20), squash `512b667` on `main`, CI green on both macOS + Ubuntu. The working tree is clean (only the two long-standing untracked items remain: `reviews/2026-06-25-full-codebase-audit.claude-findings.json`, `design/logo/`). Ready to branch fresh for the next phase.

**What Phase 6 delivered** (detail in handoff `2026-06-26-01`, `reviews/2026-06-25-phase6-perf-harness.md`, and ADR **D58**):
- **Read path fixed** — bounded-scan in `get_activity_logs` (O(total history) → O(window)); `get_day` 53→1.6 ms, `get_week` 371→10.5 ms at 1M rows, flat across all scales. Made provably complete by a 12 h write-path span cap (`MAX_OPEN_SPAN_SECS`).
- **Reusable perf harness** behind a `perf` cargo feature (generator + read-timing + write-churn tests + `seed_db` bin) — never ships.
- **Write churn** measured (45k writes/sec) → R57 deferred with data. **Memory:** live RSS ~170 MB → 2.13 GB was transient, not a leak.
- **Security:** `cargo-deny` is a committed gate (`deny.toml`) — 2 vulns fixed (`bytes`/`time`), crate `license = "MIT"` declared, no-network reaffirmed.
- Read budgets **ratified** (D58): `get_day`<50, `timeline`/`recap`<100, `week`<150 ms — all pass with big headroom.
- Reviewed via `/usageos-review`; the Codex cross-model lane caught one Critical (read-bound contract) — resolved by the span cap. 127 Rust + 32 TS tests green.

## NEXT SESSION — branding + landing page (Phase 5 launch kickoff)
The owner's direction: **finalize branding and build the landing page.** This is the front of **Phase 5 (launch)**, now unblocked (Phase 6 hardened the app first, per the owner's sequencing). Concretely:
- **Branding:** finalize the name/wordmark + domain (working name **UsageOS**, leaning **usageos.app**), the logo (a `design/logo/` folder already exists untracked — start there), app icon, and the visual identity carried from the frozen Bauhaus design system (`context/design-system.md`).
- **Landing page:** the public marketing/landing site — lead with the product's promise (a private, on-device time mirror; *active usage / screen-time*, not "where your day went" — see the tweet-voice memory for framing), the privacy story (nothing leaves the machine), and the dial/recap visuals. Plain, human, modest copy (copy-voice + tweet-voice memories).
- Rest of Phase 5 (can follow): notarized DMG + auto-update + Homebrew cask; README rewrite for the new product; Sponsor link.
- **Optional Phase-6 tail** (non-blocking, anytime): confirmatory release soak (heavy DB + long Timeline nav via `seed_db --months 24`); idle-CPU profiling; native `osascript`-off-main-loop; migration `CHECK` constraints.

## To start next session
1. Read the active plan (`plan.md`) + this handoff. Phase 6 rows are checked; Phase 5 is the launch section.
2. Branch from `main` (e.g. `phase5/branding-landing`). Mockups before UI (the `mockup` skill); deliver real HTML mockup files to open in the browser (design-session-workflow memory).
3. Register a Phase-5 plan folder if the work spans sessions, or keep it under the active redesign plan's Phase 5.
