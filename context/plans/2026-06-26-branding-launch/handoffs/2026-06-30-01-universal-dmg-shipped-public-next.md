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

## 5. ⏰ FIRST TASK for the next agent — data migration (owner has NOT installed yet; wait for their "installed" signal)
The bundle-id change (`com.favour.usage-os` → `com.usageos.app`) means the freshly-installed app opens a **brand-new, empty data dir** — the owner's accumulated dogfood history (the real titles/URLs they've been testing against) lives under the **old** id. The owner explicitly asked: **copy the old store into the new location, then delete the old.** Exact procedure (run it ONLY after they confirm they've installed + launched at least once):

```bash
OLD="$HOME/Library/Application Support/com.favour.usage-os"
NEW="$HOME/Library/Application Support/com.usageos.app"
# 1. Quit the new app first (it locks the SQLite WAL).
osascript -e 'tell application "UsageOS" to quit' 2>/dev/null; sleep 2; pkill -x usage-os 2>/dev/null
# 2. Back up the new (empty) dir, then copy the OLD store over it (DB + WAL + SHM + settings).
[ -d "$NEW" ] && mv "$NEW" "$NEW.fresh-backup-$(date +%s)"
cp -R "$OLD" "$NEW"
# 3. Relaunch and VERIFY the history shows (query the DB or open the app):
sqlite3 -readonly "$NEW/usage.db" "SELECT COUNT(*) FROM activity_logs;"   # should be the old large count
open -a UsageOS
# 4. ONLY after the owner confirms their data is there, delete the old:
#    rm -rf "$OLD"   # and the .fresh-backup-* dir
```
**Heads-up:** the new app is a new TCC identity, so the owner must **re-grant Accessibility + Automation** for `com.usageos.app` (the old `com.favour.usage-os` entries in System Settings → Privacy & Security are now stale — owner can remove them by hand; TCC isn't scriptable). The Universal DMG they're installing is at `~/Downloads/UsageOS-0.1.0-universal.dmg`.

## 6. FINAL-STEPS PLAYBOOK for the next agent (the owner's explicit ask: "finish the installer branding + the final steps")

### A. Brand the installer — the DMG background (owner flagged it looks plain: drag works, no backdrop art)
- The DMG window is **660×400** (`tauri.conf.json` → `bundle.macOS.dmg.windowSize`); the app icon sits at **(180,180)**, the Applications alias at **(480,180)**.
- **Design a background image at 660×400 (+ a `@2x` at 1320×800)** in the brand: the bone/paper field (`#EEEBE1`, the app-icon tile color), the **Contexts** dial mark + `USAGEOS` wordmark across the top, and a calm "drag → Applications" cue running between the two icon slots. Hold to the frozen design system (`context/design-system.md`): ink + paper, **blue the only chrome accent**, the dial's blue/red/gold **only inside the dial**, no other color blocks. The mark + a "DMG" surface mock already exist in `design/logo/contexts.html` — render/export from there, or build it with the `frontend-design` / `mockup` skills and export PNGs.
- Drop them at `src-tauri/dmg-background.png` (+ `@2x`) and add `"background": "dmg-background.png"` inside `tauri.conf.json` → `bundle.macOS.dmg`. Then rebuild via §E.

### B. Landing — make the download the focal point
The "Download for Mac" CTAs point at `releases/latest` (resolves once the repo is public). Surface it properly: a prominent button to the universal DMG with "**Universal · macOS 13+ · notarized · ~11 MB**". Landing is **Astro in `landing/`**; push to `main` → Cloudflare auto-deploys (preview URLs on PRs).

### C. A few more landing tweaks — **ask the owner** for the specifics (they said "a few tweaks").

### D. Release tail
Auto-update (Tauri updater — generate the **ed25519 updater key**, host `latest.json` on Releases, add the Settings toggle + the one-line disclosure of exactly what the update check sends; see [D61]) · Homebrew cask · GitHub social preview (set it to `landing/public/og.png` — owner 1-click).

### E. Rebuild → re-notarize → swap-onto-release recipe (used by A, and any future build)
```bash
source ~/.appstoreconnect/usageos-signing.env     # APPLE_SIGNING_IDENTITY + the ASC API key (gitignored)
./scripts/release-macos.sh                          # universal build → signs the app + nested sidecar (hardened runtime) → DMG
DMG="src-tauri/target/universal-apple-darwin/release/bundle/dmg/UsageOS_0.1.0_universal.dmg"
# Notarize + staple explicitly (reliable; the keychain profile is set up; notary is ~2 min now):
xcrun notarytool submit "$DMG" --keychain-profile usageos-notarytool --wait
xcrun stapler staple "$DMG" && xcrun stapler validate "$DMG"
# Confirm a downloaded copy passes Gatekeeper:
T=/tmp/dl.dmg; cp "$DMG" "$T"; xattr -w com.apple.quarantine "0083;0;Safari;" "$T"; spctl -a -t open --context context:primary-signature -vv "$T"; rm -f "$T"
# Swap onto the v0.1.0 release:
cp "$DMG" /tmp/UsageOS-0.1.0-universal.dmg
gh release delete-asset v0.1.0 UsageOS-0.1.0-universal.dmg -y
gh release upload  v0.1.0 /tmp/UsageOS-0.1.0-universal.dmg
# If the shipped CODE changed (not just the DMG art): commit → branch → PR → merge, THEN re-point the tag:
#   git tag -f v0.1.0 <merged-main-sha> && git push -f origin v0.1.0
```
> `gh release …` works. **`gh pr edit`/`gh pr merge` hit a GitHub "Projects-classic" deprecation** and fail silently — fall back to REST: `gh api -X PUT repos/f-gozie/usage-os/pulls/<n>/merge -f merge_method=merge` and `gh api -X PATCH repos/f-gozie/usage-os/pulls/<n> -f body=...`.

### F. THE LAUNCH — owner's two clicks (you can't do these)
1. Make the repo **public** (GitHub → repo Settings → General → Danger Zone → Change visibility).
2. Clear **GitHub Actions billing** (Settings → Billing) → CI runs + the README badge goes green.
Then the release page, the DMG URL, and the landing "Download" all resolve — UsageOS is live.

### Lingering low nits (optional, batch into a tweak PR)
CHANGELOG/version question (public `v0.1.0` = the redesign vs the old prototype `0.1.0`) · orphaned `docs/screenshots/onboarding-*.png` · `git push origin --delete` the merged `phase*/landing*/docs*/release*` branches · `architecture.md`'s ASCII layer map still shows old module names (a clarifying note sits below it).

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
