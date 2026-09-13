# Post-launch feedback

The rolling home for what real users surface now that v0.1.0 is public. Each item traces back
to a concrete report (tester message, GitHub issue, dogfood note) — this plan collects them so
feedback-driven work has the plan → impl-plan → review → handoff paper trail without inventing
a new plan folder per nit.

**Scope:** app-behavior polish driven by user feedback. Bigger themes that grow a roadmap of
their own (e.g. categorization gaps → `2026-06-27-categorization-v2/`) get spun out, same as
Phase 5 → branding-launch.

## Items

### 1. Menu-bar agent: Dock icon follows the window + start at login (D68)
_Source: first tester feedback (2026-07-01 WhatsApp) — "wish the app didn't have to be open,
just the menu bar" / "I've closed it but it's still showing on my Dock"._

- [x] Dynamic activation policy — Regular while the dashboard is open, Accessory when closed
      (close → no Dock icon; tray/Reopen → icon returns)
- [x] `RunEvent::Reopen` → show the window (Finder/Spotlight re-open while Dock-less)
- [x] Start at login: `tauri-plugin-autostart` (LaunchAgent, `--hidden`), opt-in default OFF
- [x] `--hidden` launch starts in menu-bar mode (window `visible: false` + setup branch)
- [x] Settings → new "Background" group with the toggle
- [x] Onboarding "Background" step (recommended, skippable) between Automation and Updates
- [x] Tests: onboarding walk + enable path, BackgroundSettings toggle, SettingsView mock
- [x] On-device verification in dev (Dock cycle, glance reopen, LaunchAgent plist, `--hidden` launch)
- [x] `/usageos-review` ([review](reviews/2026-07-01-menubar-agent.md) — 5 warnings found + fixed,
      incl. update-restart-relaunches-hidden and toggle-revert-on-failure) + PR
- [x] Bundled-build smoke test (merged as [PR #36](https://github.com/f-gozie/usage-os/pull/36)):
      LS launch shows the window · close → no Dock icon · `open` again fires Reopen → window back
      focused · toggle writes/removes the LaunchAgent plist (bundle path + `--hidden`) · `--hidden`
      launch is menu-bar-only
- [x] **Shipped in [v0.1.1](https://github.com/f-gozie/usage-os/releases/tag/v0.1.1)** — DMG
      (notarized + stapled) + updater artifact + latest.json live; landing + Homebrew cask bumped
- [x] Owner check: in-app update 0.1.0 → 0.1.1 via Settings → Check now — **worked first try**
      (found, downloaded, verified, installed, relaunched)
- [ ] Owner check: a real log-out/in with the toggle on (incl. any Dock-icon flash at login)

### 2. About modal showed a stale version after the first real update
_Source: owner, right after the 0.1.0 → 0.1.1 in-app update (which otherwise worked first try)._

- [x] `AboutModal` had a hardcoded `VERSION` constant the release bump didn't cover — now reads
      `getVersion()` from the running binary (PR #37). Shipped in the re-cut v0.1.1.

### 3. Login Items should say UsageOS, not the signing certificate (D69)
_Source: owner, first run of Start at login on v0.1.1 — macOS attributed the background item to
the signing certificate's name instead of the app; reads unofficial._

- [x] Register via `SMAppService.agent` with the agent plist bundled inside the app
      (`bundle.macOS.files` → `Contents/Library/LaunchAgents/`)
- [x] New `login_item` module (cfg-isolated `objc2-service-management` surface); the
      `get/set_launch_at_login` commands and the whole frontend unchanged
- [x] `tauri-plugin-autostart` removed
- [x] Startup migration removes the legacy bare `~/Library/LaunchAgents` plist and re-registers
- [x] Trampoline agent launch: launchd owns the process it spawns, and unregistering kills it —
      the job respawns the app detached and exits, so toggle-off can never kill a login-launched
      session (cross-model review finding; verified on-device: session survives toggle-off)
- [x] Instance flock replaces the process-list guard (atomic; `--hidden` must win it or exit;
      the updater relaunch retries briefly — verified via `launchctl kickstart`)
- [x] Migration registers the bundled agent before deleting the legacy plist
- [x] Bundled-build verification on a Developer-ID-signed build: toggle registers/unregisters,
      **Login Items shows "UsageOS" with its icon**, agent-launched session survives toggle-off,
      no bare plist ever appears. (SMAppService refuses ad-hoc-signed bundles — sign before
      verifying.)
- [x] `/usageos-review` ([review](reviews/2026-07-01-smappservice-login-item.md) — the Codex lane
      caught the unregister-kills-the-app regression pre-merge) + PR
- [x] **Shipped in the re-cut [v0.1.1](https://github.com/f-gozie/usage-os/releases/tag/v0.1.1)**
      (owner call: replace the release rather than add a third) — tag re-pointed to `c0aeb49`,
      all three assets rebuilt from merged main (PRs #37 + #38), DMG notarized + stapled, cask
      sha256 bumped. Installs from the *first* 0.1.1 build won't self-update (same version) —
      reinstall from the DMG.

## Backlog (unclaimed feedback)

_(add items here as reports come in)_

### 4. Capture permissions die silently across updates — detect, surface, guide the fix (D71)

Found by auditing the author's own 32-day database (2026-07-25/31 sessions): window titles ~0%
since the v0.1.1 install (2 Jul 00:53, to the hour — stale Accessibility grant, checkbox still
"on"); browser URLs dying in whole-day chunks whenever Chrome updates and relaunches (-1743
swallowed as "no URL"). The app reported "healthy" throughout: the error counter never fires for
a missing title, and a menu-bar app nobody opens has no banner surface.

- [x] Latch the real Automation denial in `run_osa` (stderr `-1743` classification + tests)
- [x] `evaluate_health` (pure, tested): regression ("was granted, now gone" → alert once) vs
      never-granted (degrade quietly — the user's onboarding choice)
- [x] Health monitor on the existing 10-min tray thread (first pass ~30s after launch);
      last-seen state + alert flag in `settings`
- [x] Tray affordance: degraded tooltip + "Fix recording permissions…" menu item deep-linking
      to the broken System Settings pane (Accessibility first)
- [x] One macOS notification per regression episode (`tauri-plugin-notification` `=2.3.3`,
      local only)
- [x] `WatcherStatus` + the views' degraded banner name the actual problem, with
      switch-it-off-and-on copy for the stale-checkbox trap
- [ ] On-device verification: install the release build while this machine is still degraded —
      the tray fix item should appear within a minute; then re-grant and watch it clear
- [ ] Owner check after the *next* app update: does Accessibility survive, and if not, does the
      notification fire? (This is the case the feature exists for.)

