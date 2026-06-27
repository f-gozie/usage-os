# Review — Phase 5 branding & identity (PR #21)

**Date:** 2026-06-26 · **Scope:** branch (`phase5/identity` vs `main`) · **Files:** 101 (≈13 authored code files; rest = icons/assets/docs/landing/mockups)
**Plan:** [plan.md](../plan.md) · **Impl-plan:** _none — built as one whole-phase branch (owner's call; see handoff §2). DoD note below._
**Codex:** ran (codex-cli 0.130.0, read-only, schema-validated)
**PR:** [#21](https://github.com/f-gozie/usage-os/pull/21)

## Merge gates
Run faithfully to `.github/workflows/ci.yml`, **plus `--features perf`** (covers the new demo generator in `perf.rs`/`seed_db.rs` that the default `cargo test` lane skips).

| Gate | Result |
|---|---|
| cargo fmt --all --check | ✅ |
| cargo clippy --all-targets --all-features -D warnings | ✅ |
| cargo test **--features perf** | ✅ |
| tsc --noEmit | ✅ |
| vitest run | ✅ |
| export_bindings + `git diff src/bindings.ts` (fresh) | ✅ |

_All green. (Re-ran `fmt --check` + `clippy --bin seed_db --features perf` after the two auto-fixes — still green.)_

## Findings
**Verification:** 9 verified · 1 dropped/moot (Codex #6 two-tone — see I1) · 2 auto-fixed · cross-model confirmed where noted

### Critical (must fix before merge)
- **None.** No new network in the data path, no hand-edited bindings (no new commands; `bindings.ts` untouched), no prod-path `unwrap`/`expect`/`panic`, no SQL outside the repository layer, the one `unsafe` block (`localtime_r`) has a SAFETY comment, native surface stays behind `#[cfg(target_os = "macos")]`, UI on-token.

### Warnings (should fix)
- `src-tauri/capabilities/default.json:10` — **[Codex · privacy/least-privilege]** `opener:allow-open-url` is granted **webview-wide**, not scoped to the four fixed About links. Not a hard-rule-1 violation (no network in the data path; opening the system browser is user-initiated and correctly behind a capability), but it's broader than needed. **Practical risk is low** for a local-only app that renders no remote/user HTML. If you want defense-in-depth, route the 4 known URLs through a small Rust command with a hardcoded allowlist, or scope the permission. **Owner decision — manual** (never auto-fix capabilities).
- `landing/src/pages/index.astro:138` (+ footer Sponsor `:145`) — **[Codex · bug]** the primary **Download CTA is a dead `href="#"`** (the hero CTA at `:60` correctly scrolls to `#download`, but the button in that section does nothing); the footer **Sponsor** link is also `href="#"`. **Known/deferred:** wiring the Download CTA is M5 (handoff TODO #4) and the landing isn't deployed yet / the DMG doesn't exist. **Not a code-merge blocker, but a pre-deploy blocker** — make these "Coming soon"/disabled or real links before the landing ships.
- **DoD lifecycle** — `context/decisions.md` has **no branding ADRs** for the ~17 decisions locked this phase (handoff §2; TODO #3 = "append D59+"). The diff touches `context/plans/` so the pre-push tripwire won't fire, but the *why* of the locked branding calls isn't ADR'd yet. Append before/with merge.

### Info
- **I1** — `src/components/ui/Wordmark.tsx:7,48` — **[Codex #6 + verify]** the `mono` prop is **never invoked with `true`** anywhere (both real `<Wordmark>` usages take the default; icon-only spots — `public/favicon.svg`, tray PNGs — are **separate static assets with hardcoded hex**, not this component). So it's speculative generality. _Codex's "mono renders two-tone (track stays `var(--track)`)" is technically true but **moot** since mono is unused — verified, downgraded from Warning._ Consider dropping `mono` + the `DialO` mono branch (project's no-speculative-generality stance), or document its intended future use. (Lane C's claim that "mono is used" was inaccurate — caught in verification.)
- **I2** — `landing/src/styles/landing.css:4-5` — **[Codex + verify]** the **paper** theme matches the app exactly (`--blue #1B45BE` = `--c-deep`; reds/golds/track identical across all 3 themes), but **warm/black `--blue`** (`#5C82F0`/`#5C8BFF`) **differs from the app's `--c-deep`** (`#3358E0`/`#3A6BF0`). The header comment claims it "mirror[s] the app" — true except the dark-theme blue accent. Align the two `--blue` values or soften the comment. (The brighter web blue may be deliberate for hero contrast — owner's call.)
- **I3** — `landing/src/styles/landing.css:25` — **[Codex]** `.btn-p` hardcodes `color:#fff` (off-token). Landing-only and reasonable (no pure-white token exists; #fff on blue is fine). Minor.
- **I4** — `src-tauri/src/perf.rs` (`generate_demo_day`) — **[Lane B]** worst-case demo blocks can spill past local midnight, leaving two overlapping rows straddling midnight in the seed DB. **Not a real defect** — within a day `t` is monotonic (no self-overlap) and the read path clips each span to its day window; artifact is invisible after clipping, dev-only tool behind `perf`. No action.
- **I5** — `src/components/settings/AboutModal.tsx:32` — **[Lane C · voice]** tagline "A calm, private look at **where your time goes**." drifts from the launch "screen-time, honestly" framing used on the landing/README. **Genuine tension** — `CLAUDE.md`'s own product line is "records where your time goes," so this isn't clearly wrong. Owner's call; flagged, not changed.
- **I6** — **DoD** — no `impl-plans/` entry for this branch (built as one whole-phase branch — deliberate per handoff §2). Noted for the registry's plan-anatomy expectation.

## Auto-fixes applied
- `src-tauri/src/bin/seed_db.rs:57` — **[Lane B + Lane C, cross-model-confirmed within Claude lanes]** stale `--help` string now lists the new flags: `… [--max N] [--end EPOCH] [--demo] [--force]`. (fmt + clippy re-checked.)
- `landing/src/pages/index.astro:68` — **[Codex]** faux app-window titlebar `Usageos` → **`UsageOS`** (the locked naming contract; the release `.app` productName is `UsageOS`). Text-only, provably safe.

## Manual TODO
- [ ] Decide on `opener:allow-open-url` scope (hardening; low risk) — keep as-is or route via a Rust allowlist.
- [ ] Before deploying the landing: make the Download CTA (`index.astro:138`) + Sponsor (`:145`) non-dead ("Coming soon"/disabled or real). Tied to M5.
- [ ] Append the ~17 Phase-5 branding ADRs to `context/decisions.md` (D59+).
- [ ] (Optional) Drop the unused `mono` prop from `Wordmark`/`DialO`, or document its purpose (I1).
- [ ] (Optional) Align landing warm/black `--blue` with app `--c-deep`, or soften the "mirrors the app" comment (I2).
- [ ] (Optional) Reconcile the About tagline with the "screen-time" framing (I5).

## Definition of Done
- [x] plan.md ticked for what landed (M1–M4 ✅; M5 pending — accurate)
- [ ] **decisions.md ADRs appended** — NOT done (Warning above; handoff TODO #3)
- [~] impl-plan — none (whole-phase branch, deliberate); **handoff to follow** at session end
- [x] docs move with code — diff touches `context/plans/` (pre-push tripwire would not fire)

## Plan compliance
Alignment: **good.** The diff matches the Phase-5 plan (M1 identity, M2 landing, M3 in-app, M4 README/OSS) with no scope creep beyond branding/launch. The dev-only demo generator (`perf.rs`/`seed_db.rs`) is in-scope supporting tooling for the screenshots M4 needed. Remaining Phase-5 work (M5 notarized DMG + deploy, auto-update, Homebrew, Sponsor) is correctly deferred, and the two open Warnings (opener scope decision; pre-deploy CTA wiring) align with that deferral.
