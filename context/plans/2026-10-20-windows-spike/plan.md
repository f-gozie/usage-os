# Windows spike

What it takes to run UsageOS on Windows. A 1–2 day investigation on branch
`spike/windows-port`, not the port itself. Nothing here ships to users and nothing merges to
`main`. The proposed decision is [`adr-draft.md`](adr-draft.md).

**Bottom line:** go, as a labelled beta. The app already compiles on Windows, the old "specta
won't link" error is a missing manifest in test binaries (fixed here with one `build.rs`
change), and the macOS-only surface is small and already fenced off. A friend-grade Windows
beta is about **2 weeks** of work (8–12 working days), most of it capture and on-device
testing. Getting close to macOS (event-driven capture, app icons, signed public download) is
another **1–1.5 weeks**.

## 1. CI evidence

A temporary `windows-latest` job (`windows-spike` in `.github/workflows/ci.yml`, spike branch
only, every step `continue-on-error` so one run shows everything; read the step *outcomes*
in the job summary, since `continue-on-error` reports every step's conclusion as success).
Runner `windows-2025-vs2026`, rustc 1.99.1, Node 22.

| Run | Code | `cargo build` | clippy `-D warnings` | `cargo test` | `export_bindings` + fresh | NSIS bundle |
|---|---|---|---|---|---|---|
| [39389577135](https://github.com/f-gozie/usage-os/actions/runs/39389577135) | `main` (`da55dfc`) + Windows job | ✅ | ✅ | ❌ `0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND` | ❌ same | ✅ `UsageOS_0.1.6_x64-setup.exe` |
| [39389766263](https://github.com/f-gozie/usage-os/actions/runs/39389766263) | + manifest fix (`3c0ce5d`) | ✅ | ✅ | ✅ 184 passed, 1 ignored | ✅ 1 passed; regenerated `bindings.ts` identical | ✅ 5.1 MB (artifact, 7 days) |
| [39390549127](https://github.com/f-gozie/usage-os/actions/runs/39390549127) | June 22 commit `35757c0` (last with Windows CI), unchanged | ✅ | ✅ | ❌ `0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND` | skipped | — |
| [39390568521](https://github.com/f-gozie/usage-os/actions/runs/39390568521) | `35757c0` + manifest fix | ✅ | ✅ | ✅ 26 passed | skipped | — |

The Linux and macOS lanes of the same runs stayed green, and the Windows job doesn't gate.
Branches: `spike/windows-port` (the spike), `spike/windows-june-repro` and
`spike/windows-june-fix` (the A/B on the old commit; delete after reading).

What this shows: the app compiles, lints clean under `-D warnings` and bundles an installer on
Windows today with no code changes. The only thing broken is that test executables can't
start. The original failure reproduces on the old commit and on `main`, and the same one-file
fix clears it on both.

## 2. The link failure, diagnosed

`STATUS_ENTRYPOINT_NOT_FOUND` (0xC0000139) was never a tauri-specta problem. The chain:

1. `muda` (Tauri's menus, pulled in by the tray) and `tauri-runtime-wry` (its dialogs) import
   `TaskDialogIndirect` from `comctl32.dll`. Only version 6 of that DLL exports it.
2. A process gets comctl32 v6 only if its executable carries a manifest asking for
   `Microsoft.Windows.Common-Controls` 6.0. Without one, Windows loads v5.
3. `tauri-build` embeds that manifest through `embed-resource`, which links it with
   `cargo:rustc-link-arg-bins`: the app binary only. The lib's unit-test executable
   (`usage_os_lib-<hash>.exe`) gets none.
4. So the app runs fine, but `cargo test` exits before the first test, with exactly the
   reported error. Windows was dropped in the PR that added tauri-specta (#9). Its
   `export_bindings` test calls `make_builder()`, most likely the first time a test reached
   Tauri's runtime and so pulled those imports into the test executable. Windows had been
   green on #7, which is why specta took the blame.

**Fix** (`src-tauri/build.rs` + `src-tauri/windows-app-manifest.xml`, the same manifest
Tauri ships): on `windows`/`msvc` targets only, turn off tauri-build's own manifest
(`WindowsAttributes::new_without_app_manifest()`) and link the manifest into **every** target
with `/MANIFEST:EMBED /MANIFESTINPUT:<path>`. The app binary ends up with the same manifest
as before; test binaries get one too. Nothing changes on macOS or Linux (the branch is
behind a target check). The manifest is Tauri's default, unchanged; `tauri-build`'s docs warn
that any custom manifest must keep this common-controls dependency.

Caveat to check in the real port: `/MANIFESTINPUT:` is passed unquoted, so a checkout path
with spaces may need quoting.

## 3. macOS-only surface, with the Windows plan

Sizes: **XS** < ½ day · **S** ½–1 day · **M** 2–3 days · **L** 4+ days. "Beta" = needed for
the friend beta; "later" = parity work after it.

| Surface (today) | Windows equivalent | Size | When |
|---|---|---|---|
| `capture/macos` — NSWorkspace activation + per-PID `AXObserver` (D29) | `WindowsCapture` behind `CaptureSource`. Beta: poll `active-win-pos-rs` (foreground HWND → `GetWindowTextW` title, process via `QueryFullProcessImageNameW`, name from the exe's FileDescription; UWP apps resolved through `ApplicationFrameHost`) + `user-idle` (`GetLastInputInfo`). No permission needed for any of these. | S | beta |
| ↳ event-driven capture | `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` + `EVENT_OBJECT_NAMECHANGE` filtered to the foreground window, `WINEVENT_OUTOFCONTEXT` on a dedicated thread with its own message loop, events sent over the existing `std::mpsc` channel. Same shape as D29/D33. | M | later |
| `capture/macos/browser.rs` — Chromium AppleScript privacy check, browser registry via NSWorkspace (D76) | **Time only for every browser.** Detect browsers from the registry (`HKLM`/`HKCU\SOFTWARE\Clients\StartMenuInternet\*` → exe paths), plus a fixed exe backstop (`chrome`, `msedge`, `firefox`, `brave`, `opera`, `vivaldi`, `arc`, `zen`, `tor`, …). A matched window drops title and URL (`BrowserUrl::Unverified`). | S | beta |
| ↳ browser detail | UI Automation can read the address bar of Chromium and Firefox windows, but nothing in UIA proves a window isn't private (Edge's "InPrivate" and Firefox's "Private Browsing" are title/locale heuristics D76 rejected; Chrome incognito has no reliable marker). The route that can prove it is the browser extension + native messaging (#44), whose `incognito` flag works on both OSes. | L | later |
| `capture/macos/terminal.rs` — iTerm2 AppleScript, `pgrep` + `proc_pidinfo` cwd | **Skip.** Windows has no supported API for another process's cwd (only reading its PEB, which is undocumented and fragile). Project inference falls back to titles (VS Code's "file — folder — Visual Studio Code" works). | — | skip |
| `local_clock.rs` — `localtime_r` on macOS, **UTC elsewhere** | Must fix for Windows: the recap's clock phrases and the tray hour would be off by the UTC offset. `GetTimeZoneInformationForYear` + `SystemTimeToTzSpecificLocalTimeEx` (DST-correct per date), or the MSVC CRT's `localtime_s`. | XS | beta |
| `login_item.rs` — `SMAppService` start at login | `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` value with `--hidden` (what `tauri-plugin-autostart` does on Windows). `respawn_detached` isn't needed (no launchd). | S | beta |
| `login_item.rs` — `flock` instance lock | `tauri-plugin-single-instance` (named mutex), which also focuses the existing window on a second launch — the Windows stand-in for `RunEvent::Reopen`. | S | beta |
| `permissions/` — Accessibility + Automation | Nothing to grant. The non-macOS stub already reports `NotApplicable`/trusted. The work is in the UI (row below). | — | — |
| `tray_icon.rs` — black template PNGs | Tray works on Windows, but black frames vanish on the default dark taskbar. Add light frames and pick by `SystemUsesLightTheme`. | S | beta |
| `glance_panel.rs` — NSPanel reclass (D56) | No reclass needed: an undecorated, always-on-top, `skip_taskbar` window. `position_glance` assumes a menu bar at the top; on Windows the tray is usually bottom-right, so open upwards and clamp to the work area. Needs on-device tuning. | S–M | beta |
| `set_dock_visible` — Dock activation policy (D68) | Skip. The taskbar button follows the window already. | — | skip |
| Window chrome — `titleBarStyle: Overlay`, `hiddenTitle`, `TitleBar.tsx` leaves 80px for traffic lights | Overlay is macOS-only, so Windows would show the native title bar **plus** the 38px custom bar. Either `decorations: false` in a `tauri.windows.conf.json` with min/max/close buttons in `TitleBar`, or keep the native bar and drop the custom one on Windows. | S | beta |
| Frontend — Accessibility/Automation onboarding steps, Settings permission rows, `DegradedBanner` copy, "Show in Finder" | Hide the permission steps and rows on Windows; "Show in File Explorer". Needs a platform flag in the frontend (a small command through tauri-specta, or `tauri-plugin-os`). | S | beta |
| `apps.rs` — `/Applications` bundles + `sips` icons | Already degrades to an empty catalog off macOS (monogram fallback). Real version: walk the Start Menu `.lnk` files (`%ProgramData%` and `%AppData%`), resolve targets, extract icons (`SHGetFileInfo`/`IShellItemImageFactory` → PNG). | M | later |
| Seed category rules (`0003`, `0006`, `0008` migrations) | Substring matches mostly carry over ("Visual Studio Code" ∋ "Code", "Windows Terminal" ∋ "Terminal"), but Windows-common apps are missing: Microsoft Edge, Outlook, Word, Excel, PowerPoint, PowerShell, File Explorer, Notepad++. One new seed migration (harmless on macOS). | XS | beta |
| `enrich/project.rs` — shells out to `git` | Works if git is on `PATH`. A GUI-subsystem app spawning `git` flashes a console window unless it sets `CREATE_NO_WINDOW`. Only reached from a cwd today, which Windows won't have, but guard it. | XS | beta |
| Swift sidecar (`externalBin`, dormant since D74) | Skip. Override `externalBin` to `[]` in `tauri.windows.conf.json` so no stub is needed. | XS | beta |
| `tauri.conf.json` macOS bundle block, `entitlements.plist`, agent plist | Ignored on Windows. Add `bundle.windows` (NSIS, `webviewInstallMode`, publisher). | XS | beta |
| `scripts/release-macos.sh`, signing + notarization (D64) | A `release-windows` CI job on `windows-latest` (NSIS bundle, updater signature with the same minisign key), `gen-latest-json.sh` gains a `windows-x86_64` entry. Authenticode signing: see §5. | M | beta (unsigned) / later (signed) |

**Beta total:** capture with the browser gate ~1.5d, clock ¼d, login + single instance ½d,
tray ½d, glance 1d, window chrome ½d, frontend platform gating 1d, seed rules + sidecar
config + git flag ½d, Windows release job + updater 1.5d, on-device testing and fixes 2d,
docs/copy ½d → **≈ 10 days**.

**Later:** event-driven capture 2–3d, app catalog 2d, signing setup 1–2d, winget ½d →
**≈ 6–7 days**. Browser detail through an extension is its own project (L) and serves macOS
Firefox too.

## 4. Capture design on Windows

- **Behind the trait.** `capture/windows/mod.rs` (`#[cfg(target_os = "windows")]`) with a
  `WindowsCapture: CaptureSource`; `default_source()` gains a Windows arm. `PollingCapture`
  stays the fallback for everything else. The consumer, D8 exclusions, D39 idle gates,
  enrichment and the database are unchanged. `FakeCapture` keeps the spine testable on any OS
  (hard rule 5).
- **The polling fallback already gets Windows titles.** `active-win-pos-rs` 0.8.4 reads the
  title with `GetWindowTextW` on Windows, which needs no permission (on macOS the same field
  comes from `CGWindowName`, which needs Screen Recording; that's why `polling.rs` drops it).
  The beta source can be `PollingCapture` with titles turned on for Windows, plus the
  browser gate. App names come from the exe's FileDescription ("Google Chrome", "Visual
  Studio Code"), falling back to the file stem.
- **Elevated windows:** `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` works on elevated
  processes from a normal one, and `GetWindowTextW` works across integrity levels, so admin
  apps still show up. Protected processes (some anti-cheat, Defender UI) come back without a
  process path; they should record as an unknown app, not an error.
- **Idle and lock:** `GetLastInputInfo` covers idle. Locking the screen stops input, so the
  existing idle gate closes the span; `WTSRegisterSessionNotification` could close it at the
  lock instant in the event-driven version.
- **Browsers (D76):** time only, as in §3. The registry check is the Windows equivalent of
  "any app macOS lists as opening web links and HTML files"; terminals that register for
  `http` (rare on Windows) get the same exception as on macOS.
- **Event-driven later:** `SetWinEventHook` with `WINEVENT_OUTOFCONTEXT` delivers callbacks
  on the thread that called it, which must pump messages. A dedicated thread with
  `GetMessageW` keeps it off Tauri's event loop (unlike macOS, nothing forces the main
  thread). `EVENT_OBJECT_NAMECHANGE` is chatty system-wide; filter by `hwnd ==
  GetForegroundWindow()` and `idObject == OBJID_WINDOW`, then the existing dedupe.

## 5. Packaging and distribution

- **Installer:** Tauri's NSIS target (per-user install, no admin, `.exe`). MSI (WiX) is an
  option for managed machines; not needed for a beta. WebView2 is built into Windows 11 and
  shipped with Edge on Windows 10; NSIS's default `webviewInstallMode` (download
  bootstrapper) only runs if it's missing. That download happens in the installer, not the
  app's data path.
- **Updater:** the Tauri updater supports Windows (NSIS and MSI). With
  `createUpdaterArtifacts: true` it signs the installer itself with the existing minisign
  key, and `latest.json` needs a `windows-x86_64` platform entry. Installs run in passive
  mode and the app restarts.
- **Code signing (Authenticode):** unsigned installers trigger SmartScreen's "Windows
  protected your PC" (More info → Run anyway). Options:
  - **SignPath Foundation:** free for OSI-licensed open-source projects (MIT qualifies).
    Signing happens on their HSMs from a CI build of the public repo, with origin
    verification. The publisher shown is "SignPath Foundation", not the owner. Needs a
    code-signing policy on the project site.
  - **Azure Artifact Signing** (formerly Trusted Signing): about $9.99/month (Basic, 5,000
    signatures). Individual developers must be in the **US or Canada**; organizations in
    more countries. No free/trial subscriptions.
  - **OV certificate** on a cloud HSM (e.g. Certum SimplySign, from about €49–€170 a year
    depending on term and reseller). Since 2023 keys must live on hardware or a cloud HSM.
    SmartScreen reputation builds over downloads either way; EV certificates were reported
    to stop getting instant reputation in 2024.
  - **Recommendation:** unsigned for the friend beta. Before a public download, SignPath
    Foundation if the owner is fine with its name as publisher; otherwise Artifact Signing
    if eligible, else a cloud OV certificate.
- **winget:** a manifest PR to `microsoft/winget-pkgs` per release (automatable with
  `wingetcreate` or the `winget-releaser` action), pointing at the GitHub release asset.
  Unsigned installers are accepted but can be flagged by Defender/SmartScreen in validation,
  so do this after signing. ½ day.

## 6. Recommendation

**Go, as a Windows beta**, with the ADR draft lifting "No Windows in v1" for Windows only.
Order:

1. Land the manifest fix and make Windows a gated CI lane (cheap, and it keeps the core
   honest on a third OS).
2. Beta (≈ 2 weeks): §3 "beta" rows. Ship an unsigned NSIS build to the friend through a
   GitHub pre-release.
3. Parity (≈ 1–1.5 weeks): event-driven capture, app catalog, signing, winget.

**Prerequisite:** a Windows machine for on-device checks (capture, tray, glance, installer,
updater). CI compiles and runs tests but can't exercise a desktop session. A Windows 11 ARM
VM on the Mac (UTM/Parallels, running the x64 build under emulation) or the friend's PC
both work; budget the 2 testing days either way.

**Risks:** the glance popover and tray behaviour are the least predictable (multi-monitor,
taskbar on other edges, DPI scaling); browser users get time only until an extension exists,
which makes the Windows beta noticeably thinner for browser-heavy days; the macOS
`tauri-plugin-shell` pin (=2.2.1) and the frozen Tauri 2.9.3 stack are untested on Windows
beyond compiling.

## Spike checklist

- [x] Windows CI job on the spike branch (build, clippy, test, bindings, NSIS bundle)
- [x] Root cause of `STATUS_ENTRYPOINT_NOT_FOUND` + fix in `build.rs`
- [x] macOS-only inventory with Windows equivalents and sizes
- [x] Capture design behind `CaptureSource`, D76 for browsers
- [x] Packaging, signing, updater, winget
- [x] ADR draft
- [ ] Owner decision on the ADR
