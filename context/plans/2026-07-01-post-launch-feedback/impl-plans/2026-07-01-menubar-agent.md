# Menu-bar agent: hide the Dock icon when closed + start at login

## Context

First tester feedback after launch: *"I wish the app didn't have to be open — just the menu bar"* and *"I've closed it but it's still showing on my Dock"*. Half of this already works — closing the main window hides it and tracking keeps running behind the tray ([lib.rs:720–734](src-tauri/src/lib.rs)) — but the app keeps macOS's default **Regular** activation policy, so a Dock icon lingers with no window behind it (and clicking it does nothing). And the app still has to be launched once per login for tracking to happen at all.

Two features, one PR:
1. **Dynamic Dock icon** — Dock icon while the dashboard is open; close it → app switches to the **Accessory** activation policy and lives purely in the menu bar (like Rectangle/Stats). Reopen from the tray → icon returns.
2. **Start at login** — opt-in (default OFF, consistent with D67), via `tauri-plugin-autostart`, launching hidden into menu-bar mode. Settings toggle + a recommended onboarding step.

User decisions (asked & answered): dynamic Dock behavior (not always-hidden) · onboarding step + Settings toggle · docs filed in a **new plan folder** `context/plans/2026-07-01-post-launch-feedback/`.

**Verified against tauri 2.9.3 source** (the version the `=2.9.0` updater pin holds us to):
- `AppHandle::set_activation_policy(ActivationPolicy)` exists at runtime (`tauri-2.9.3/src/app.rs:600`).
- `RunEvent::Reopen` exists (`app.rs:251`) — fires when the user opens the already-running app from Finder/Spotlight.
- `tauri-plugin-autostart` 2.5.1 requires `tauri ^2.8.2` → compatible with 2.9.3 (checked crates.io index).

## Backend (src-tauri)

### 1. Dynamic activation policy — `src-tauri/src/lib.rs`
- Add a small helper (near `show_main`, ~line 485):
  ```rust
  /// Regular = Dock icon + Cmd-Tab while the dashboard is open; Accessory = menu-bar only.
  fn set_dock_visible(app: &AppHandle, visible: bool) {
      #[cfg(target_os = "macos")]
      {
          use tauri::ActivationPolicy;
          let policy = if visible { ActivationPolicy::Regular } else { ActivationPolicy::Accessory };
          if let Err(e) = app.set_activation_policy(policy) { eprintln!("[Dock] {e}"); }
      }
  }
  ```
  Errors logged, never unwrapped (hard rule 3). Order matters: **Regular before show/focus** (else the window can appear unfocused), **Accessory after hide**.
- `show_main()` (lib.rs:485): call `set_dock_visible(app, true)` first, then the existing show/unminimize/set_focus.
- `CloseRequested` handler (lib.rs:726–733): after `window.hide()`, call `set_dock_visible(&app_handle, false)` (get the handle from `window.app_handle()`).
- Minimize/Cmd-H are untouched — only close (X) goes to menu-bar mode, matching normal macOS expectations.
- **Reopen handling**: with no Dock icon, a user who forgets it's running will open it from Applications/Spotlight — macOS activates the existing process and fires `RunEvent::Reopen`, which today would do nothing visible. Change the tail of `run()` (lib.rs:786) from `.run(tauri::generate_context!())` to:
  ```rust
  .build(tauri::generate_context!())  // then run with an event closure
  ```
  and handle `RunEvent::Reopen { .. } => show_main(app_handle)`. (`.build()` returns `Result<App>`; keep the existing fatal-error print/exit shape.)

### 2. Launch at login — plugin + commands
- `src-tauri/Cargo.toml`: add `tauri-plugin-autostart = "2"`. After `cargo update`, **verify `cargo tree | grep "tauri "` still shows 2.9.3** — the ecosystem's caret deps have dragged tauri toward 2.10 before (handoff 2026-06-30-02 §7); pin exact (`=2.5.1`) only if the resolver misbehaves, same treatment as the shell/updater pins.
- Register in the builder chain (lib.rs:709ff):
  ```rust
  .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec!["--hidden"])))
  ```
  LaunchAgent mode writes `~/Library/LaunchAgents/<bundle-id>.plist` — shows up under System Settings → General → Login Items as a background item; no Automation/AppleScript permission involved.
- Two new commands next to the settings commands (lib.rs ~315), using the plugin's `ManagerExt`:
  - `get_launch_at_login(app: AppHandle) -> Result<bool, AppError>` → `app.autolaunch().is_enabled()`
  - `set_launch_at_login(app: AppHandle, enabled: bool) -> Result<(), AppError>` → `enable()`/`disable()`
  - **No DB settings row** — the system LaunchAgent state is the single source of truth (nothing to drift). Map the plugin error into a new `AppError` variant (see `error.rs` for the existing pattern).
- Add both to `collect_commands!` in `make_builder()` (lib.rs:664–699); regenerate bindings with `cargo test export_bindings`.
- **No capability changes** — `capabilities/default.json` gates webview→plugin invokes; we only call the plugin from Rust behind our own specta commands.

### 3. Hidden launch (`--hidden`)
- `src-tauri/tauri.conf.json`: main window gets `"visible": false`.
- In `setup` (first thing, before the heavier init):
  ```rust
  if std::env::args().any(|a| a == "--hidden") {
      set_dock_visible(app.handle(), false);   // menu-bar only from the start
  } else {
      show_main(app.handle());                  // normal launch: show as before
  }
  ```
  Login launch → no window, no Dock icon (a sub-second Dock blink while the policy flips is acceptable; the `LSUIElement` Info.plist route would remove it but complicates dev/normal launches — rejected, note in the ADR).

## Frontend (src)

### 4. IPC wrappers — `src/lib/tauri.ts`
`getLaunchAtLogin(): Promise<boolean>` / `setLaunchAtLogin(on: boolean): Promise<void>` following the existing `unwrap(await commands...)` pattern (tauri.ts:158–164, 230–242).

### 5. Settings toggle — `src/views/SettingsView.tsx`
New `SettingGroup` (suggested title: **"Background"**, placed before "Software update") with one `SettingRow` + `Toggle`, exactly the `UpdateSettings.tsx:19–31,66–71` pattern: `useState` + `useEffect`-on-mount reading `getLaunchAtLogin()`, optimistic toggle calling `setLaunchAtLogin`. Draft copy (match the Settings voice — plain, ownership-focused; final wording at impl time):
- Label: **"Start at login"**
- Description: *"UsageOS starts quietly in the menu bar when you log in — your day is tracked without you having to remember to open anything. Close the window anytime; tracking keeps going."*

### 6. Onboarding step — `src/components/onboarding/Onboarding.tsx`
Insert a step between **Automation** and **Updates**: `STEPS = [... "Automation", "Background", "Updates", "Ready"]`. Reuse the existing `Eyebrow`/`H`/`Why`/`GrantBox` pieces (same shape as the Updates step, lines 167–191):
- Eyebrow "Always on" · H "Runs quietly in the background"
- `Why` bullets (draft): lives in your menu bar, close the window and tracking keeps going · start at login means the day is tracked from the moment you sit down · off by default, change anytime in Settings.
- `GrantBox` label "Start at login", sub "Recommended. Change anytime in Settings.", `grantLabel="Enable"` → `setLaunchAtLogin(true)`.
- Footer: `Continue →` when enabled, ghost `Not now` otherwise (mirror step 4's pattern, lines 238–243).

## Tests

- **TS (vitest + RTL):** update `Onboarding.test.tsx` (step count/labels, new step's Enable path with `@/lib/tauri` mocked); add a `SettingsView.test.tsx` case for the new toggle (mock `getLaunchAtLogin`/`setLaunchAtLogin`, assert toggle renders and calls the setter) — follow the existing mock pattern (SettingsView.test.tsx:1–87).
- **Rust:** the new code is thin glue over macOS UI APIs — no meaningful unit surface; the `export_bindings` test doubles as the command-registration check. Keep helpers small so this stays true.

## Docs (lockstep — part of the change)

- Create `context/plans/2026-07-01-post-launch-feedback/` with `plan.md` (roadmap: this feature as item 1, room for future tester feedback), `impl-plans/`, `handoffs/`, `reviews/`; register it as **active** in `context/plans/README.md`.
- Save this plan as `impl-plans/2026-07-01-menubar-agent.md` once approved.
- Append ADR **D68** to `context/decisions.md`: dynamic activation policy (Regular↔Accessory tied to main-window visibility) + opt-in launch-at-login via LaunchAgent; system state as source of truth (no DB mirror); `LSUIElement` rejected (dev/normal-launch complications); default OFF per the D67 opt-in philosophy.
- Handoff entry at session end.

## Order of work

Branch `feat/menubar-agent` off `main` → backend §1–3 → `cargo test export_bindings` (regen bindings) → frontend §4–6 → tests → docs → `/usageos-review` → PR.

## Verification

1. **Gates:** `cargo clippy -D warnings` · `cargo fmt --check` · `cargo test` · `tsc` · `npx vitest run` · bindings freshness (regen + git diff clean).
2. **Dock behavior (`npm run tauri dev`):** launch → window + Dock icon as today → close window → Dock icon disappears, tray icon remains → tray → Open UsageOS → Dock icon returns, window frontmost and focused → repeat a few cycles (policy flapping is the known quirk to catch) → glance popover still opens/auto-hides correctly in both policies.
3. **Reopen:** with the window closed (no Dock icon), `open -a UsageOS` (or Spotlight) → window reappears.
4. **Start at login:** toggle ON in Settings → `~/Library/LaunchAgents/com.usageos.app.plist` exists and System Settings → Login Items shows UsageOS; toggle OFF → gone. Full end-to-end (log out/in → app in menu bar, no window) is best done with a bundled build — **note:** enabling from `tauri dev` registers the *dev binary* path in the plist; toggle off before ending the dev session.
5. **Onboarding:** clear the onboarding-done flag (or fresh profile) → walk the flow → new Background step renders, Enable works, skippable.
