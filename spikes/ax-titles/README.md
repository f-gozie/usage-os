# Spike #1 — AX focused-window titles WITHOUT Screen Recording

> **Status: ✅ RUN — PASS (2026-06-22, macOS / Apple Silicon).** AX returns real,
> non-empty focused-window titles for **every app class tested** — Chromium
> (Chrome, Brave), Electron editors/apps (Cursor, Claude, Notion, Figma, WhatsApp,
> Spotify), native (Finder), terminals (iTerm2) — with **only Accessibility
> granted and Screen Recording OFF**. Results + findings in
> [Observed results](#observed-results--pass-2026-06-22) below. This crate is
> **isolated**: it is *not* a member of the `src-tauri` workspace.

## What this proves (and why it's make-or-break)

The entire UsageOS redesign — recap, day-dial, local narration — assumes we can
read the **focused window title** of the frontmost app using the macOS
**Accessibility (AX)** permission *alone*, with **Screen Recording OFF**.

Today the app reads titles via [`active-win-pos-rs`], which falls back to
`CGWindowList`. `CGWindowList` only returns window *titles* when **Screen
Recording** is granted — otherwise titles come back **empty**. Asking users for
Screen Recording (the "your screen is being observed" permission) to run a
*local, private* time tracker is a trust-killer and contradicts the product's
whole promise. So the foundational question is:

> Does AX return a **real, non-empty** `AXTitle` for the focused window of
> Chromium/Electron apps (Chrome, Brave, Arc, Cursor, VS Code), terminals
> (iTerm2, Terminal), and native apps (Safari, Finder), with **only
> Accessibility** granted and **Screen Recording explicitly OFF**?

If yes → the redesign's capture layer is viable as designed. If some apps return
`EMPTY`/`NIL` → we learn *exactly which*, and can plan fallbacks before building
on the assumption. Either way, this spike turns the #1 unknown into a fact.

This binary **never** touches `CGWindowList` or Screen Recording. The only OS
surfaces it uses are `NSWorkspace` (who is frontmost) and the AX API
(`AXUIElementCopyAttributeValue`). No network, no disk writes, no DB.

[`active-win-pos-rs`]: https://crates.io/crates/active-win-pos-rs

## What it does

Once per ~1.5s tick:

1. **Trust:** call `AXIsProcessTrusted()`. If not trusted, call
   `AXIsProcessTrustedWithOptions({ kAXTrustedCheckOptionPrompt: true })` to fire
   the system prompt, then poll until trust is granted.
2. **Pump the run loop** (`CFRunLoop::run_in_mode`, ~0.3s) so NSWorkspace's
   frontmost-app tracking refreshes, then read
   `NSWorkspace::sharedWorkspace().frontmostApplication()` → `localizedName`,
   `bundleIdentifier`, `processIdentifier`. (Without the pump the value stays
   frozen on the launching app — see [Findings](#findings).)
3. **Title via AX:** `AXUIElement::new_application(pid)` →
   copy `"AXFocusedWindow"` → copy `"AXTitle"`.
4. **Print one classified line** per tick:

   ```
   16:42:07  trusted=true   app=Google Chrome         bundle=com.google.Chrome              pid=53120  title=REAL("New tab - Google Chrome")
   16:42:09  trusted=true   app=Finder                bundle=com.apple.finder               pid=410    title=REAL("Downloads")
   16:42:11  trusted=true   app=Terminal              bundle=com.apple.Terminal             pid=901    title=AXERR(NoValue)
   ```

   The title is classified as:
   - `REAL("…")` — a non-empty title string (**the win we're hoping for**).
   - `EMPTY` — `AXTitle` resolved to `""`.
   - `NIL` — the focused window or its title was absent / not a string.
   - `AXERR(<variant>)` — an AX error code, named (e.g. `NoValue`,
     `AttributeUnsupported`, `CannotComplete`, `APIDisabled`). These are
     **expected outcomes to classify**, not crashes.

No `unwrap()` / `expect()` / `panic!` in the logic — every AX outcome is a value
we print.

## Build

```sh
cd spikes/ax-titles
cargo build
```

- **Built binary:** `spikes/ax-titles/target/debug/ax-titles`
- **Build result (this machine, `aarch64-apple-darwin`, Rust 1.87.0):**
  clean `cargo build` after `cargo clean`, plus `cargo clippy --all-targets -- -D warnings`
  and `cargo fmt --check` both green.

  ```
  Compiling ax-titles v0.1.0 (…/spikes/ax-titles)
   Finished `dev` profile [unoptimized + debuginfo] target(s) in 26.16s
  $ file target/debug/ax-titles
  target/debug/ax-titles: Mach-O 64-bit executable arm64
  ```

> `docs.rs` may render these `objc2-*` crates as x86_64-only — that is a docs
> display artifact. They build fine for `aarch64-apple-darwin` (Apple Silicon),
> which is what we target.

## Granting Accessibility for this dev binary

AX is gated by **TCC** (Transparency, Consent & Control). A CLI binary has no
app bundle, so TCC keys the grant to the **binary's code-signing identity**
(for an unsigned dev build, effectively its path + ad-hoc signature). Two ways
to grant:

**Option A — let the binary prompt you (simplest):**

```sh
./target/debug/ax-titles
```

On first run it calls the prompt variant, and macOS shows
*"'ax-titles' would like to control this computer using accessibility features."*
Click **Open System Settings**, flip the toggle **on** for `ax-titles`. The
binary is polling — it prints `...still waiting for Accessibility` until you
grant, then starts capturing. (You may need to quit and re-run once after the
first grant so the trust check picks it up.)

**Option B — add it manually:**
System Settings → Privacy & Security → **Accessibility** → **+** →
navigate to `spikes/ax-titles/target/debug/ax-titles` → enable the toggle.

### ⚠️ The R14 rebuild footgun

TCC keys the Accessibility grant to the binary's **code-signing identity**. A
plain `cargo build` of an unsigned binary produces an *ad-hoc* signature that
can change across rebuilds, so **rebuilding may silently detach the grant** —
the toggle in System Settings still looks "on", but `AXIsProcessTrusted()`
returns `false`. Symptoms: every app reads `AXERR(CannotComplete)` /
`AXERR(APIDisabled)` even though Settings shows the app enabled.

**Simplest reliable path found here:** treat the *built binary path* as the unit
of grant, and after any rebuild that misbehaves, **reset and re-grant** (below).
Don't `cargo clean` between test runs unless you intend to re-grant. If you want
grant stability across rebuilds, ad-hoc **self-sign a stable identity** once:

```sh
cargo build
codesign --force --sign - target/debug/ax-titles   # stable ad-hoc identity
```

…then grant that binary. (For the real app this is moot — it ships as a properly
signed `.app` bundle, and TCC keys to the bundle's stable Developer ID.)

## Test protocol

With **Screen Recording OFF** (verify in System Settings → Privacy & Security →
Screen Recording — `ax-titles` must NOT be listed/enabled) and **Accessibility
ON** for `ax-titles`:

1. Run `./target/debug/ax-titles`.
2. Switch the frontmost app through each target below, pausing ~2s on each so a
   tick lands while it's frontmost. For each, open a window with a recognizable
   title (a named tab / a file / a folder):

   | App        | Bundle id (expected)         | Notes for the test                    |
   |------------|------------------------------|---------------------------------------|
   | Chrome     | `com.google.Chrome`          | Open a page with a clear `<title>`    |
   | Brave      | `com.brave.Browser`          | Open a page with a clear `<title>`    |
   | Arc        | `company.thebrowser.Browser` | Open a page / space                   |
   | Cursor     | `com.todesktop.230313mzl4w4u92` (varies) | Open a named file        |
   | VS Code    | `com.microsoft.VSCode`       | Open a named file                     |
   | iTerm2     | `com.googlecode.iterm2`      | Title may follow shell/SSH/tab title  |
   | Terminal   | `com.apple.Terminal`         | Title may follow the tab title        |
   | Safari     | `com.apple.Safari`           | Open a page with a clear `<title>`    |
   | Finder     | `com.apple.finder`           | Open a named folder window            |

3. **Capture the printed lines** and record `REAL` / `EMPTY` / `NIL` /
   `AXERR(…)` per app. Paste the log into the PR / spike findings.

### Observed results — PASS (2026-06-22)

Run on macOS / Apple Silicon, `trusted=true`, **Screen Recording OFF**. Every app
class returned a real, non-empty title:

| App | Bundle id | Class | Result |
|-----|-----------|-------|--------|
| Google Chrome | `com.google.Chrome` | Chromium | `REAL` — full tab title incl. page title + profile |
| Brave Browser | `com.brave.Browser` | Chromium | `REAL("New Tab - Brave - Favour")` |
| Cursor | `com.todesktop.230313mzl4w4u92` | Electron editor | `REAL("Browser Tab — nudge")` — carries the project name |
| Claude | `com.anthropic.claudefordesktop` | Electron | `REAL("Claude")` |
| Notion | `notion.id` | Electron | `REAL("Stakeholder Management ")` |
| Figma | `com.figma.Desktop` | Electron | `REAL("Stakeholder Management - Stigdata - Final")` |
| WhatsApp | `net.whatsapp.WhatsApp` | Electron | `REAL("‎WhatsApp")` (stray LTR mark) |
| Spotify | `com.spotify.client` | Electron/CEF | `REAL("Spotify Premium")` |
| Finder | `com.apple.finder` | native | `REAL("Downloads")` |
| iTerm2 | `com.googlecode.iterm2` | terminal | `REAL` |

VS Code, Safari, Arc, and Terminal were not hit directly, but each is covered by a
same-engine proxy that passed (Cursor is a VS Code fork; Chrome/Brave cover the
Chromium browsers; Finder covers native). **Verdict: R4 confirmed feasible — the
capture redesign's core premise holds. No Screen Recording needed.**

#### Findings

- **A run loop is required** to track app switches. Early runs read
  `NSWorkspace::frontmostApplication` with no run loop and it stayed frozen on the
  launching terminal (iTerm2). Pumping `CFRunLoop::run_in_mode` each tick fixed it.
  Directly confirms audit risk **R6** — the real capture layer must own a run loop
  (NSWorkspace activation + AXObserver), which it will.
- **System-wide `AXFocusedApplication` returned `AXERR(CannotComplete)`** from this
  plain CLI; reading the title from the frontmost app *by pid* works. Prefer the
  per-pid path.
- **Titles already carry rich project/page signal** (Cursor → "nudge"; Chrome →
  the GitHub PR title / Gmail subject) — strong for project inference (D6) even
  before the Automation/URL path is wired.
- **Titles also carry sensitive content** — the Chrome line exposed a Gmail subject
  + email address. Confirms **D8** (exclusion list / per-app Private /
  incognito-never-recorded) is load-bearing, not optional. The capture layer should
  also strip stray bidi/control marks (the `‎` in WhatsApp's title).

### Clean re-test reset

To wipe the Accessibility grant and re-test from a clean slate:

```sh
# By binary path / id (unsigned dev binary):
tccutil reset Accessibility

# ^ With no identifier, this resets Accessibility for ALL apps — broad but
#   reliable for a dev binary that has no stable bundle id. If you self-signed
#   with a bundle identifier, target it specifically:
# tccutil reset Accessibility <your.bundle.id>
```

Then re-grant via Option A or B above.

## Crate / API choices

Chosen surface: **`objc2-application-services` + `objc2-core-foundation` +
`objc2-app-kit`** (the higher-level objc2 family) — *not* the lower-level
`accessibility-sys` + `core-foundation` raw-FFI option. Why:

- It is **simpler for a polling title read**: `AXUIElement::new_application(pid)`
  hands back an owned `CFRetained<AXUIElement>` (auto-`CFRelease` on drop),
  `copy_attribute_value` returns a typed `AXError`, and `CFType::downcast_ref::<CFString>()`
  gives a safe, checked cast of the returned value. No manual `CFRelease`, no
  `TCFType` boilerplate.
- It keeps the spike on the **same `objc2` family the redesign already commits
  to** (CLAUDE.md: "Native macOS access via `objc2`"), so what compiles here is
  directly portable into the capture layer.
- `NSWorkspace` / `NSRunningApplication` live in `objc2-app-kit`, which is in the
  same family — one coherent dependency set, no mixing FFI styles.

**Resolved versions (all the provisional pins held):**

| Crate                        | Pin       | Resolved |
|------------------------------|-----------|----------|
| `objc2`                      | `0.6`     | 0.6.4    |
| `objc2-app-kit`              | `0.3.2`   | 0.3.2    |
| `objc2-foundation`           | `0.3.2`   | 0.3.2    |
| `objc2-application-services` | `0.3.2`   | 0.3.2    |
| `objc2-core-foundation`      | `0.3.2`   | 0.3.2    |

**Feature flags that mattered (not obvious from the pins):**

- `objc2-app-kit`: `NSWorkspace`, `NSRunningApplication`, **`libc`** — the last
  one is required for `NSRunningApplication::processIdentifier` (returns
  `libc::pid_t`).
- `objc2-application-services`: `AXUIElement`, **`AXError`** (gates
  `copy_attribute_value`, which returns `AXError`), **`libc`** (gates
  `AXUIElement::new_application(pid)`).
- `objc2-core-foundation`: `CFString`, `CFDictionary`, **`CFNumber`** (the last
  pulls in `CFBoolean` / `kCFBooleanTrue` for the prompt-options dictionary).

The AX attribute-name constants (`kAXFocusedWindowAttribute`, `kAXTitleAttribute`)
are **not** re-exported by `objc2-application-services`, exactly as the brief
warned — so the spike builds CFStrings from the literals `"AXFocusedWindow"` and
`"AXTitle"`, which is the documented attribute-name format.

### Compile surprises (worth noting for the capture layer)

1. **`AXUIElementCreateApplication` (free fn) is deprecated** in favor of
   `AXUIElement::new_application`, and — despite the FFI returning
   `Option<NonNull<…>>` — the *generated wrapper returns a bare
   `CFRetained<AXUIElement>`* (it `expect()`s internally on null). To keep panic
   surface out of *our* code we use the method form. The free function would have
   forced us to handle an `Option` that the wrapper doesn't actually expose.
2. **The `NSWorkspace` / `NSRunningApplication` accessors are SAFE** (not
   `unsafe`) in `objc2-app-kit` 0.3.2 — wrapping them in `unsafe` triggers
   `unused_unsafe`. Only the raw AX C functions and the CF pointer reborrows are
   `unsafe`.
3. **`AXIsProcessTrustedWithOptions` wants an untyped `&CFDictionary`**, but
   `CFDictionary::from_slices` produces a typed `CFDictionary<CFString, CFBoolean>`.
   They share layout (generics are `PhantomData`), so we reborrow the retained
   pointer as the base type — one small, documented `unsafe`.
