# Login item is branded "UsageOS" everywhere (SMAppService migration)

## Context

Dogfooding v0.1.1 surfaced a branding problem with **Start at login**: macOS's
background-activity notification and System Settings → Login Items attribute the item to the
*signing certificate's name* (a personal Developer ID) instead of to UsageOS — it reads as
unofficial. Cause: `tauri-plugin-autostart` registers the login item the legacy way (a bare
plist in `~/Library/LaunchAgents`), which macOS can't attribute to an app, so it falls back to
naming the certificate holder.

**Fix:** register via **`SMAppService.agent`** (macOS 13+ — matches our minimum) with the agent
plist bundled *inside* UsageOS.app. macOS then attributes the item to the app: the notification
and Login Items both say **UsageOS** (with icon). Certificate details remain visible only to
deliberate signature inspection, which is fine and unavoidable. The user-facing toggle, IPC
signatures, and frontend do not change at all.

**Docs rule for this task:** the repo is public — every artifact (ADR, plan entries, handoff,
PR description, commit messages) describes this as *certificate-attribution / branding*,
generically. Never quote the notification text or any personal name. **First action after
approval:** the uncommitted plan.md backlog entry currently in the working tree quotes it —
rewrite it generically before anything is committed.

**Verified before planning (no unknowns left):**
- `objc2-service-management` 0.3.2 exposes `SMAppService`: `agentServiceWithPlistName(&NSString)`,
  `registerAndReturnError()/unregisterAndReturnError() -> Result<(), Retained<NSError>>`,
  `status() -> SMAppServiceStatus` (NotRegistered/Enabled/RequiresApproval/NotFound). Needs
  objc2 ≥0.6.2 + objc2-foundation ^0.3.2 — both already in the tree at compatible versions.
- tauri 2.9.3 `bundle.macOS.files` copies extra files into the .app **relative to Contents/**
  (`tauri-utils-2.9.3/src/config.rs:613ff`).
- A bundled agent plist may combine `BundleProgram` (bundle-root-relative exe path) with
  `ProgramArguments` (argv incl. our `--hidden`) — the Mozilla VPN pattern.

## Changes

### 1. Bundled agent plist — new `src-tauri/agents/com.usageos.app.agent.plist`
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.usageos.app.agent</string>
  <key>BundleProgram</key><string>Contents/MacOS/usage-os</string>
  <key>ProgramArguments</key>
  <array><string>usage-os</string><string>--hidden</string></array>
  <key>RunAtLoad</key><true/>
</dict>
</plist>
```
(Label must match the plist filename minus extension — SMAppService convention.)

`src-tauri/tauri.conf.json` → `bundle.macOS.files`:
`{"Library/LaunchAgents/com.usageos.app.agent.plist": "agents/com.usageos.app.agent.plist"}`.
The existing `--hidden` handling in the setup hook (incl. the `USAGEOS_SHOW_AFTER_RESTART`
override) is untouched — the arg now comes from this static plist.

### 2. New native module — `src-tauri/src/login_item.rs`
Follows the project's native-isolation pattern (hard rule 5; like `glance_panel.rs`):
- `#[cfg(target_os = "macos")]` impl using `SMAppService` (unsafe calls get the project's
  standard safety comments; errors map to `String` → `AppError::Autostart`):
  - `is_enabled() -> Result<bool, String>` — `status() == Enabled`.
  - `set_enabled(on: bool) -> Result<(), String>` — `registerAndReturnError` /
    `unregisterAndReturnError`. A `RequiresApproval` outcome or NSError surfaces as `Err` —
    the frontend toggle already reverts on failure (built in the v0.1.1 review pass).
  - `migrate_legacy()` — if `~/Library/LaunchAgents/UsageOS.plist` exists (pre-0.1.2 installs:
    the owner + possibly the tester): delete it and `set_enabled(true)` best-effort, `eprintln!`
    on failure. Called once from the setup hook, macOS only.
- `#[cfg(not(target_os = "macos"))]` stub (`Ok(false)` / `Err("macOS only")`) so Linux CI compiles.

### 3. Swap the commands' backend — `src-tauri/src/lib.rs`
- `get_launch_at_login` / `set_launch_at_login` keep their exact signatures (bindings unchanged →
  freshness gate proves it); bodies call `login_item::` instead of `app.autolaunch()`.
- Remove the `tauri_plugin_autostart` plugin registration + `ManagerExt` import.
- Call `login_item::migrate_legacy()` from setup (after tray setup; non-critical path).

### 4. Dependencies — `src-tauri/Cargo.toml`
- Add under the macOS target table: `objc2-service-management = { version = "0.3.2", features = ["SMAppService"] }`
  (same pattern as the other objc2-* deps; add the crate's foundation/objc2 meta-features only if
  the compile asks).
- Remove `tauri-plugin-autostart` (drops 6 packages from the lock).

### 5. Frontend
**No changes.** `BackgroundSettings.tsx`, `Onboarding.tsx`, `tauri.ts`, and all tests stay as-is.
Behavior change worth knowing: in `tauri dev` (unbundled binary) SMAppService registration fails →
the toggle now reverts with an error instead of silently registering the dev binary — this
*removes* the known dev footgun from handoff -01.

### 6. Docs (lockstep)
- ADR **D69**: SMAppService over legacy LaunchAgent so login-item surfaces carry the app's own
  branding; plugin removed; legacy plist migration; wording kept generic per the docs rule above.
- Plan folder `2026-07-01-post-launch-feedback/`: add as item 3 (replaces the backlog note);
  save this plan as `impl-plans/2026-07-02-smappservice-login-item.md`; handoff at session end.
- Note in D69/plan: the `USAGEOS_SHOW_AFTER_RESTART` and Reopen behavior are unaffected.

## Order of work
Branch `feat/smappservice-login-item` off main → plist + conf → `login_item.rs` → lib.rs swap →
deps → gates (`cargo test export_bindings` must produce **no** bindings diff) → docs →
`/usageos-review` → PR. (PR #37, the About version fix, is separate and still awaiting merge.)

## Verification
1. **Gates:** fmt · clippy (all targets/features) · cargo test · tsc · vitest · bindings fresh
   (expect *unchanged* bindings).
2. **Dev negative path:** `tauri dev` → toggle ON → errors → toggle visibly reverts (the
   regression test from v0.1.1 covers the component; confirm live once).
3. **Bundled:** `tauri build --bundles app` → verify the plist landed at
   `UsageOS.app/Contents/Library/LaunchAgents/`; launch, toggle ON → `launchctl print
   gui/$UID/com.usageos.app.agent` shows the agent; **System Settings → Login Items shows
   "UsageOS" with its icon** (the point of the change); nothing appears in
   `~/Library/LaunchAgents`. Toggle OFF → unregistered. If ad-hoc signing trips SMAppService,
   re-verify with a `release-macos.sh` signed build before concluding anything.
4. **Migration:** plant a legacy `~/Library/LaunchAgents/UsageOS.plist`, launch the new build →
   file removed, agent registered.
5. **Owner:** log out/in → app starts hidden in the menu bar; the background-items notification
   (if macOS re-shows one) now names UsageOS.
6. Ships in the next release (v0.1.2) together with PR #37's About fix.
