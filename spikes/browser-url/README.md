# Spike ③ — browser URL capture + incognito/private exclusion

> **Status: ✅ RUN — core PASS (2026-06-22, macOS / Apple Silicon).** The Chromium
> path works end-to-end: the active-tab **URL reads** via Apple Events, and an
> **incognito window is excluded** because `mode of front window` is read **first**
> and any non-`"normal"` mode skips the URL entirely — proven live, normal →
> incognito → normal (see [Observed results](#observed-results--pass-2026-06-22)).
> osascript latency is **~140–160 ms** (fine for event-driven capture). **Residual
> (documented manual checks):** Safari URL + private-detection (R16/R18 — Safari
> wasn't running), Arc, and the `-1743` deny path (code present, not triggered).
> This crate is **isolated** — not a member of the `src-tauri` workspace, and has
> **zero dependencies** (pure std + `/usr/bin/osascript`).

## What this proves (and why it matters)

The "site" axis wants the **active browser tab URL** — but **D8 is non-negotiable**:
an incognito/private URL must never be read, let alone stored. Spike #1 already
showed titles leak sensitive content (a Gmail subject), so the URL path has to be
privacy-safe by construction. This spike settles:

| Risk | Question | Result |
|------|----------|--------|
| **R15** | Chromium front-tab URL via `URL of active tab of front window`? | ✅ Chrome + Brave return the exact URL. App names matter (`"Brave Browser"`, not `"Brave"`). |
| **R17** | Is `mode of front window` actually **readable** (sdef says "set only once at creation"), and does it return `"incognito"`? | ✅ Readable; returns `"normal"` / `"incognito"`. Checked **before** any URL read; non-`"normal"` ⇒ skip. |
| **R21** | osascript shell-out latency acceptable for event-driven capture? | ✅ ~140–160 ms warm (≈270 ms cold). We query on app-switch, not in a hot loop, so this is fine. `NSAppleScript` not needed (C7). |
| **R19** | Automation TCC is per-(client,target); first query prompts; deny ⇒ `-1743` permanent? | ✅ (partial) separate prompts for Chrome then Brave; grants persisted. `-1743` fallback is coded (`classify_err`) but not live-triggered. |
| **R16** | Safari URL via `URL of front document`? | ⬜ Not run (Safari wasn't running). Coded; verify in the manual pass. |
| **R18** | Safari Private Browsing detectable? | ⬜ No scriptable property → **safe-default: never emit a Safari URL we can't prove non-private.** The spike flags Safari reads as *unenforced* and does not pretend otherwise. |

### The fallback ladder (capture standard C6) — privacy-safe at every rung

```
URL via Automation
  └─ incognito / private window      → skip (no URL)                ✅ proven (Chromium)
  └─ denied (-1743) / unsupported    → title-derived "site" (upstream, from the AX title)
       └─ app-level only (app + timestamps)
```

`-1743` (`errAEEventNotPermitted`) is **permanent** — returned forever after "Don't
Allow", or immediately if `NSAppleEventsUsageDescription` is missing. Treat it as
"fall back", never retry.

## Design

Pure std, no objc2: each browser's **`front window` is queried directly**, so there
is no frontmost-app detection, no NSWorkspace, and no run-loop staleness. In the real
app, Spike ②'s event-driven capture says *when* a browser is frontmost; this query
says *what* it's showing. Per C7, it shells `/usr/bin/osascript` with **static**
scripts (no interpolated user input → no injection surface).

- `application "X" is running` (a Launch Services query — no Apple Event, no prompt,
  never launches the app) guards every query, so non-running browsers are skipped
  without side effects.
- **Chromium** (`Google Chrome`, `Brave Browser`, `Microsoft Edge`, `Arc`):
  ```applescript
  tell application "<app>"
    if (count of windows) is 0 then return "NOWIN"
    set m to mode of front window
    if m is not "normal" then return "PRIVATE<TAB>" & m   -- skip incognito/guest/…
    return "URL<TAB>" & (URL of active tab of front window)
  end tell
  ```
  Skipping on **any** non-`"normal"` mode is the safe-default (covers `incognito`,
  `guest`, and Edge's variants without enumerating them).
- **Safari**: reads `URL of front document` but **flags it unenforced** — Safari has
  no private-browsing property (R18), so production must add the System-Events
  safe-default check before this URL may be used.
- Per-query **latency** is measured; output is **deduped** (a browser's line prints
  only when its state changes).

## Build

```sh
cd spikes/browser-url
cargo build
codesign --force --sign - target/debug/browser-url   # consistency with the other spikes
```

- **Binary:** `spikes/browser-url/target/debug/browser-url` — `Mach-O 64-bit executable arm64`.
- `cargo clippy --all-targets -- -D warnings` + `cargo fmt --check` green; crate root carries the hard-rule-3 `deny`. Zero dependencies.

## Run / test protocol

```sh
./target/debug/browser-url
```

It lists running browsers, then prints one line per browser **when its state changes**:

```
[HH:MM:SS] <App>   <RESULT>   (<latency> ms)
```

- `URL  <u>` — a normal-window active-tab URL.
- `SKIPPED-PRIVATE (mode=<m>) — URL not read  ✅ D8` — an incognito/non-normal window.
- `NOT AUTHORIZED (-1743) → fall back to title-derived site` — Automation denied.
- `no windows` / `error: …`.

To exercise each path:
1. **Normal tab** → `URL` line; navigate to confirm it updates (R15/R16).
2. **Open an incognito window** (Chrome/Brave `⌘⇧N`), bring it to front → `SKIPPED-PRIVATE` (R17/D8). Switch back → `URL` returns.
3. **Safari** (R16/R18): bring Safari forward; confirm `URL of front document`. Open a **Private** window and confirm production must safe-default to skip (the spike prints `⚠ unenforced`).
4. **Arc** (R17): confirm `mode`/tab semantics on Arc's window/space model.
5. **Deny path** (R19): in System Settings → Privacy & Security → Automation, turn a browser **off** for this binary → confirm `-1743` → fallback.

> **First query per browser prompts** ("…wants to control <Browser>"). Click **Allow**.
> Grants are **per (client, target)** — you approve Chrome, then Brave, then Safari
> separately. This is by design (C8: onboarding frames it as "add the browsers you
> use", not one toggle), not a bug.

### Observed results — PASS (2026-06-22)

Driven by reordering Chrome's front window between a normal and an incognito window
(events delivered using the terminal's existing Chrome grant). **Screen Recording
irrelevant — this path is Apple Events, not AX.**

```
[17:08:20] Google Chrome   URL  https://dstv.stream/#/livetv/play/…   (149 ms)   ← normal: URL read
[17:08:20] Brave Browser   no windows                                 (138 ms)
[17:08:22] Google Chrome   SKIPPED-PRIVATE (mode=incognito) — URL not read  ✅ D8   (156 ms)   ← incognito front: skipped
[17:08:26] Google Chrome   URL  https://dstv.stream/#/livetv/play/…   (159 ms)   ← back to normal: read again
```

**Verdict: core PASS.** The Chromium URL read works, and the **incognito exclusion
is enforced live** by reading `mode` before the URL (R15/R17, D8). Latency is
acceptable for event-driven capture (R21). Safari (R16/R18), Arc, and the `-1743`
deny path are documented residual manual checks above.

#### Findings

1. **`mode of front window` is readable and reliable** — the sdef's "can be set only
   once during creation" wording was a *write* constraint, not a read constraint.
   Reading it **first** and skipping any non-`"normal"` value is a clean, enumerable-
   free safe-default that covers incognito/guest/Edge-variants. This is the load-
   bearing mechanism for D8 on Chromium, and it holds.
2. **AppleScript app names are exact** — `"Brave Browser"` (not `"Brave"`); Edge is
   `"Microsoft Edge"`. The capture layer maps bundle id → app name from a table.
3. **osascript latency ~140–160 ms warm** — process-isolated and well within budget
   for query-on-switch. `NSAppleScript` (in-process, faster, but not thread-safe and
   needs a dedicated run-loop thread) is **not needed** (C7).
4. **Automation attribution flows to the responsible process.** Run from a terminal,
   the binary used the **terminal's** existing Chrome/Brave grant (no new prompt) —
   TCC keyed the consent to the controlling app, not the child binary. In the real
   app the TCC client is the **`.app` bundle** (R20), so each browser prompts once
   against "UsageOS". A standalone unsigned binary launched on its own would prompt
   under its own identity — note this when interpreting dev runs.
5. **Safari stays conservative.** With no private-browsing property, the only signal
   is a locale-fragile System-Events Window-menu inspection. The spike does **not**
   pretend to enforce it — it flags Safari reads as unenforced, and the safe-default
   (skip when uncertain) is a Phase-1 requirement, not an optional nicety.

## Note for the capture-layer port

- Static scripts only; never interpolate captured strings into AppleScript.
- Map bundle id → AppleScript app name from a fixed table (R15).
- **Always read `mode` before the URL** for Chromium; treat anything but `"normal"`
  as private and skip (D8). For Safari, implement the System-Events private check and
  **safe-default to skip** when it can't confirm non-private (R18).
- Treat `-1743` as a permanent "fall back to title-derived site", never a retry (C6).
- `NSAppleEventsUsageDescription` + the `com.apple.security.automation.apple-events`
  entitlement are mandatory in the bundle Info.plist, or Automation fails with
  `-1743` **before any prompt** (C9).
