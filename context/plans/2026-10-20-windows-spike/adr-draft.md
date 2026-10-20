# ADR draft — D77: A Windows beta, behind the same capture trait

> **Status: draft, not accepted.** Written by the Windows spike
> ([`plan.md`](plan.md)). Nothing here applies until the owner accepts it; on acceptance,
> append the text below to `context/decisions.md` as D77 (or the next free number) and
> update the places listed under "If accepted".

### D77 — Lift "No Windows in v1": ship a Windows beta, macOS stays first

**"No Windows/Linux in v1" (CLAUDE.md, `vision.md`) is lifted for Windows only.** UsageOS
gets a Windows beta, labelled beta, built from the same codebase. macOS stays the first
platform: no macOS behaviour changes for Windows, and Windows work never blocks a macOS
release. Linux stays out.

**Why now:**
- The core was already platform-neutral (database, rules, rollup, recap, Calendar, the IPC
  layer), and capture sits behind `CaptureSource` (hard rule 5). The spike found the
  macOS-only surface is small and already fenced by `cfg(target_os = "macos")`.
- The reason Windows was dropped from CI wasn't real. `STATUS_ENTRYPOINT_NOT_FOUND` was
  never a tauri-specta problem: Tauri links its common-controls v6 manifest into the app
  binary only, so `cargo test` binaries load comctl32 v5, which has no `TaskDialogIndirect`,
  and die before any test runs. A `build.rs` change that links the same manifest into every
  target fixes it (see the spike's CI evidence).
- D60 already set the identity as platform-agnostic, with "macOS is the first platform, not
  the brand". A Windows beta is consistent with it.
- The owner promised Windows to a friend; the spike puts the cost at about two weeks for a
  beta.

**What the Windows beta is:**
- **Capture:** a `WindowsCapture` behind `CaptureSource`. The beta polls (`active-win-pos-rs`
  foreground window: process, file description, `GetWindowTextW` title) with `user-idle`
  (`GetLastInputInfo`). Later it moves to `SetWinEventHook(EVENT_SYSTEM_FOREGROUND,
  EVENT_OBJECT_NAMECHANGE)` on its own message-loop thread, the Windows counterpart of D29.
  Windows needs no consent for any of these, so there is no permissions step.
- **Browsers follow D76: time only.** Any app registered as a browser (registry
  `StartMenuInternet`), plus a fixed backstop list of browser executables, records no title
  and no URL. Window-title suffixes such as "InPrivate" are not used as proof, for the same
  reasons D76 rejected per-browser heuristics. Browser detail comes back only with something
  that can prove a window is normal (the extension/native-messaging route of #44, which works
  on both platforms).
- **No terminal cwd.** Windows has no supported way to read another process's working
  directory; project inference uses titles only (e.g. VS Code's "file — folder" title).
- **Skipped on Windows:** the Swift sidecar (dormant since D74), the Dock activation policy,
  `RunEvent::Reopen` (replaced by single-instance focus), Accessibility/Automation
  onboarding.
- **Distribution:** an NSIS installer built by a Windows CI job, the Tauri updater with the
  same updater key (a `windows-x86_64` entry in `latest.json`). Unsigned for the private
  beta; Authenticode signing (see below) before any public download link or winget.

**Signing:** the beta ships unsigned to known testers (SmartScreen's "More info → Run
anyway"). Before a public Windows download: SignPath Foundation (free for OSI-licensed
projects, signs CI-built artifacts, and the publisher shown is the Foundation) or Azure
Artifact Signing (about $9.99/month; individual developers must be in the US or Canada),
else a cloud-HSM OV certificate. Decide at that point.

**Rejected:**
- **Keep "No Windows in v1".** It rested on a link error that turned out to be a
  manifest problem.
- **Wait for event-driven capture before any beta.** Polling with titles is the
  same fallback D32 kept; it is good enough for a beta, and D5's objection (blurry
  boundaries) is about a 5 s edge, not about what is recorded.
- **Record browser titles on Windows with a title-suffix check.** It breaks D76's
  promise in a locale- and version-fragile way.
- **Linux at the same time.** Nobody has asked; it doubles the on-device testing.

**If accepted:**
- `CLAUDE.md` → "What this product is NOT" becomes "No Linux in v1. Windows is a beta.";
  the Stack/CI paragraphs drop "Windows dropped — the specta IPC stack won't link there".
- `context/vision.md` → Out of scope: "Linux" (Windows moves to "beta").
- `context/architecture.md` line 74 (CI paragraph) and the `ci.yml` matrix comment.
- `.github/workflows/ci.yml` → Windows becomes a real matrix entry (gated), not the spike
  job; the `push: spike/**` trigger is removed.
- README / landing: "Download for Windows (beta)" only once a build exists (D60's honesty
  rule).
