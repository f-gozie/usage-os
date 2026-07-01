# Handoff — 2026-07-01-02 · PR #36 merged; bundled-build smoke test passed

## 1. Current state
- **[PR #36](https://github.com/f-gozie/usage-os/pull/36) is MERGED to main** (owner-authorized,
  REST merge per the Projects-classic workaround). Branch `feat/menubar-agent` deleted.
- **Bundled-build smoke test done** on a local release build (`tauri build --bundles app`,
  unsigned — the signing error at the end is only the updater artifact, harmless locally):
  - LaunchServices launch → window shows, Foreground.
  - Close → UIElement (no Dock icon), tracking alive.
  - `open` the bundle again → **no second instance; `RunEvent::Reopen` fired → window back,
    focused, Foreground.** (The one path dev couldn't test.)
  - Settings → Background renders with the final copy; toggle ON wrote
    `~/Library/LaunchAgents/UsageOS.plist` pointing at the bundle binary + `--hidden`;
    toggle OFF removed it. Toggled off and test instance quit — nothing left behind.
  - Release binary `--hidden` → UIElement + zero windows.
- The owner's installed v0.1.0 kept tracking throughout (untouched).

## 2. Remaining (owner-only)
1. A real **log-out/in** with Start at login enabled (also eyeball a possible sub-second
   Dock-icon flash at login — accepted D68 trade-off, just confirm it's subtle).
2. The **updater-restart** path (`USAGEOS_SHOW_AFTER_RESTART`) — first testable when a
   v0.1.1 release exists and a 0.1.0 install updates to it.
3. Reply to the tester once these ship in a release build.

## 3. Gotchas (this session's additions)
- **macOS 26 "click wallpaper to reveal desktop" was the phantom:** a stray automation click
  left desktop-reveal active for ~40 min and made two LS-launched instances look broken
  (AX reported 0 windows; one instance even ended up UIElement). Once cleared (click the edge
  tile), the same launches behaved perfectly. If window checks look impossible, clear the
  desktop state FIRST.
- `open -n <bundle>` launches a second instance alongside the installed app (different bundle
  path, same id); plain `open <bundle>` on an already-running bundle activates it (Reopen).
- The release webview's AX tree came back empty via `entire contents` where the dev build's
  didn't — real clicks (computer-use) are the dependable route for bundled-app UI.
