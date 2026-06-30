# Handoff — 2026-06-30-02 · **Release tail SHIPPED** (branded DMG · Automation fix · data migrated · landing live · opt-in auto-update · Homebrew). The only thing left is the **owner's launch switch**.

## 1. Current state
- **Everything technical for v1 launch is done.** What remains is two owner-only clicks: **make the repo public** + **clear GitHub Actions billing**. Once public, the download link, the Homebrew cask, and the in-app updater all resolve (they 404 while private). That flip **is** the launch.
- The `v0.1.0` GitHub release now carries **three** assets, all built from `main` (tag re-pointed to `84bfe55`, so source == shipped):
  - `UsageOS-0.1.0-universal.dmg` (~14.7 MB — branded background, **now contains the auto-updater**, notarized + stapled, Gatekeeper-clean on a quarantined copy)
  - `UsageOS.app.tar.gz` (the signed updater artifact)
  - `latest.json` (the updater manifest, v0.1.0)
- **Landing is LIVE** at usageos.app (verified): rewritten hero copy, widened to 1220px + side borders, and the download CTAs wired straight to the DMG with "Universal · macOS 13+ · ~11 MB" (no "notarized" — owner corrected that twice; see [[no-notarized-in-copy]]).

## 2. What landed this session (a big one — 6 PRs + a tap)
- **Branded DMG installer background** ([PR #30](https://github.com/f-gozie/usage-os/pull/30), #31) — 660×400 retina TIFF (bone field, the Contexts dial the only color, blue drag arrow); caption trimmed to "v0.1.0 · universal". Source: `design/dmg/dmg-background.html`.
- **Automation "Grant access" actually prompts now** ([PR #32], **[D66]**) — dogfood bug on macOS 26: the wildcard `AEDeterminePermissionToAutomateTarget` doesn't raise the consent dialog; now sends a real Apple Event to each running browser, and returns `NoBrowserRunning` so onboarding shows an "open your browser" hint.
- **Data migration done** — copied the owner's 4,622-log dogfood history from the old bundle id `com.favour.usage-os` into `com.usageos.app`, verified (integrity ok, real titles+URLs), deleted the old store + backup. The live store is `~/Library/Application Support/com.usageos.app/`.
- **Landing: download wiring + width + copy** ([PR #33]) — see above. Astro in `landing/`; push to `main` → Cloudflare deploys.
- **Opt-in auto-update** ([PR #34], **[D67]**, default decided by a 2-round Codex-vs-Opus `/debate`) — `tauri-plugin-updater` (pinned `=2.9.0` to keep tauri on 2.9.x). **OFF by default**; onboarding "Updates" step recommends Enable; Settings "Software update" toggle + manual "Check now"; launch check debounced once/24h; calm install banner. Rust-side fetch of `latest.json` (bypasses CSP — the sanctioned exception), ed25519-signed, **hard rule 1 untouched**.
- **Signed v0.1.0 rebuild + ship** ([PR #35] for pubkey/licenses/changelog) — rebuilt with the updater, uploaded all three assets, re-pointed the tag.
- **Homebrew tap** — created **`f-gozie/homebrew-tap`** (public) with `Casks/usageos.rb` (sha256 `2daa59ca…`): `brew install --cask f-gozie/tap/usageos`. README updated.
- **CHANGELOG + version** — kept **v0.1.0** as the honest first *public* release; real entry added.

## 3. Key decisions
**D66** Automation prompt via a real Apple Event (not the wildcard determination). · **D67** Software updates are **opt-in** (default OFF), Tauri updater, ed25519-signed — chosen over default-on to keep hard rule 1 verbatim for a privacy launch.

## 4. ⚠️ CRITICAL — the updater signing key (don't lose this)
- The ed25519 **private key** is `~/.appstoreconnect/usageos-updater.key`, password **`usageos-updater-2026`** (both also in `usageos-signing.env`, all gitignored, outside the repo). The **public key** is baked into `tauri.conf.json`.
- **Back both up somewhere safe.** If the key or password is lost, you can never sign an update that already-installed copies will accept — you'd have to ship a new pubkey, orphaning every existing install from updates.
- For CI builds later, add the key + password as GitHub Actions secrets (`TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`).

## 5. FIRST for the next agent / owner
1. **THE LAUNCH (owner, 2 clicks):** repo Settings → make **public**; Settings → Billing → clear **Actions billing**. Then DMG download, `brew install --cask f-gozie/tap/usageos`, and the in-app updater all work.
2. **Verify the updater end-to-end on the NEXT release** — it can't be tested 0.1.0→0.1.0 (same version = "up to date"). When you cut v0.1.1: bump `version` in `tauri.conf.json` + `package.json`, run `./scripts/release-macos.sh` (it builds + signs the DMG **and** the `.app.tar.gz` + `latest.json` via `scripts/gen-latest-json.sh`), then upload the DMG + `UsageOS.app.tar.gz` + `latest.json` to the new release (use the same stable asset names). An opted-in 0.1.0 install should then see and install 0.1.1.
3. **Owner 1-clicks:** GitHub social preview → `landing/public/og.png`; Sponsor link is already in the footer.

## 6. How updates work (for reference)
- **Site:** push to `main` → Cloudflare rebuilds usageos.app (~1–2 min).
- **App:** opted-in installs fetch `latest.json` from the release once/24h; if its `version` is newer, download `UsageOS.app.tar.gz`, verify the ed25519 signature, install, relaunch (`restart_app`). Homebrew users: `brew upgrade` (bump the cask's `version` + `sha256` per release).

## 7. Gotchas (learned this session)
- **`tauri signer generate --ci` is NOT passwordless** — it produces a password-encrypted key whose password isn't empty. Regenerate with an explicit `-p` and store it. (Cost us a failed build mid-session.)
- **The updater drags tauri toward 2.10** — `tauri-plugin-updater` caret `2.9.0` resolves to 2.10.1 (needs tauri ≥2.10). Pin **`=2.9.0`** to stay on 2.9.3 (the project's shell-plugin pin keeps it there).
- **New license to allow:** the updater's HTTPS stack pulls `webpki-roots` (`CDLA-Permissive-2.0`) — added to `deny.toml` + `about.toml`.
- **The updater 404s while the repo is private** (the endpoint is `releases/latest/download/latest.json`). Expected; resolves on the public flip.
- **`gh pr merge` still hits the Projects-classic bug** — merge via REST: `gh api -X PUT repos/f-gozie/usage-os/pulls/<n>/merge -f merge_method=merge`. The tap push needs the **SSH** remote (HTTPS has no cred helper here).
- **Build PATH:** prepend `/usr/bin` so Apple's `lipo`/`codesign` beat anaconda's (`PATH="/usr/bin:$PATH" ./scripts/release-macos.sh`).

## 8. Testing status
All merge gates green on every PR (fmt, clippy `--all-features`, **127 Rust**, tsc, **32 vitest**, bindings fresh, cargo-deny licenses ok). The shipped DMG verified end-to-end: universal main + sidecar, Developer-ID + hardened runtime, notarized + stapled (app *and* DMG), Gatekeeper-clean quarantined. `latest.json` + signed `.app.tar.gz` validated. Updater UX builds + typechecks; **its real network behavior is untested until the repo is public + a newer version exists** (§5.2).

## 9. Next steps recommendation
The owner flips the repo public + clears Actions billing — that's the launch. Then smoke-test: download the DMG fresh, `brew install --cask f-gozie/tap/usageos`, enable auto-update in Settings. The first real updater test comes with v0.1.1. Remaining polish (social preview, the older lingering nits from handoff -01) is optional.
