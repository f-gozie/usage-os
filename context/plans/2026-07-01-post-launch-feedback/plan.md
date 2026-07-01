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
- [ ] Owner-only checks: in-app update 0.1.0 → 0.1.1 via Settings → Check now (the first real
      updater run), a real log-out/in with the toggle on (incl. any Dock-icon flash at login)

### 3. Login Items should say UsageOS, not the signing certificate (D69)
_Source: owner, first run of Start at login on v0.1.1 — macOS attributed the background item to
the signing certificate's name instead of the app; reads unofficial._

- [x] Register via `SMAppService.agent` with the agent plist bundled inside the app
      (`bundle.macOS.files` → `Contents/Library/LaunchAgents/`)
- [x] New `login_item` module (cfg-isolated `objc2-service-management` surface); the
      `get/set_launch_at_login` commands and the whole frontend unchanged
- [x] `tauri-plugin-autostart` removed
- [x] Startup migration removes the legacy bare `~/Library/LaunchAgents` plist and re-registers
- [x] Duplicate-instance guard: launchd starts the agent at registration time (and login can race
      window restore) — a `--hidden` launch that finds UsageOS already running exits immediately
      (verified via `launchctl kickstart`: duplicate exits 0)
- [x] Bundled-build verification on a Developer-ID-signed build: toggle registers/unregisters,
      **Login Items shows "UsageOS" with its icon**, migration removes the legacy plist, no bare
      plist ever appears. (SMAppService refuses ad-hoc-signed bundles — sign before verifying.)
- [ ] `/usageos-review` + PR

## Backlog (unclaimed feedback)

_(add items here as reports come in)_
