# Standard: macOS capture + permissions

_Phase 0 standard. Scope: how UsageOS reads what you're doing (active app, window title, browser URL, idle) and how it asks for the macOS permissions that gate that — Accessibility (AX titles) and Automation (Apple Events for URLs), never Screen Recording._

> **Status: PROVISIONAL.** This document is grounded in research that had **no independent verification pass**. Confirmed claims carry a pinned version + citation. Anything not confirmed by a cited authoritative source is parked under [Open questions / verify in the Phase-0 spike](#️-open-questions--verify-in-the-phase-0-spike) — do not treat those as settled. The spike (D22) replaces the provisional bits with measured fact; update this file when it lands.

---

## Why this standard exists (the one thing to remember)

The product's whole promise is local-only. Capture is where that promise is kept or broken. Two hard rules from `CLAUDE.md` govern everything below:

> **Hard rule #1 — Nothing leaves the machine.** No network calls in the data path, ever. This is auditable in the open source and is the product's whole promise.

> **Hard rule #5 — The native + AI surface stays minimal and isolated.** Capture lives behind a `capture` trait [...]. Both must be mockable so the rest of the app is testable without macOS permissions or a model.

Apple Events (AppleScript) are **local IPC, not network** — they are compliant with rule #1. But the URLs and titles they yield are sensitive, so the privacy bar (incognito exclusion, the exclusion list) is the same as for everything else.

The single most important capture fact:

> **Window titles come from the Accessibility (AX) API, gated ONLY by Accessibility permission — NOT Screen Recording.** The empty titles in v0.1.0 are because `active-win-pos-rs` reads `kCGWindowName` via `CGWindowListCopyWindowInfo`, and that field is gated by Screen Recording on modern macOS. **The AX rewrite must never regress into any `CGWindowList` title path**, or it re-acquires the permission we are specifically avoiding. [tcc-permissions.json, capture-ax-titles.json]

---

## The permission map (what gates what)

| Capability | macOS permission (TCC service) | API that triggers the prompt | If denied → degrade to |
|---|---|---|---|
| Active app name + switch events | **none** | `NSWorkspace` activation notifications | — (always works) |
| Idle time | **none** | `user-idle` (CoreGraphics idle read) | — (always works) |
| Window **title** | **Accessibility** (`kTCCServiceAccessibility`) | `AXIsProcessTrustedWithOptions({prompt:true})` | app name only, `title = null` |
| Browser **URL** | **Automation** (`kTCCServiceAppleEvents`), **per (UsageOS, browser) pair** | `AEDeterminePermissionToAutomateTarget` / first send | title-derived site, then app-only |
| ~~Window title via CGWindowList~~ | ~~Screen Recording~~ | — | **NEVER USE — forbidden path** |

Rationale: app name + idle need no grant, so the degraded mode (rule: graceful degradation per D21) is always a real product, not a dead screen. [tcc-permissions.json]

---

## Conventions

### C1. Title source is the AX API, never CGWindowList
**One-line rationale:** AX titles need only Accessibility; `kCGWindowName` needs Screen Recording — the toxic permission we refuse.

Read chain: `AXUIElementCreateApplication(pid)` → copy attribute `"AXFocusedWindow"` → copy attribute `"AXTitle"`. The attribute-name constants are **not re-exported** by `objc2-application-services` 0.3.2 — build them as `CFString`s from their stable raw values (`"AXFocusedWindow"`, `"AXTitle"`). [capture-ax-titles.json]

```rust
// inside the `capture` trait impl — objc2 NEVER leaks past this boundary (hard rule #5)
use objc2_application_services::AXUIElement;
use objc2_core_foundation::{CFString, CFType};
use core::ptr::NonNull;

// PROVISIONAL signatures — re-verify against resolved Cargo.lock before relying on them.
fn focused_window_title(pid: libc::pid_t) -> Result<Option<String>, CaptureError> {
    let app = unsafe { AXUIElement::new_application(pid) };

    let focused_attr = CFString::from_static_str("AXFocusedWindow");
    let title_attr = CFString::from_static_str("AXTitle");

    let mut window_out: *const CFType = core::ptr::null();
    let err = unsafe {
        app.copy_attribute_value(&focused_attr, NonNull::new(&mut window_out).unwrap())
    };
    // AXError variants (NoValue, AttributeUnsupported, CannotComplete) are EXPECTED, not bugs.
    // Map each to a typed Result and fall back to app-name-only. NEVER unwrap (hard rule #3).
    match err {
        AXError::Success => { /* downcast window_out → AXUIElement, repeat for title_attr */ }
        AXError::NoValue | AXError::AttributeUnsupported => return Ok(None),
        other => return Err(CaptureError::Ax(other)),
    }
    // ...
}
```

> **Hard rule #3 — No `unwrap()` / `expect()` / `panic!` in production paths.** `copy_attribute_value` legitimately returns `NoValue` / `AttributeUnsupported` / `CannotComplete` (unresponsive app). Every one is a typed `Result`, never a panic. [capture-ax-titles.json gotchas]

### C2. App switches and titles are event-driven, not polled
**One-line rationale:** push-based capture means near-zero wakeups while you sit on one window — replaces the v0.1.0 5s poll loop.

- App switch: observe `NSWorkspaceDidActivateApplicationNotification` on `NSWorkspace::sharedWorkspace().notificationCenter()`. The new app's `localizedName` / `bundleIdentifier` come from the notification's `userInfo` (`NSWorkspaceApplicationKey` → `NSRunningApplication`). [capture-events-runloop.json]
- Title change within an app: `AXObserver` (from **`objc2-application-services`** — not `accessibility-sys`; see Spike ② / D29). **`AXObserver`s are per-PID** — when the frontmost app changes, tear down the old observer and create a new one for the new PID, or title changes stop firing after the first switch. Register `AXFocusedWindowChanged` (+ `AXMainWindowChanged` / `AXFocusedUIElementChanged`) on the **application** element, but register `AXTitleChanged` on the **focused window** element and **re-point it whenever the focused window changes** — `AXTitleChanged` on the app element does *not* fire for window title changes. ✅ Spike ②: `AXFocusedWindowChanged` delivery confirmed; the observer source runs on the main run loop with no `NSApplication`. [capture-events-runloop.json gotchas]
- Heartbeat: one slow timer (~30–60s) bounds long-running/idle windows. This is the only periodic timer.

```rust
use block2::RcBlock;
// objc2-foundation needs features: NSOperation + NSString + block2
let block = RcBlock::new(move |note: NonNull<NSNotification>| {
    // read userInfo → NSRunningApplication → send (pid, name, bundle_id) to the watcher
});
let token = unsafe {
    nc.addObserverForName_object_queue_usingBlock(Some(name), None, None, &block)
};
// KEEP `token` alive for the observer's lifetime; dropping it unregisters and can crash on
// later delivery. removeObserver on teardown. [capture-events-runloop.json]
```

### C3. `kAXTitleChangedNotification` must be debounced
**One-line rationale:** browsers mutate the title during page load and progress bars tick in title bars — undebounced, the "near-zero wakeups" promise becomes an event storm.

Coalesce title-change events (e.g. trailing debounce ~250–500ms; **exact window is a spike measurement**, see Open questions). [capture-events-runloop.json]

### C4. All AX / Apple Event / NSWorkspace code lives behind the `capture` trait
**One-line rationale:** hard rule #5 — the rest of the app (and the whole cross-platform test suite) must build and pass without macOS, objc2, or any permission.

- The objc2 / `unsafe` / raw `NonNull<*const CFType>` ownership stays inside the impl. Nothing above `capture/` imports objc2.
- A fake `capture` impl drives all non-macOS tests. CI is headless Linux and **cannot exercise any native path** — the native + aarch64 spike is a manual macOS-only gate (D25). [capture-events-runloop.json gotchas]

### C5. Browser URL via Apple Events — incognito is checked BEFORE the URL is read
**One-line rationale:** D8 is non-negotiable; fetching then discarding an incognito URL still puts it in memory/logs.

Chromium family (Chrome, Brave = `"Brave Browser"`, Edge = `"Microsoft Edge"`, Arc = `"Arc"`, Vivaldi, Opera): branch on `mode of front window` (`"normal"` | `"incognito"`) and **skip the URL read entirely** when incognito. Map the captured bundle id → the correct AppleScript app name; hardcoding `"Brave"` (not `"Brave Browser"`) silently fails. [browser-automation-urls.json]

```applescript
-- Chromium: check mode first, only then read URL.
tell application "Google Chrome"
    if mode of front window is "incognito" then return "SKIP"
    return URL of active tab of front window
end tell
```

```applescript
-- Safari: WebKit document model, different script.
tell application "Safari" to return URL of front document
```

Safari has **no AppleScript property for private browsing**. Detection requires a localization-fragile System Events Window-menu inspection (`"Move Tab to New Private Window"` exists → private). **Safe default: if private-detection is uncertain, treat the Safari URL as unavailable** (title-derived site or no URL) — never risk recording a private URL. [browser-automation-urls.json]

### C6. The URL fallback ladder is explicit and privacy-safe at every rung
**One-line rationale:** capture must degrade per-browser on denial, not globally, and never leak an excluded/private URL.

```
URL via Automation
  └─ denied (-1743) / unsupported / private window
       └─ title-derived "site" (heuristic, lossy — from the AX title)
            └─ app-level only (app name + timestamps)
```

`-1743` (`errAEEventNotPermitted`) is a **permanent "permission unavailable"** signal (returned forever after "Don't Allow", or immediately if `NSAppleEventsUsageDescription` is missing). Treat it as "fall back," never as a transient error to retry. [browser-automation-urls.json, tcc-permissions.json]

### C7. Prefer shelling out to `/usr/bin/osascript` for the spike; NSAppleScript only if latency forces it
**One-line rationale:** osascript is process-isolated (a hung browser can't take down the app) and its prompt/escaping behavior is well-understood; the scripts are fixed static strings so injection risk is nil.

`NSAppleScript` (via `objc2-foundation` 0.3.2) is in-process and faster but **not thread-safe**, needs a dedicated CFRunLoop thread, and pulls in the same Info.plist/entitlement plumbing. Only adopt it if measured osascript latency is unacceptable for event-driven capture. [browser-automation-urls.json]

```rust
// Static scripts only, never interpolated user input.
let out = std::process::Command::new("/usr/bin/osascript")
    .arg("-e")
    .arg(r#"tell application "Google Chrome" to return URL of active tab of front window"#)
    .output()?; // non-zero exit / stderr signals -1743 → fall back per C6
```

### C8. Permission state is detected silently; prompted explicitly; never nagged
**One-line rationale:** the AX prompt typically appears **once per identity** — if dismissed, the only recovery is the deep link + manual toggle, so the app must detect state without re-prompting.

- Silent check (decide degraded mode, no dialog): `AXIsProcessTrusted()`.
- Explicit prompt (onboarding, user action): `AXIsProcessTrustedWithOptions({kAXTrustedCheckOptionPrompt: true})`.
- Automation prompt: `AEDeterminePermissionToAutomateTarget(...)` reports state and prompts once per target without sending a real event.
- Deep link to the Accessibility pane (confirmed): `x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility`.
- The **Automation** deep-link anchor (`?Privacy_Automation`) is **unverified** for the target OS — see Open questions; do not ship it unproven. [tcc-permissions.json]

Automation consent is **per (UsageOS, browser) pair** — the user is prompted separately for Chrome, then Brave, then Safari, then Arc. Onboarding must frame this as "add the browsers you use," not one toggle, so multiple prompts don't read as a bug. [tcc-permissions.json, browser-automation-urls.json]

### C9. `NSAppleEventsUsageDescription` is mandatory in the bundle Info.plist
**One-line rationale:** without it, Automation fails with `-1743` **before any prompt appears** — it looks like a hard denial the user never saw.

Set via `tauri.conf.json` macOS bundle config (the TCC client is the `.app` **bundle**, not the Swift AI sidecar). Required keys: `NSAppleEventsUsageDescription` (user-facing, e.g. "UsageOS reads your active browser tab URL on-device to label your day. It never leaves your Mac.") and the `com.apple.security.automation.apple-events` entitlement. [tcc-permissions.json, browser-automation-urls.json]

### C10. Idle detection stays on `user-idle` — never CGEventTap
**One-line rationale:** `user-idle` 0.6 (already shipped in v0.1.0) reads CoreGraphics aggregate idle time, which needs **no** permission; `CGEventTap` would trigger an Input Monitoring / Accessibility prompt (the wrong tool).

```rust
let idle_secs = UserIdle::get_time().map(|t| t.as_seconds()).unwrap_or(0);
```
[capture-events-runloop.json]

### C11. Stabilize the dev signing identity before touching permissions
**One-line rationale:** TCC keys grants to the binary's code-signing identity (cdhash / Developer ID); an unsigned/ad-hoc dev build whose hash changes every rebuild silently detaches the grant, producing phantom "permission" bugs.

`active-win-pos-rs` can be kept for app name / window geometry, but the **title path** moves to AX (C1). Before the spike: either ad-hoc-sign the dev `.app` with a **stable** identifier or sign with a Developer ID cert, then verify `AXIsProcessTrusted()` stays true across rebuilds. (Whether stable ad-hoc alone holds the grant is an Open question.) [tcc-permissions.json, capture-ax-titles.json, architecture.md]

---

## Pinned versions (provisional — confirm against resolved `Cargo.lock`)

| Crate / surface | Version | Note |
|---|---|---|
| `objc2-application-services` | 0.3.2 | `AXUIElement`, `AXIsProcessTrusted(WithOptions)`, `kAXTrustedCheckOptionPrompt`. Attribute-name constants NOT re-exported — build CFStrings. |
| `objc2-core-foundation` | 0.3.2 | Must be version-matched to the line above (same release train) or types are incompatible. |
| `objc2-app-kit` | 0.3.2 | `NSWorkspace::sharedWorkspace()` / `notificationCenter()`; `NSRunningApplication` feature-gated. |
| `objc2-foundation` | 0.3.2 | `NSNotificationCenter` block observer (features `NSOperation` + `NSString` + `block2`); `NSAppleScript`. |
| `objc2-application-services` (observers) | 0.3.2 | ✅ Spike ②/D29: also exposes the **`AXObserver`** family — `AXObserver::create(pid, cb, out)`, `add_notification` / `remove_notification`, `run_loop_source()`, callback type `AXObserverCallback`. Keeps the whole AX surface in one crate. |
| ~~`accessibility-sys`~~ | ~~0.2.0~~ | **Dropped** (Spike ②/D29). The objc2 family covers `AXObserver*`; no second FFI style needed. |
| `block2` | 0.6 (`>=0.6.1,<0.8`) | `RcBlock` backs the `NSWorkspace` activation observer; resolves to 0.6.2 under `objc2-foundation` 0.3.2. |
| `user-idle` | 0.6 | Already shipped v0.1.0. CoreGraphics idle read — no permission. |
| `active-win-pos-rs` | 0.8 | Already in Cargo.toml. Keep for app/geometry; do NOT use for titles (CGWindowList → Screen Recording). |
| Target OS floor | macOS 13+ (System Settings era); validate on 15 / 26 | D9 (Foundation Models) pushes the realistic floor high. |

---

## ⚠️ Open questions / verify in the Phase-0 spike

These are **not settled**. The research had no verification pass; each item below is either unconfirmed by an authoritative source or known to shift by OS version. Do not assert any of these as fact in code or docs until the spike (D22/D25) proves them on a real Apple Silicon Mac.

1. ~~**Crate version + exact signatures.**~~ ✅ **RESOLVED — Spikes #1 + ②.** `objc2-application-services` 0.3.2 exposes `AXUIElement::new_application(pid) -> CFRetained<AXUIElement>`, `copy_attribute_value(&CFString, NonNull<*const CFType>) -> AXError`, **and** the full `AXObserver` family — so the observer path stays in the **objc2 family** and `accessibility-sys` is dropped (D29). All provisional objc2 pins held (`objc2` 0.6.4, the `objc2-*` crates 0.3.2, `block2` 0.6.2).
2. ~~**Electron / Chromium AX titles (HIGHEST RISK).**~~ ✅ **RESOLVED — Spike #1 (`spikes/ax-titles/`): all real.** Chrome, Brave, Cursor (VS Code fork), Claude, Notion, Figma, WhatsApp, Spotify, Finder, iTerm2 all returned non-empty `AXTitle` with Accessibility only / Screen Recording OFF. The make-or-break premise holds; R4 retired. (Titles also carry sensitive content → D8 is load-bearing.)
3. ~~**Threading / run-loop model.**~~ ✅ **RESOLVED — Spike ②: model (a), the main run loop.** Both the `AXObserver` run-loop source and synchronously-delivered `NSNotificationCenter` blocks need a running `CFRunLoop`; the spike attaches the observer source and activation block to the **main run loop** and proves callbacks fire, the per-PID observer is rebuilt on app switch, and a `Send`-channel hand-off keeps the Tokio executor unblocked — **with no `NSApplication`** (in the app, register into Tauri's main loop during `setup` instead of calling `run()`). The dedicated-thread option (b) is unnecessary and was not adopted. [capture-events-runloop.json riskyClaims]
4. ~~**Apple Silicon (aarch64) build + runtime.**~~ ✅ **RESOLVED — Spikes #1 + ②.** Both crates build as `arm64` Mach-O and run on a real Apple Silicon Mac; AX queries and observer callbacks fire at runtime. The docs.rs x86_64-only rendering was indeed a display artifact.
5. ~~**Chromium `mode` readability (load-bearing for D8).**~~ ✅ **RESOLVED — Spike ③.** `mode of front window` **is readable** on existing windows and returns `"normal"` / `"incognito"` (the sdef's "set only once" was a *write* constraint). Read it **first** and skip any non-`"normal"` value (enumerable-free safe-default) — proven live on Chrome (normal→incognito→normal). **Arc still unverified** (not running during the spike; its window/space model may differ) — keep on the manual list.
6. **Safari private-window detection.** The System Events menu-item label (`"Move Tab to New Private Window"`) is version- and locale-specific. If wrong, a private Safari URL could be recorded (a D8 violation). Confirm on the target Safari version; ship the safe-default fallback regardless. [browser-automation-urls.json riskyClaims]
7. ~~**osascript vs NSAppleScript latency.**~~ ✅ **RESOLVED — Spike ③: osascript is fine.** One URL fetch measured **~140–160 ms warm (~270 ms cold)** — process-isolated and well within budget for query-on-switch (not a hot loop). `NSAppleScript`'s in-process speed is **not needed**; keep shelling osascript (C7).
8. **Dev-build grant persistence.** Does a *stable* ad-hoc identity hold the Accessibility + Automation grants across `cargo tauri dev` rebuilds, or is a real Developer ID signature required? Grant once, rebuild N times, check `AXIsProcessTrusted()` stays true and Apple Events still succeed without re-prompt. Document the exact `codesign` invocation. [tcc-permissions.json riskyClaims]
9. **Automation deep-link anchor.** `x-apple.systempreferences:com.apple.preference.security?Privacy_Automation` (and `?Privacy_ScreenCapture`) is unverified for macOS 15/26 — System Settings (Ventura+) changed anchors and some silently land on the top of Privacy & Security. Confirm the working anchor on the target OS before shipping it in onboarding. [tcc-permissions.json riskyClaims]
10. **Non-bundle client visibility (macOS 26.1).** A reported bug fails to show binary (non-bundle) clients in the Accessibility list. Ensure the `.app` **bundle** is the requesting TCC client and the Swift Foundation Models sidecar does NOT separately request AX/Automation. [tcc-permissions.json gotchas]
11. **Idle underlying call.** `user-idle` 0.6 macOS path is *believed* to use a CoreGraphics idle read (not `CGEventTap`); confirm by eyeballing the source — it's load-bearing for the "no Input Monitoring" promise. [capture-events-runloop.json riskyClaims]
12. **Idle wakeups / CPU figure.** Measure idle wakeups/CPU with `powermetrics` over ~10 min idle and a normal work session; confirm chatty-title apps don't cause a wakeup storm and the C3 debounce window is right. [capture-events-runloop.json]
13. **Sequoia 15+ re-prompts.** Users may be re-prompted to re-confirm Accessibility periodically and after reboot. The watcher must detect a runtime drop to `AXIsProcessTrusted() == false` and surface it, not silently log empty titles. [capture-ax-titles.json gotchas]

---

## Recovery / testing harness

Repeatable spike testing needs a clean-state reset (uses the **exact final bundle id**, or it silently no-ops):

```bash
tccutil reset Accessibility app.usageos
tccutil reset AppleEvents app.usageos   # clears every per-target pair
```
[tcc-permissions.json, browser-automation-urls.json]

---

## Citations (source URLs)

**AX titles / Accessibility**
- https://docs.rs/objc2-application-services/0.3.2/objc2_application_services/struct.AXUIElement.html
- https://docs.rs/objc2-application-services/0.3.2/objc2_application_services/all.html
- https://developer.apple.com/documentation/applicationservices/axuielement
- https://developer.apple.com/documentation/applicationservices/1462085-axuielementcopyattributevalue
- https://developer.apple.com/documentation/applicationservices/1459186-axisprocesstrustedwithoptions
- https://developer.apple.com/forums/thread/94878
- https://github.com/dimusic/active-win-pos-rs
- https://github.com/electron/electron/issues/37465

**Events / run loop / idle**
- https://docs.rs/objc2-app-kit/latest/objc2_app_kit/struct.NSWorkspace.html
- https://developer.apple.com/documentation/appkit/nsworkspace/didactivateapplicationnotification
- https://docs.rs/objc2-foundation/latest/x86_64-apple-darwin/objc2_foundation/struct.NSNotificationCenter.html
- https://docs.rs/accessibility-sys/latest/accessibility_sys/fn.AXObserverCreate.html
- https://docs.rs/accessibility-sys/latest/accessibility_sys/type.AXObserverCallback.html
- https://github.com/madsmtm/objc2
- https://lib.rs/crates/user-idle
- https://hacktricks.wiki/en/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-input-monitoring-screen-capture-accessibility.html
- https://deepwiki.com/tauri-apps/tauri/2.2-application-lifecycle-(app-and-apphandle)

**Browser URLs / Automation**
- https://chromium.googlesource.com/chromium/src.git/+/lkgr/chrome/browser/ui/cocoa/applescript/scripting.sdef
- https://gist.github.com/vitorgalvao/5392178
- https://joschua.io/posts/2023/05/24/automating-arc-applescript/
- https://alexwlchan.net/2021/detect-private-browsing/
- https://scriptingosx.com/2020/09/avoiding-applescript-security-and-privacy-requests/
- https://steipete.me/posts/2025/applescript-cli-macos-complete-guide
- https://v2.tauri.app/distribute/macos-application-bundle/
- https://docs.rs/objc2-foundation/latest/objc2_foundation/
- https://docs.rs/osascript/

**TCC / permissions / signing / distribution**
- https://jano.dev/apple/macos/swift/2025/01/08/Accessibility-Permission.html
- https://www.rainforestqa.com/blog/macos-tcc-db-deep-dive
- https://www.felix-schwarz.org/blog/2018/08/new-apple-event-apis-in-macos-mojave
- https://developer.apple.com/forums/thread/666528
- https://ss64.com/mac/tccutil.html
- https://mjtsai.com/blog/2023/02/09/resetting-tcc/
- https://eclecticlight.co/2025/11/08/explainer-permissions-privacy-and-tcc/
- https://github.com/asmvik/yabai/issues/2688
- https://developer.apple.com/documentation/security/notarizing-macos-software-before-distribution
- https://angelica.gitbook.io/hacktricks/macos-hardening/macos-security-and-privilege-escalation/macos-security-protections/macos-tcc
