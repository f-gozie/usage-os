# Handoff — 2026-07-01-01 · Menu-bar agent built, reviewed, PR #36 open

## 1. Current state
- **New plan started:** `2026-07-01-post-launch-feedback/` (registered active) — the rolling home
  for feedback-driven polish now that v0.1.0 is public. Item 1 is done pending merge.
- **[PR #36](https://github.com/f-gozie/usage-os/pull/36) is open** on `feat/menubar-agent`
  (2 commits: feature + review fixes). All gates green (fmt · clippy all-targets/all-features ·
  127 Rust · tsc · 36 vitest · bindings fresh). Source: first tester feedback (WhatsApp) — dead
  Dock icon after close + "wish it just ran from the menu bar".

## 2. What landed (D68)
- **Dock icon follows the window:** `set_dock_visible` (lib.rs) flips ActivationPolicy —
  Regular *before* show (else the window arrives unfocused), Accessory *after* hide in the
  CloseRequested handler. Run-loop is now `.build().run(closure)` with
  `RunEvent::Reopen → show_main` (macOS-only variant — the cfg gate keeps Linux CI green).
- **Start at login (opt-in, default OFF):** `tauri-plugin-autostart` 2.5.1 (LaunchAgent mode,
  `--hidden` arg; resolves cleanly against the pinned tauri 2.9.3 — no version pin needed).
  Commands `get/set_launch_at_login`; **no settings row — the LaunchAgent is the source of
  truth.** Main window is `visible: false` in config; the setup hook shows it unless `--hidden`.
  Plist is `~/Library/LaunchAgents/UsageOS.plist` (app name, not bundle id).
- **UI:** Settings → new "Background" group (`BackgroundSettings.tsx`, mirrors UpdateSettings);
  onboarding gained a "Background" step between Automation and Updates (STEPS is 7 now).
- **Docs:** D68 appended; plan folder + impl-plan + review report all in the PR.

## 3. Review (the /usageos-review pass paid for itself)
Report: `reviews/2026-07-01-menubar-agent.md`. All 8 hard rules pass (privacy verified into the
vendored plugin sources). 5 warnings found → fixed:
- **Updater restart relaunched hidden** in a login-launched session (argv keeps `--hidden` across
  `app.restart()`): `restart_app` now sets `USAGEOS_SHOW_AFTER_RESTART=1`, setup honors it.
- **Toggle now reverts on a failed LaunchAgent write** (Codex + Lane A cross-model confirmed;
  regression test added) — matches the source-of-truth model.
- Copy: onboarding eyebrow "Always on" → "In your menu bar" (clashed with "Off by default");
  GrantBox sub → "Adds UsageOS to your Login Items."; Settings description tightened.

## 4. Verified on-device (dev build, this session)
Foreground → close → UIElement (no Dock icon, tracking alive) → glance Open → Foreground +
focused, cycled repeatedly (checked via `lsappinfo -only ApplicationType`). `--hidden` launch:
UIElement + zero windows. Toggle on/off: plist created with `--hidden` + `RunAtLoad`, removed
cleanly.

## 5. FIRST for the next session
1. **Merge PR #36** (owner). Remember the Projects-classic bug: merge via
   `gh api -X PUT repos/f-gozie/usage-os/pulls/36/merge -f merge_method=merge`.
2. **Release smoke test on the next bundled build:** Finder/Spotlight reopen while Dock-less;
   a real log-out/in with the toggle on; updater-restart shows the window; check for a
   sub-second Dock flash at login (accepted trade-off vs LSUIElement — D68).
3. The tester's feedback is fully addressed once merged + shipped — worth a reply with the build.

## 6. Gotchas (learned this session)
- **`tauri dev` + Start at login registers the DEV binary path** in the plist — always toggle it
  off before ending a dev session (this session did).
- **Verifying tray/policy behavior programmatically:** synthetic AX clicks (AppleScript
  `click`/`AXPress`) do NOT dispatch tauri tray-icon events — use real mouse clicks
  (computer-use). But AX **does** work for buttons *inside* the webviews (glance "Open UsageOS",
  the Settings toggle) — `entire contents of window 1` → click by role/name. The glance panel is
  invisible to screenshots (level-101 reclassed NSPanel) while AX still sees it.
  `lsappinfo info -only ApplicationType pid=N` is the ground truth for the policy
  (Foreground/UIElement).
- **macOS 26 "click wallpaper to reveal desktop"** can eat automation clicks that miss a window
  and shove every window into edge tiles — re-click the tile to restore.
