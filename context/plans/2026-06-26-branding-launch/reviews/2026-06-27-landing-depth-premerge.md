# Review — Landing depth pass + claim-accuracy (pre-merge re-review of PR #21)

**Date:** 2026-06-27 · **Scope:** branch delta since the first review (`8be23df..HEAD`) · **Files:** ~10 authored
**Plan:** [plan.md](../plan.md) · **First review:** [2026-06-26-phase5-branding.md](2026-06-26-phase5-branding.md) · **Decision:** [D59](../../../decisions.md)
**Codex:** ran (codex-cli 0.130.0, read-only, schema) · **Why:** pre-merge gate before merging PR #21 → `main` for the Cloudflare deploy.

## Merge gates (run with `--features perf`; real sidecar present — not stubbed)
| Gate | Result |
|---|---|
| cargo fmt --all --check | ✅ |
| cargo clippy --all-targets --all-features -D warnings | ✅ |
| cargo test **--features perf** | ✅ |
| tsc --noEmit | ✅ |
| vitest run | ✅ |
| export_bindings + `git diff src/bindings.ts` | ✅ |

## Findings
**Verification:** 1 raised · 1 verified · 1 resolved · 0 carried to merge · Codex cross-model: 1

### Critical (must fix before merge)
- **None.**

### Warnings (should fix)
- **None.**

### Resolved this pass
- `design/landing-new-sections.html` — **[Codex, raised Critical → verified, downgraded]** the one-off **scratch mockup** I built before the claim-accuracy verification still carried the *old* false claims ("incognito… never recorded", "password managers/banking… excluded out of the box") and loaded **Google Fonts from a CDN**. In context it's not a hard-rule-1 violation (a non-shipped design artifact, not the product data path, not in the curated `design/` gallery), so not truly Critical — but it's a real cleanup: a committed file contradicting the same commit's accuracy work. **Resolved by deleting it** (its purpose is served; the real, approved landing supersedes it). Good cross-model catch.

### Info / verified-good
- **Landing re-sort toggle (`index.astro` client JS)** — correctness verified: flipping Reddit re-tints the one arc + recomputes Work/Browsing while the total holds at 5h 30m (330 min) in both states; numbers reconcile. Wiring (`aria-pressed` sync + `renderCat`) is correct.
- **a11y (the debate's P0) landed and verified:** global `:focus-visible{outline:2px solid var(--blue)}` (themed → works in all 3 themes); theme switcher `role="group"` + `aria-pressed` + `localStorage` persistence + `prefers-color-scheme`; every dial (`appDial`/`catDial`/week) has a descriptive `aria-label`; the re-sort + privacy controls are real `aria-pressed` groups.
- **Claim accuracy (D59)** re-confirmed against code: rules match `process`/`title` only (landing shows `title`, not `site`); incognito wording = "no title or URL stored" (matches `capture/mod.rs` private treatment); auto-exclude claim dropped (no default seed exists). `TimelineRow` newest-first matches the run agenda.
- **Off-diff but relevant to M5:** the on-device recap was falling back to template because a placeholder sidecar stub (staged by a local gate run) had overwritten the real Foundation Models binary. Rebuilt via `sidecar/build.sh` (real 111 KB Mach-O), verified `availability=available`. For the **notarized DMG (M5)**: the release pipeline must run `sidecar/build.sh` before `tauri build`, and the nested `usageos-ai` binary must be Developer-ID-signed + hardened-runtime + entitled (`sidecar/entitlements.plist`) or notarization fails.

## Auto-fixes applied
- Deleted `design/landing-new-sections.html` (superseded scratch mockup with stale claims + CDN fonts).

## Manual TODO (pre-deploy / M5, not merge-blocking)
- [ ] Landing Download + Sponsor CTAs are intentionally dead `#` links (owner decision) — wire at M5 before the public download matters.
- [ ] Cloudflare Pages: connect repo (root `landing`, `npm ci && npm run build`, `dist`, Node 22), production branch `main`, add custom domain `usageos.app`.

## Definition of Done
- [x] plan.md annotated (M2 depth pass + Cloudflare-in-progress)
- [x] decisions.md ADR appended (D59)
- [x] docs move with code (pre-push tripwire would not fire)
- [~] impl-plan — none (the phase ships as one branch; categorization-v2 backlog plan created)

## Plan compliance
Alignment: **good.** The delta is exactly the landing depth pass + claim-accuracy corrections + the timeline ordering fix + the categorization-v2 backlog plan + Cloudflare build config — all in scope for the Phase-5 launch (M2). No scope creep. Gates green; one scratch-artifact cleanup resolved. **Ready to merge PR #21 → main.**
