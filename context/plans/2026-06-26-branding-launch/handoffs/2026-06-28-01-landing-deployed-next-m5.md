# Handoff — 2026-06-28-01 · Phase 5 landing **LIVE at usageos.app**; only **M5 (notarized DMG)** left

## 1. Current state
- **Project:** UsageOS — the private, on-device macOS time tracker.
- **Phase 5 (branding + launch):** **M1–M4 done. The landing is LIVE at [usageos.app](https://usageos.app).**
- [PR #21](https://github.com/f-gozie/usage-os/pull/21) (branding/identity) and [PR #22](https://github.com/f-gozie/usage-os/pull/22) (landing depth pass + fonts/theme + affordances) are **merged to `main`**. The `phase5/identity` and `landing/fonts-theme-polish` branches are deleted.
- Reviewed twice via `/usageos-review` (gates green incl. `--features perf`, 0 Critical) — reports in `reviews/`. ADR **D59** appended.
- **The only milestone left in this plan is M5** (notarized DMG + release tail).

## 2. What landed since the last handoff (2026-06-26-01)
**Review + merge:** `/usageos-review` on PR #21 (report `reviews/2026-06-26-phase5-branding.md`) → safe fixes → merged. Pre-merge re-review of the landing pass (`reviews/2026-06-27-landing-depth-premerge.md`) → merged.

**Landing depth pass (D59):** new **"You decide what counts as work"** categorization section with a live **re-sort toggle**; the dark Privacy prose → the app's real **exclusion rows**; cut the redundant stats band; **"Deep work" → "Work"** (canonical name); a11y (global `:focus-visible`, theme switcher `aria-pressed` + persistence + dropped `prefers-color-scheme`, descriptive dial `aria-label`s); a real phone breakpoint; copy pass; timeline reordered newest-first.

**Claim verification (the big one):** every public claim checked against the code. 3 were wrong and corrected on **landing + README + in-app Settings**: (1) category rules match `process`/`title` only — no `site` match → "youtube.com · site" became "YouTube · title"; (2) password managers/banking are **NOT auto-excluded** (no default seed) → claim dropped (deferred to the categorization-v2 plan W3); (3) incognito is recorded as *private* (time counted, title/URL blanked) → "never recorded" → "no title or URL is ever stored". Verified-true-and-kept: reprocess ("edit one rule → past re-sorts" = real, `db::reprocess_logs`), project inference (`enrich::infer_project` runs in capture), no-network, never-Screen-Recording.

**App fixes:** `TimelineRow` renders expanded app-stretches **newest-first** (matches the run agenda); `SettingsView` incognito copy corrected.

**Fonts + theme:** self-hosted Anton/Jost in `landing/public/fonts/` + `@font-face` + **preload** the above-the-fold weights (fixed a serif/Times FOUT) + sans fallback on every Anton stack; **OFL-1.1 license files** ship alongside the woff2; landing **defaults to paper** (saved choice still wins).

**Affordances** (both reduced-motion-gated): timeline — a closed row's **chevron turns blue + gently bobs**; re-sort toggle — the unselected option **breathes a soft blue ring** + hover-lift. Motion-off users keep a static blue chevron + static ring.

**Cloudflare deploy:** git-integrated Pages project (name `usage-os`): root dir `landing`, build `npm ci && npm run build`, output `dist`, Node 22, production branch `main`. Custom domain `usageos.app` (the zone was already on Cloudflare). **Push to `main` → auto-deploys; PRs get preview URLs** (`<branch>.usage-os.pages.dev`).

**Also:** on-device recap was falling back to template because a placeholder sidecar stub had overwritten the real Foundation Models binary — rebuilt via `sidecar/build.sh`, verified `availability=available`. New backlog plan **`context/plans/2026-06-27-categorization-v2/`** (site rules, rule precedence, default exclusions) registered.

## 3. Key decisions (this session)
- **D59** — landing lean-blend depth pass + claim-accuracy corrections (full rationale in `decisions.md`). Direction set by a 2-round `/debate` (Codex + Opus): stay lean/calm, reject demo-reel, close the categorization + privacy-depth gap.
- **Owner calls:** drop the auto-exclude claim now (build the feature later); fix incognito copy (no behavior change); **merge commits** (not squash); landing Download/Sponsor CTAs **left dead `#`** until M5; theme default = **paper**; the one re-sort interaction = the **pulse-ring** affordance (same "the interactive element signals itself" language as the chevron).
- **Self-hosting fonts** is correct architecture + convention (cache partitioning killed the CDN benefit; privacy + preload win). OFL-1.1 permits it; obligation = ship the license with the files (done for landing; **app/DMG still needs it — M5**).

## 4. Blockers
- **None.** M5 notarization needs the Apple Developer ID cert — the owner **has** it (Nudge shipped). Developer-ID + DMG, **not** Mac App Store.

## 5. Work in progress / uncommitted
- **Nothing uncommitted.** All session work is merged to `main`. This handoff + the `plan.md` M2/M5/status updates are committed to `main` directly (process docs).

## 6. TODO — next session = **finish M5** (the last Phase-5 milestone)
1. **Notarized DMG:** Developer-ID sign + notarize + DMG layout (drag-to-Applications).
2. **Sidecar in the release build:** run `sidecar/build.sh` before `tauri build`; Developer-ID-sign the nested `usageos-ai` binary + hardened runtime + `sidecar/entitlements.plist` (else notarization fails).
3. **Third-party license notices** bundled in the DMG (fonts OFL ✓ + npm + crates via `cargo-about`/`license-checker`); optionally an About "Acknowledgements".
4. **Wire the landing Download + Sponsor CTAs** to the DMG / GitHub Sponsors; re-deploy (push to `main`).
5. **Owner ~1 click:** set the GitHub social preview to `landing/public/og.png`.
6. **Then the Phase-5 tail:** auto-update (Tauri updater/Sparkle) · Homebrew cask · Sponsor link.
- *(Separate / future, not Phase 5: the `2026-06-27-categorization-v2` backlog plan — site rules, rule precedence, default exclusions.)*

## 7. Gotchas
- **Sidecar is git-ignored** (built locally via `sidecar/build.sh`). **Do NOT run the CI-style gate that stages a placeholder sidecar stub in the dev tree** — it overwrites the real binary and the on-device recap silently falls back to the template. If it happens, re-run `sidecar/build.sh`. macOS 26 + Apple Intelligence on this machine; Foundation Models works.
- **Cloudflare:** project name `usage-os`; push to `main` → `usageos.app` in ~30–60s; PRs preview at `<branch>.usage-os.pages.dev`. DNS is on Cloudflare (no nameserver work).
- **Landing fonts** are self-hosted in `landing/public/fonts/` with their OFL licenses — no CDN (consistent with the product promise).
- Gates: run `cargo clippy --all-features` + `cargo test --features perf` (the demo generator lives behind `perf`).

## 8. Testing status
- Gates green at both reviews (`fmt`, `clippy --all-features`, `test --features perf`, `tsc`, `vitest`, bindings fresh). Landing builds; PR preview deploys verified (paper default, preloaded self-hosted fonts, affordances). On-device recap verified working after the sidecar rebuild. Native tray/AX/sidecar paths aren't in CI (by design).

## 9. Next steps recommendation
Start **M5**: build + sign the sidecar, then sign/notarize the app, lay out the DMG, bundle the third-party notices, wire the download CTA, push to `main` (auto-deploys), set the social preview. That completes Phase 5 — effectively v1 ship.
