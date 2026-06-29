# Handoff — 2026-06-29-01 · M5 pipeline **built, signed, merged**; the notarized DMG is blocked on a **24h+ Apple Notary outage**

## 1. Current state
- **Project:** UsageOS — the private, on-device macOS time tracker. **Phase 5 (branding + launch).**
- **M5 (notarized DMG) pipeline is DONE and merged** — [PR #26](https://github.com/f-gozie/usage-os/pull/26) merged to `main` (`e80bfc4`). Signing, notarization credentials, the release script, license notices, and the bundle config all work and are verified. **The only thing missing is the notarized DMG itself, blocked by an extraordinary Apple Notary Service outage (>24h).**
- Two capture/UI fixes also landed this session (found dogfooding the signed build): **Timeline shows window titles** ([D62]) and **Chromium/Electron apps capture titles+URLs** ([D63]).
- Reviewed via `/usageos-review` (0 Critical; report `reviews/2026-06-28-m5-notarization.md`). ADRs **D60–D63** appended.

## 2. What landed this session
**Signing/notarization credentials (the real prior blocker — all OUTSIDE the repo in `~/.appstoreconnect/`):**
- Created a **Developer ID Application cert** (none existed — Nudge is iOS): local CSR → account-holder web portal → imported as PKCS#12 w/ the Apple G2 intermediate. `codesign` signs unattended (full chain + hardened runtime + secure timestamp verified). The ASC API **cannot** mint Developer ID certs (403, account-holder-only).
- **App Store Connect API key** "UsageOS Notarization" (Developer role) → `notarytool` keychain profile **`usageos-notarytool`**. Issuer `cdbb6905-…`, Key `TAWUQ7F26N`.
- Reference (gitignored): `~/.appstoreconnect/usageos-signing.txt` (facts) + `usageos-signing.env` (the env the release script sources) + `private_keys/` (`.p8`, `usageos-devid.{key,cer,csr.pem}`). **These are the only copies — back them up; the `.p8` can't be re-downloaded.**

**M5 pipeline (D60):** `scripts/release-macos.sh` (preflight → real sidecar → license notices → sign app + nested `usageos-ai` w/ hardened runtime → notarize → staple → DMG → verify). `scripts/gen-licenses.sh` + `gen-licenses-extra.mjs` + `src-tauri/about.{toml,hbs}` → committed `THIRD-PARTY-LICENSES.html` (bundled resource). `tauri.conf.json`: `mainBinaryName`, `minimumSystemVersion 13.0` *(confirm)*, DMG layout, license resource. **`seed_db` bin → cargo `example`** (Tauri's bundler copies every package bin into the `.app`; `mainBinaryName` alone didn't fix it).

**Capture/UI fixes:** **D62** — `rollup::TimelineSegment.title` (+ regenerated bindings) so the Timeline expand shows the window title (`truncate`/`minmax(0,1fr)` ellipsis), not the always-"—" project. **D63** — `capture/macos/mod.rs` sets `AXManualAccessibility` on each focused app so Chromium/Electron (Chrome, the Claude app, Slack, …) expose their window title + URL; **verified on-device** (Chrome now records full titles + front-tab URLs).

**Also:** **D61** telemetry & auto-update plan recorded (two-plane: data never leaves; downloads/active-installs/geo come from GitHub + Cloudflare + the update-ping, never in-app analytics).

## 3. The blocker — Apple Notary outage (NOT our code)
- **Three submissions, all stuck "In Progress" 13–24h+:** `ea724dfa` (08:27Z, pre-fix), `d72814d3` (14:08Z, pre-fix), **`0673edc2` (20:33Z, the FIXED build — the one to ship).** All on 2026-06-28. None processed.
- This is 100% Apple-side. Our build/signing/cert/credentials are all verified correct (`codesign --verify --deep --strict` passes; upload endpoint works — submissions accept). **Re-submitting does NOT help** (joins the same backlog).
- **A resilient watcher is armed** (background task `br20b98et`, polls `0673edc2` every 15 min for ~20h) → on `Accepted` it staples the **already-built DMG** at `src-tauri/target/release/bundle/dmg/UsageOS_0.1.0_aarch64.dmg` (5.3 MB, signed). If it lapses, just re-run the staple (see §6).

## 4. Other blocker — GitHub Actions billing
- **CI on PR #26 / `main` can't run** — *"job not started because recent account payments have failed or your spending limit needs to be increased."* GitHub → Settings → Billing & plans. **Not code** — all gates pass locally (fmt, clippy `--all-features`, 127 Rust tests, tsc, 32 vitest, bindings fresh). GitGuardian + Cloudflare Pages pass.

## 5. Work in progress / uncommitted
- **Nothing uncommitted of mine.** PR #26 merged. This handoff is committed to `main` directly (process doc). (Untracked `design/social/`, `design/landing-toggle-affordance.html` are from parallel branding work — not mine, left alone.)

## 6. TODO — next session
1. **When `0673edc2` flips to `Accepted`** (`xcrun notarytool info 0673edc2-f482-4ece-9651-1890dc5c5e4f --keychain-profile usageos-notarytool`): the watcher staples the DMG; if not, run `xcrun stapler staple <the DMG>` + `stapler validate`.
2. **Create the GitHub release** `v0.1.0` + upload the stapled `UsageOS_0.1.0_aarch64.dmg` → the landing's "Download for Mac" (`releases/latest`) goes live. (Confirm publish-vs-draft with the owner.)
3. **Owner ~1 click:** set the GitHub social preview to `landing/public/og.png`.
4. **Fix GitHub Actions billing** so CI runs green.
5. **Phase-5 tail:** auto-update (Tauri updater — generate the **ed25519 updater key**, host `latest.json` on Releases, Settings toggle + disclosure; see [D61]) · Homebrew cask · Sponsor link.
6. *(Polish, optional)* a branded DMG background; consider notarizing+stapling the app *inside* the DMG too (current: the DMG is notarized+stapled; the app inside is notarized but relies on an online check on first launch).

## 7. Gotchas
- **Notarization is per-build.** Any code change ⇒ re-run `scripts/release-macos.sh` for a fresh submission. The stale `ea724dfa`/`d72814d3` are pre-fix — abandon them.
- **Don't ship the un-notarized DMG** to users (Gatekeeper blocks downloads). A signed-but-un-notarized DMG is fine for **local** testing only (no quarantine on the build machine).
- **TCC permissions persist across rebuilds** signed with the same Developer ID (stable designated requirement) — local rebuilds don't reset Accessibility/Automation. **Chrome/Claude need the `AXManualAccessibility` build** (D63) + Automation approved per-browser for URLs.
- Sidecar is git-ignored; `sidecar/build.sh` before any release build (never the CI placeholder stub).

## 8. Testing status
- All local gates green (fmt, clippy `--all-targets --all-features`, 127 Rust tests, tsc, 32 vitest, bindings fresh). `/usageos-review`: 0 Critical. The signed build was **dogfooded on-device** — capture, titles (incl. Chrome/Claude after D63), on-device recap, tray all working. The notarized-DMG path is verified up to the notary submit; only Apple's processing is outstanding.

## 9. Next steps recommendation
Nothing to build — **wait out Apple's notary**, then steps §6.1–§6.2 finish the ship (staple → release → live download). In parallel the owner can clear the **GitHub Actions billing** and set the **social preview**. Then the Phase-5 tail (auto-update / Homebrew / Sponsor) is the last stretch to v1.
