# Review — menubar-agent (Dock icon follows the window + start at login)

**Date:** 2026-07-01 · **Scope:** branch (`main...feat/menubar-agent`) · **Files:** 16 (+504/−24 pre-fix)
**Plan:** [plan.md](../plan.md) · **Impl-plan:** [2026-07-01-menubar-agent.md](../impl-plans/2026-07-01-menubar-agent.md)
**Codex:** ran (codex exec, read-only, structured output)

## Merge gates

| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy --all-targets --all-features -D warnings | ✅ |
| cargo test | ✅ 127 passed |
| tsc --noEmit | ✅ |
| vitest | ✅ 36 passed (12 files) |
| bindings fresh (`cargo test export_bindings` + clean diff) | ✅ |

Re-run after the review fixes below; all green.

## Hard-rules gate (Lane A) — all 8 PASS

Privacy verified into the vendored deps: `tauri-plugin-autostart` 2.5.1 / `auto-launch` 0.5.0 in
LaunchAgent mode only create/remove/stat a plist under `~/Library/LaunchAgents` — no network
anywhere (the `osascript` path is the AppleScript branch, dead code for us). New commands are in
`collect_commands!`; bindings regenerated, freshness gate clean. No unwrap/expect/panic in new
prod paths. No SQL in the diff. No objc2 outside the sanctioned modules (the policy switch uses
tauri's own API). `capabilities/default.json` untouched — the plugin's own webview commands are
ACL-denied since no `autostart:*` permission was granted; the only IPC path is our specta commands.
New UI reuses `SettingGroup`/`SettingRow`/`Toggle` and the onboarding `Why`/`GrantBox` primitives —
no ad-hoc tokens.

## Findings

**Verification:** 14 verified · 0 dropped · 1 cross-model confirmed (Codex + Lane A on the same
finding-class)

### Critical
None.

### Warnings (all fixed on the branch)

- `src-tauri/src/lib.rs:704` — **[B]** `restart_app` re-execs with the original argv, so an update
  installed in a login-launched (`--hidden`) session relaunched the app *invisible*. Fixed:
  `restart_app` sets `USAGEOS_SHOW_AFTER_RESTART=1` (inherited by the re-execed child); the setup
  hook ignores `--hidden` when it's present.
- `src/components/settings/BackgroundSettings.tsx` + `Onboarding.tsx` — **[D + A · cross-model
  confirmed]** the optimistic toggle swallowed a failed LaunchAgent write, showing "on" when the
  system state was "off" — contradicting D68's the-system-is-the-source-of-truth model. Fixed:
  revert state on rejection (both components) + a regression test.
- `src-tauri/src/lib.rs` (run-loop comment) — **[C]** comment stated an inverted lint rationale for
  the `_`-prefixed closure params. Fixed.
- `src/components/onboarding/Onboarding.tsx` — **[C]** eyebrow "Always on" contradicted the
  "Off by default" bullet on the same card. Fixed → "In your menu bar".
- `src/components/onboarding/Onboarding.tsx` — **[C]** GrantBox sub duplicated the Updates step's
  sub verbatim one screen later. Fixed → "Adds UsageOS to your Login Items." (mechanical-transparency
  idiom, Apple's own terminology).

### Info (fixed)

- Bullet 3 varied so Background/Updates steps don't read copy-pasted **[C]**.
- Settings row description tightened; dropped the clause describing hide-on-close (not what the
  toggle controls) **[C]**.
- Onboarding state renamed `[startAtLogin, setStartAtLogin]` (kills the `State`-suffix collision
  workaround) **[C]**.
- `BackgroundSettings` import alphabetized in SettingsView **[C]**.
- Cargo.toml comment trimmed to the one non-obvious fact (no capability entry needed) — the
  LaunchAgent details live at the plugin-init site **[C]**.
- Impl-plan corrected: the plist is `UsageOS.plist` (app name), not `<bundle-id>.plist` **[A]**.
- SettingsView test now asserts the Background group renders **[B]**.

### Info (no action, recorded)

- `auto-launch` 0.5.0 has an internal `home_dir().unwrap()` — third-party, unreachable in a real
  macOS session **[A]**.
- `--hidden` launch + tray-setup failure leaves the app Accessory/windowless/trayless, but fully
  recoverable: Finder/Spotlight reopen fires `RunEvent::Reopen` → window shows, and with
  `TRAY_READY` false a close is a real quit **[B]**.
- Possible sub-second Dock-icon flash at login (policy applied at `RunEvent::Ready`) — check on a
  bundled build during the next release smoke test; not fixable in code without `LSUIElement`
  (rejected in D68) **[B]**.

## Auto-fixes applied

All "fixed" items above (one commit on the branch), gates re-run green afterwards. No network,
unsafe, migration, or IPC-shape code was auto-touched; the two logic fixes (restart env-var,
toggle revert) were verified against tauri 2.9.3 / plugin sources and covered by tests where
testable.

## Manual TODO

- [ ] Release smoke test (bundled build): Finder/Spotlight reopen, login-item end-to-end
      (log out/in), Dock-flash-at-login check, updater-restart-shows-window check.

## Definition of Done

- [x] plan.md ticked for what landed
- [x] decisions.md D68 appended
- [x] impl-plan present · handoff to follow at session end
- [x] docs move with code (pre-push tripwire would not fire)

## Plan compliance

Alignment: **good** — the diff matches the impl-plan section-for-section (policy helper, close
handler, Reopen, autostart plugin/commands, `--hidden` flow, Settings group, onboarding step,
tests, docs). One deliberate deviation: no DB settings row (that *was* the plan); one addition
beyond plan: the restart env-var override, prompted by this review.

## On-device verification (dev build, this session)

Foreground→close→UIElement (Dock icon gone, tracking alive) → glance "Open UsageOS" →
Foreground + focused, repeated cleanly. `--hidden` launch: UIElement + zero windows from the
start. Settings toggle: `~/Library/LaunchAgents/UsageOS.plist` created with `--hidden` +
`RunAtLoad`, removed on toggle-off. Glance opens/toggles correctly under Accessory.
