# Handoff — 2026-06-30-01 · v0.1.0 **Universal notarized DMG SHIPPED**; repo still **private** — public flip + landing tweaks next

## 1. Current state
- **UsageOS — the private, on-device macOS time tracker. Phase 5 (branding + launch): M1–M5 done.**
- **The notarized Universal DMG is live** on the [v0.1.0 GitHub release](https://github.com/f-gozie/usage-os/releases/tag/v0.1.0): `UsageOS-0.1.0-universal.dmg` — Intel + Apple Silicon, bundle id `com.usageos.app`, signed (Developer ID + hardened runtime), notarized + stapled, Gatekeeper-clean on a quarantined download. The `v0.1.0` tag points at the merged `main` (`bb1e8c7`) so source == shipped DMG.
- **BUT the repo is still PRIVATE** — so the release page, the DMG URL, and the landing's "Download" link all **404 for the public**. **Flipping the repo to public is the actual launch switch** (owner action — I can't change repo visibility).
- The whole history was audited (secrets + showcase quality, two multi-agent workflows): **no secrets, history is clean, keep it** (no rewrite, no `.mailmap` — owner kept the two author emails). Pre-public docs polish merged ([PR #28](https://github.com/f-gozie/usage-os/pull/28)).

## 2. What landed this session (big one)
- **Signing + notarization set up from scratch** ([D64]): created the **Developer ID Application cert** (none existed — Nudge is iOS) via a local CSR → account-holder portal → PKCS#12 import; a dedicated **App Store Connect API key** (`usageos-notarytool` keychain profile). All creds live in `~/.appstoreconnect/` (gitignored), see `~/.appstoreconnect/usageos-signing.txt`.
- **M5 release pipeline** ([D64]): `scripts/release-macos.sh` + `gen-licenses.sh` + bundled `THIRD-PARTY-LICENSES.html`; `seed_db` bin → cargo example (the bundler was trying to ship it).
- **Universal + bundle id** ([D65]): `com.favour.usage-os` → **`com.usageos.app`**; Universal (Intel+AS) build; `src-tauri/Info.plist` adds `NSAppleEventsUsageDescription` (Tauri auto-merges it). [PR #29](https://github.com/f-gozie/usage-os/pull/29), merged.
- **Two capture/UI fixes found dogfooding** (PR #26): Timeline shows window **titles** not the always-"—" project ([D62]); **Chromium/Electron apps now capture titles + URLs** via `AXManualAccessibility` — Chrome/Claude were blank ([D63]).
- **Telemetry & auto-update strategy** recorded ([D61], two-plane model; unimplemented).
- **Pre-public docs polish** (PR #28): de-staled README/CLAUDE/vision/architecture (embeddings shelved D47, WAL on, CI Linux+macOS), archived the gamification specs → `context/history/`, new `spikes/README` + `design/README`, rewrote `CONTRIBUTING`, `CHANGELOG` → pointer, resolved the **duplicate ADR** (M5 was a second "D60" → renumbered **D64**).
- **Attribution killed for good:** `~/.claude/settings.json` → `attribution: { commit: "", pr: "" }` (global, all projects); memory rule [[never-claude-attribution]]; scrubbed from PRs #26/#28. **Never add "Generated with Claude Code" to commits/PRs.**

## 3. Key decisions
**D61** telemetry/auto-update · **D62** Timeline titles · **D63** Chromium `AXManualAccessibility` · **D64** M5 distribution (was the dup "D60") · **D65** Universal build + `com.usageos.app`. (D60 = brand positioning, unchanged.)

## 4. Blockers (both owner-side, neither is code)
- **Repo is private** → nothing public is reachable. Owner: GitHub → repo Settings → make public. **This is the launch.**
- **GitHub Actions billing** failing → CI red (the badge in the README). Owner: Settings → Billing. All gates pass locally; GitGuardian + Cloudflare Pages are green.

## 5. Work in progress / pending owner action
- **Data migration (waiting on owner):** the bundle-id change means the new app uses a **fresh `com.usageos.app` data dir** — the owner's accumulated test data is under the old `com.favour.usage-os`. Owner asked me to **copy the old store → new + delete the old app/data once they've installed.** Do this when they confirm installed (quit the app first; copy `~/Library/Application Support/com.favour.usage-os/` → `…/com.usageos.app/`; verify; then remove the old).
- The Universal DMG is also at `~/Downloads/UsageOS-0.1.0-universal.dmg` (owner installing it).

## 6. TODO — next session = **landing tweaks + wire the download** (owner's stated focus)
1. **Landing: surface the download** — the "Download for Mac" CTA currently points at `releases/latest`; make the download prominent on the page (direct button / version + size, "Universal · macOS 13+"). Landing is Astro in `landing/`, auto-deploys on push to `main`.
2. **A few landing tweaks** (owner has specifics).
3. **DMG branded background** (cosmetic) — the installer window is currently plain (drag works, no backdrop art). Add a `bundle.macOS.dmg.background` image; quick rebuild + re-notarize (~2 min now).
4. **The release tail:** auto-update (Tauri updater — generate the **ed25519 updater key**, host `latest.json` on Releases, Settings toggle + disclosure, see [D61]) · Homebrew cask · GitHub social preview (`landing/public/og.png`, owner 1-click).
5. **Then: owner flips the repo public** + fixes Actions billing → download goes live.
- *(Lingering doc nits, low: the CHANGELOG/version question — public `v0.1.0` is the redesign vs the old prototype `0.1.0`; orphaned `docs/screenshots/onboarding-*.png`; delete merged remote branches; the architecture.md ASCII layer map still shows old module names though a clarifying note was added below it.)*

## 7. Gotchas
- **Notarization is slow ONLY the first time** (first-submitter "additional analysis", ~24h — a documented Apple behaviour, not an outage; confirmed via Apple DTS forums + Codex). Subsequent submissions: ~2 min. Don't panic or re-submit en masse.
- **Universal build needs THREE sidecar slices** — `sidecar/build.sh --universal` emits `usageos-ai-{aarch64,x86_64,universal}-apple-darwin` (Tauri builds each arch then assembles). The sidecar is gitignored; FoundationModels links on x86_64 and degrades to the template recap on Intel.
- **`bundle.macOS.infoPlist` is NOT a valid Tauri 2.9.4 config key** — use a `src-tauri/Info.plist` (auto-merged).
- **`tauri build` / `cargo` target dir is huge (14 GB)** — disk filled mid-session; `rm -rf src-tauri/target` to reclaim. Sidecar still git-ignored (don't stage the CI placeholder stub).
- **Re-tagging:** if a future build changes the shipped code, move the `v0.1.0` tag (or cut a new version) so the tag matches the DMG.

## 8. Testing status
All local gates green (fmt, clippy `--all-features`, 127 Rust tests, tsc, 32 vitest, bindings fresh) before the merges. The Universal DMG was verified end-to-end: both main binary + sidecar fat (`x86_64 arm64`), signature valid, bundle id `com.usageos.app`, notarized + stapled, `spctl` accepts a quarantined copy as "Notarized Developer ID". On-device capture (titles + Chrome/Claude URLs) confirmed working. CI not run (Actions billing).

## 9. Next steps recommendation
Owner is installing the Universal DMG. **Immediate:** when they confirm, do the **data migration + old-app cleanup** (§5). **Next session:** the **landing download wiring + tweaks** (§6.1–6.2) + optionally the **DMG background** (§6.3). **The launch itself** is two owner clicks — make the repo public + clear Actions billing — after which the download is live. The release tail (auto-update/Homebrew/social) is the last stretch to a complete v1.
