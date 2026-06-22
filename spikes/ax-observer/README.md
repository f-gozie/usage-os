# Spike ② — the event-driven capture model

> **Status: ✅ RUN — PASS (2026-06-22, macOS / Apple Silicon).** The real capture
> architecture works end-to-end with **Accessibility only, Screen Recording OFF**:
> the `NSWorkspace` activation block fires on every app switch, a per-PID
> `AXObserver` delivers focus/window-change notifications through the run loop, a
> chatty-title debounce coalesces duplicates, and every event marshals to a
> **Tokio** consumer thread over a `Send` channel without blocking. Crucially this
> works **without an `NSApplication`** — a bare run loop is enough. Results +
> findings in [Observed results](#observed-results--pass-2026-06-22). This crate is
> **isolated**: it is *not* a member of the `src-tauri` workspace.

## What this proves (and why it matters)

Spike #1 ([`../ax-titles`](../ax-titles)) proved AX returns real focused-window
titles with Accessibility alone — but it **polled** on a timer and **pumped** the
run loop as a stopgap. That is not the architecture we ship. This spike builds the
**event-driven** model the Phase-1 `capture` trait impl will use, and settles the
threading/observer questions the feasibility audit flagged as uncertain:

| Risk | Question | Result |
|------|----------|--------|
| **R6**  | A working threading model: AX + NSWorkspace on a run loop, results marshaled to the async (Tokio) side **without blocking the executor**? | ✅ Proven. Run loop on the main thread; events `send` over an unbounded channel to a Tokio runtime on its own thread. |
| **R8**  | Observe `NSWorkspaceDidActivateApplicationNotification` from Rust; callback fires per switch with the new app's pid/name/bundle? | ✅ Fired on all 6 switches; pid/name/bundle correct each time. |
| **R9**  | The observer is a Rust closure in `block2::RcBlock`; token retained, no UAF across switches? | ✅ Block fired repeatedly, no crash; token + block kept alive for the run. |
| **R10** | A per-PID `AXObserver` fires focus/title-change notifications; rebuilt on PID change; title observer re-pointed at the new focused window? | ✅ Fresh observer per pid; `AXFocusedWindowChanged` delivered through the run-loop source; title re-registration ran. |
| **R11** | Event-driven (no polling); chatty title storms debounced; near-zero idle? | ✅ `coalesced` counter shows duplicate suppression; idle measured **0.0% CPU** (`ps`). `powermetrics` wakeup count = documented manual step below. |
| **R13** | Compiles **and runs** on `aarch64-apple-darwin`? | ✅ `arm64` Mach-O, ran on Apple Silicon. |

This binary **never** touches `CGWindowList` / Screen Recording, the network, or
disk. Its only OS surfaces are `NSWorkspace` (frontmost + activation notifications)
and the AX API (`AXUIElement` + `AXObserver`).

> **R12 (idle detection, `user-idle`)** is intentionally **out of scope here** — it
> already shipped in v0.1.0, reads CG aggregate idle time (no run loop, no extra
> permission), and is orthogonal to this event-driven path. Re-proving it in this
> spike would add a dependency for no new information.

## Architecture (and how it maps onto the real app)

```
         ┌─────────────────────── main thread = the run loop ────────────────────────┐
         │                                                                            │
         │  NSWorkspace.notificationCenter                                            │
         │    └─ addObserverForName(DidActivateApplication, block: RcBlock) ──┐       │
         │                                                                    ▼       │
         │   on activation:  CaptureState::switch_to(new pid)                         │
         │     ├─ drop old AppObserver  (Drop removes its run-loop source)            │
         │     └─ install new AppObserver:                                            │
         │          AXObserverCreate(pid, observer_callback)                          │
         │          add_notification(app,   AXFocusedWindowChanged / Main / FocusedUI)│
         │          add_notification(window, AXTitleChanged)   ← re-pointed on switch │
         │          CFRunLoopAddSource(run_loop, observer.run_loop_source)            │
         │                                                                            │
         │   AX callback (focus/title) ─► read AXTitle ─► dedupe debounce ─► tx.send ─┼──┐
         └────────────────────────────────────────────────────────────────────────────┘  │
                                                              Send channel (UnboundedSender)│
         ┌──────────────── consumer thread = the async side ──────────────────────────┐  │
         │   Tokio runtime · while let Some(ev) = rx.recv().await { print } ◄──────────┼──┘
         └────────────────────────────────────────────────────────────────────────────┘
```

**Porting note.** The spike runs `CFRunLoop::run()` on the **main** thread because a
standalone CLI has no host loop. In the real app, **Tauri already owns a main-thread
`NSApplication` run loop** — so the `capture` impl registers the same NSWorkspace
observer and AXObserver sources during Tauri `setup` and does **not** call `run()`.
Everything else (the block, the per-PID observer rebuild, the `Send` channel to the
existing Tokio runtime) ports directly. The spike proves activation delivery works
with a bare run loop and **no `NSApplication`**, so the Tauri loop (which is a
superset) is more than sufficient.

## Crate / API choices — AXObserver lives in the **objc2 family**

The audit's R10 row tentatively cited `accessibility-sys 0.2.0` for the observer
API. That turned out to be unnecessary: **`objc2-application-services` 0.3.2 exposes
the full `AXObserver` surface** —

- `AXObserver::create(pid, callback, out)` (the Create-rule constructor),
- `observer.add_notification(element, name, refcon)` / `remove_notification(...)`,
- `observer.run_loop_source()` → `CFRetained<CFRunLoopSource>`,
- callback type `AXObserverCallback = Option<unsafe extern "C-unwind" fn(NonNull<AXObserver>, NonNull<AXUIElement>, NonNull<CFString>, *mut c_void)>`.

So the spike stays on the **same `objc2` family Spike #1 chose** (CLAUDE.md: "Native
macOS access via `objc2`") — no `accessibility-sys`, one coherent FFI style, directly
portable into the capture layer. The activation observer adds `block2 0.6` (`RcBlock`)
and the spike adds `tokio` to stand in for the app's async side.

**Feature flags that mattered:**

- `objc2-foundation`: `NSString` + `NSNotification` + **`NSOperation`** + **`block2`**
  — the last three gate `addObserverForName:object:queue:usingBlock:`.
- `objc2-application-services`: `AXUIElement` + **`AXError`** (gates the observer
  calls) + **`libc`** (gates `AXObserver::create(pid, …)`).
- `objc2-core-foundation`: `CFString`, **`CFRunLoop`** (the `CFRunLoopSource` type +
  `kCFRunLoopCommonModes`), `CFDictionary`/`CFNumber` (the trust-prompt options).

The AX **attribute and notification names** are not re-exported (same as Spike #1) —
we build `CFString`s from the literals (`"AXFocusedWindowChanged"`, `"AXTitleChanged"`,
`"AXFocusedWindow"`, `"AXTitle"`).

## Build

```sh
cd spikes/ax-observer
cargo build
codesign --force --sign - target/debug/ax-observer   # R14: stable ad-hoc identity so the AX grant sticks across rebuilds
```

- **Binary:** `spikes/ax-observer/target/debug/ax-observer` — `Mach-O 64-bit executable arm64`.
- `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` both green.
- The crate root carries `#![cfg_attr(not(test), deny(clippy::unwrap_used, expect_used, panic))]`
  (hard rule 3) — there are no `unwrap`/`expect`/`panic!` in the logic.

See [`../ax-titles/README.md`](../ax-titles/README.md#-the-r14-rebuild-footgun) for the
**R14 rebuild footgun** (re-sign + re-grant after a rebuild that misbehaves) and the
`tccutil reset Accessibility` clean-slate reset — they apply identically here.

## Run / test protocol

With **Screen Recording OFF** and **Accessibility ON** for `ax-observer`:

```sh
./target/debug/ax-observer
```

It prints its own pid, installs the observer for the current frontmost app (one
`ACTIVATED` line), then **switch between apps and change window titles**. Each event
prints one line on the Tokio consumer:

```
[#<seq> HH:MM:SS] <KIND>  app=<name>  bundle=<id>  pid=<pid>  via=<notification>  title=<REAL("…")|EMPTY|NIL|AXERR(…)>  (coalesced N)
```

- **KIND**: `ACTIVATED` (app switch) · `FOCUS-WIN` (focused/main window changed) ·
  `TITLE` (title changed in place) · `OBS-FAIL` (observer couldn't be created).
- **`(coalesced N)`**: N chatty duplicate title events were suppressed before this one.

To exercise the paths:
1. **App switches** → `ACTIVATED` per switch (R8/R9).
2. **Open / focus a different window in the same app** → `FOCUS-WIN` (R10).
3. **Navigate browser tabs / pages so a title changes while the app stays frontmost**
   → `TITLE` events (the in-place `AXTitleChanged` path).
4. **Idle wakeups (R11):** while NOT touching the machine, in another terminal:
   ```sh
   sudo powermetrics --samplers tasks -i 1000 -n 5 | grep -i ax-observer
   ```
   An event-driven capture should sit near **0 wakeups/s** when idle.

### Observed results — PASS (2026-06-22)

Driven by `open -a` activations (Launch Services — no Automation prompt) on a real
Apple-Silicon Mac, `trusted=true`, **Screen Recording OFF**:

```
[#1 ACTIVATED app=Claude         bundle=com.anthropic.claudefordesktop pid=85819 via=NSWorkspaceDidActivateApp  title=REAL("Claude")
[#2 ACTIVATED app=Finder         bundle=com.apple.finder               pid=1029  via=NSWorkspaceDidActivateApp  title=REAL("Favour’s MacBook Pro")
[#3 FOCUS-WIN app=Finder         bundle=com.apple.finder               pid=1029  via=AXFocusedWindowChanged     title=REAL("Downloads")  (coalesced 3)
[#4 ACTIVATED app=Cursor         bundle=com.todesktop.230313mzl4w4u92  pid=57241 via=NSWorkspaceDidActivateApp  title=REAL("Browser Tab — nudge")
[#5 ACTIVATED app=Brave Browser  bundle=com.brave.Browser              pid=54439 via=NSWorkspaceDidActivateApp  title=REAL("New Tab - Brave - Favour")
[#6 ACTIVATED app=Finder         bundle=com.apple.finder               pid=1029  via=NSWorkspaceDidActivateApp  title=REAL("Downloads")
[#7 ACTIVATED app=Claude         bundle=com.anthropic.claudefordesktop pid=85819 via=NSWorkspaceDidActivateApp  title=REAL("Claude")
```

**Verdict: PASS.** Every switch produced an `ACTIVATED` event (R8/R9); each app got a
distinct per-PID observer (R10); the `AXFocusedWindowChanged` notification was
**delivered through the run-loop source** (#3) and the debounce suppressed 3 chatty
duplicates before emitting the distinct `"Downloads"` title (R11); all events marshaled
to the Tokio consumer (R6); idle CPU was **0.0%** (`ps`). Built + ran on **arm64** (R13).

#### Findings

1. **No `NSApplication` required.** A bare `CFRunLoop::run()` is enough for NSWorkspace
   to deliver activation notifications — the audit's worry (does activation work without
   a full app object?) is answered **yes**. The real Tauri loop is a strict superset.
2. **AXObserver belongs to `objc2-application-services`** — `accessibility-sys` is not
   needed. Refines the architecture doc / R10 (see [Decisions](../../context/decisions.md)).
3. **In-app focus changes need the title observer re-pointed.** `AXTitleChanged`
   registered on the *application* element does not fire for window title changes — it
   must be registered on the focused **window**, and re-registered when the focused
   window changes. The spike does exactly this (see `reregister_title_observer`).
4. **`AXFocusedWindowChanged` delivery is proven; pure in-place `AXTitleChanged` is the
   one residual manual check** — the automated `open -a` sweep changes windows, not
   titles-in-place. The machinery is registered and the run-loop source demonstrably
   delivers; confirm `TITLE` events with a browser-navigation pass.
5. **Debounce is load-bearing even at this scale** — a single Finder window settle
   produced 3 duplicate title reads (`coalesced 3`). Dedupe-identical is the simplest
   storm-killer; production may add a trailing-edge timer to also coalesce rapid
   *distinct* titles (e.g. progress %), which the spike deliberately omits to avoid
   hiding data and adding run-loop timers.
6. **Title sanitization is wired in** — control chars + bidi/zero-width marks are
   stripped (Spike #1's WhatsApp-LTR-mark finding), so titles are clean before they
   reach the channel.

## Lifecycle / safety notes (for the capture-layer port)

- **Observer teardown order.** `AppObserver::drop` removes the run-loop source **first**
  (synchronously, on the run-loop thread) so no pending callback can fire against the
  boxed `CallbackContext` (the live AX `refcon`) while it is being freed — no UAF across
  app switches (R9).
- **`refcon` stability.** The `CallbackContext` is heap-`Box`ed; its address is captured
  as the `refcon` and stays valid across the `Box` move into `AppObserver` (moving a
  `Box` moves the 8-byte handle, not the pointee).
- **No `&mut` aliasing through `refcon`.** The C callback reconstructs only a *shared*
  `&CallbackContext` and mutates via `RefCell`/`Cell`, so the `*mut`→`&` round-trip is
  sound.
- **Single-threaded capture state.** `CaptureState` / `CallbackContext` are touched only
  on the run-loop thread (`Rc`/`RefCell`, not `Arc`/`Mutex`); only the `CaptureEvent`
  (all-owned, `Send`) crosses the thread boundary.
