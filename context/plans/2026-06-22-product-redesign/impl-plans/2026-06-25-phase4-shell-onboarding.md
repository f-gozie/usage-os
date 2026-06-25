# Phase 4 — Shell & onboarding (menubar + first-run permission priming)

_Branch `phase4/shell-onboarding`. Approved plan-mode plan + as-built. Pairs with the review
`reviews/2026-06-25-phase4-shell-onboarding.md` (after on-device verification) and ADR **D55**._

**Goal:** make UsageOS installable as a real product — a first-run experience that explains +
requests the two macOS capture permissions, a menubar tray with a quick-glance popover, and the
app running in the background when the window is closed (quit only from the tray). One PR.

**Decisions taken (owner, in plan mode):** menubar = **tray + hide-on-close** (keep the Dock
icon; closing hides + keeps tracking); ship **everything in one PR**; the tray quick-glance uses
a **share donut** (not the time-positioned Day dial — it crowds at small size / long days, shown
in the `design/menubar.html` busy-15h stress test the owner reviewed).

## As-built (landed 2026-06-25)

- **Chunk 1 — Permission seam** (`src-tauri/src/permissions/`, cfg-gated + non-macOS stub):
  - `Permissions { accessibility: bool, automation: PermissionState }`; commands `get_permissions`,
    `request_accessibility`, `request_automation`, `open_settings_pane` (registered; bindings regen).
  - **Accessibility** via `AXIsProcessTrusted` / `…WithOptions`. **Automation** via
    `AEDeterminePermissionToAutomateTarget` — the only API that reads the TCC-Automation grant
    **without sending an Apple Event** (`ask_user=false` queries silently; `true` prompts without
    launching the target), aggregated over the Chromium bundle ids. Raw `extern "C"` FFI linked via
    `CoreServices` (compiles + links on macOS; **runtime verified on-device**). The pure
    `aggregate_automation` reduction is unit-tested without the FFI.
  - `open_settings` deep-links the Privacy panes via `/usr/bin/open x-apple.systempreferences:…`.
  - Capture's AX prompt was **factored into the seam** (single source; `capture/macos/mod.rs` now
    calls `permissions::{accessibility_trusted, prompt_accessibility_trust}`).
- **Chunk 2 — Onboarding** (`src/components/onboarding/Onboarding.tsx`, from `design/onboarding.html`):
  5 steps (Welcome → Privacy → Accessibility → Automation → Ready), every grant skippable → a
  degraded Ready. Live status from **`usePermissions`** (re-reads on window focus, so a grant flips
  to "Granted ✓" on return from System Settings). First-run gate in `App.tsx` via an
  `onboarding_completed` setting (fail-open to the app on a read error). Re-grantable from a new
  **Settings → Permissions** group. RTL tests for the flow.
- **Chunk 3 — Tray + glance** (`lib.rs` `setup_tray`/`toggle_glance`; `src/components/glance/Glance.tsx`):
  core `TrayIconBuilder` (Dock icon retained); **left-click** toggles a borderless, always-on-top
  `glance` webview at `#/glance` (positioned under the tray icon, hidden on focus-loss) rendering a
  **share-donut + Active/Top/Focus + top-3**, fed by `getDay` for today (numbers in Rust). **Right-click**
  menu = Open / Quit. Main-window `CloseRequested` → `prevent_close` + `hide` (tracking continues);
  app exits only via tray Quit. New commands `show_main_window` / `quit_app`; `glance` added to the
  default capability.
- **Chunk 4 — Polish:** **CSP** set from `null` to a strict `default-src 'self'; … connect-src 'self'
  ipc: http://ipc.localhost; object-src 'none'; frame-src 'none'` (defense-in-depth for hard rule 1).
  Dark-mode parity (token-only components) + idle-CPU (glance/permissions poll on focus, not on a
  timer) — verify on-device.

**Build surprises:** the tray event `rect` type is `tauri::Rect` (crate-root re-export), **not**
`tauri::tray::Rect` (which resolves to a private `tauri_runtime::dpi::Rect` — E0603). `tray-icon`
feature added to `tauri` in Cargo.toml (no Cargo.lock change — the crate was already locked).

**Gates:** 125 Rust + 32 TS tests, `clippy -D warnings`, `cargo fmt`, `tsc`, `vitest`, bindings
fresh — all green in CI (Linux + macOS-compile).

## On-device verification (Favour's Mac — REQUIRED before /usageos-review + PR)
1. Fresh state → onboarding launches; Accessibility "Grant" opens the right pane + pill flips to
   Granted ✓ on return; Automation "Grant" prompts the running browser(s); Skip → degraded Ready;
   "Open my day" completes and never re-shows.
2. Settings → Permissions reflects + re-requests correctly.
3. Tray icon appears; left-click opens the glance with correct today numbers; "Open UsageOS" shows
   the window; **closing the window keeps tracking** (new events recorded while hidden); **Quit (tray)
   exits**; Cmd-Q quits.
4. Strict CSP — app + glance render (no white-screen); fonts + icons load.
5. Dark-mode parity across all views + onboarding + glance; idle CPU ~0%.

## Deferred
Day-start offset (D14); the evening "your day is ready" ping (Phase 3 deferral). Tray-popover
positioning across multiple displays may need tuning (settled on-device).

## As-built revision — glance re-architected to NSPanel (D56, via `/debate`)
On-device, the D55 glance (a transparent `WebviewWindow`) failed three ways: didn't float over
full-screen apps, transparency/rounding halo, donut center-text overflow. A two-round `/debate`
(Codex vs Opus) converged: the window **primitive** was wrong. Redone as a **non-activating
`NSPanel`** (in-repo objc2 reclass — `src/glance_panel.rs`) at `NSPopUpMenuWindowLevel` with
`CanJoinAllSpaces | CanJoinAllApplications | FullScreenAuxiliary | Transient | IgnoresCycle`, no
`set_focus()`; native rounded/frosted chrome via tauri's built-in `set_effects(Effect::Popover,
radius)` (no new dep — it wraps window-vibrancy); CSS card removed; donut center text sizes off
string length; positioning clamps to the tray's display. Full rationale: **D56**. Still on-device-
gated (the float-over-fullscreen + frosted render must be confirmed on Favour's Mac).
