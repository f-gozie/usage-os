# Handoff — start here for the next session

_Updated 2026-06-22 at the end of the **Spike ② session**. Read this, then `CLAUDE.md`, then `context/feasibility/2026-06-22-feasibility-audit.md`, then the redesign plan. Supersedes the prior (Phase-0 spike) handoff._

## 1. Where we are (TL;DR)

**The native capture gate is COMPLETE — all four spikes PASS** on the real Apple-Silicon Mac:
- **Spike #1** (merged, PR #6): AX returns real focused-window titles with **Accessibility only, Screen Recording OFF**, for every app class. R4 retired.
- **Spike ②** (this session, **uncommitted**): the **event-driven capture model** — NSWorkspace activation + per-PID AXObserver on the main run loop, marshaled to Tokio over a `Send` channel. R6/R8/R9/R10/R11/R13.
- **Spike ③** (this session, **uncommitted**): **browser URL + incognito exclusion** — Chromium active-tab URL via Apple Events; `mode of front window` read **first** excludes incognito (D8 enforced live). R15/R17/R21. Safari/Arc/-1743 = documented residual checks.
- **Spike ④** (this session, **uncommitted**): **`proc_pidinfo` cwd read** — a non-root, unsandboxed process reads another process's cwd, no `EPERM`, matching `lsof` (shell in `…/usage_os` → that path). R22 resolved; no TCC grant; R24 fallbacks not needed.

The feasibility verdict was **GO-WITH-CAVEATS**; with the whole native gate passed it now **leans GO**. The three capture signals — title (①), URL (③), cwd (④) — are all proven, on the proven event-driven model (②).

The **project-inference spike** also ran (snapshot): zero false assignments on real signals, abstain threshold set, and it surfaced D30 (canonicalize project identity on the git remote). **Phase 0 is essentially complete; Phase 1 is next.**

`origin/main` is at `a2f2a7f` (3 PRs from last session). **This session's work (Spikes ②+③+④ + the project-inference spike + doc updates) is uncommitted** — see §8.

## 2. The headline result — Spike ② PASSED (R6, R8–R11, R13)

`spikes/ax-observer/` (isolated crate, mirrors `ax-titles`) proves the **real** capture architecture the Phase-1 `capture` trait impl will use — the polling/run-loop-pump in Spike #1 was a stopgap. Driven by automated `open -a` app switches plus a smoke run; full results + protocol in `spikes/ax-observer/README.md`:

| Risk | Proven |
|---|---|
| **R8** | NSWorkspace `didActivateApplication` block fires on **every** switch (Finder→Cursor→Brave→…), correct pid/name/bundle. |
| **R9** | The observer is a `block2::RcBlock` Rust closure; fired across 6 switches, token retained, no UAF. |
| **R10** | A fresh per-PID `AXObserver` per app; `AXFocusedWindowChanged` delivered through the run-loop source; title observer re-pointed on window change. |
| **R11** | Dedupe debounce coalesced chatty title duplicates (observed `coalesced 3`); idle **0.0% CPU**. |
| **R6**  | Every event marshaled to a Tokio consumer thread over an unbounded `Send` channel; run loop never blocked. |
| **R13** | `arm64` Mach-O, ran on Apple Silicon. |

**Three findings that shaped decisions (now in `context/decisions.md` D29):**
1. **`AXObserver` lives in `objc2-application-services` 0.3.2** (`create` / `add_notification` / `run_loop_source`) → the whole AX surface stays in one objc2 family; **`accessibility-sys` is dropped**. Corrects `architecture.md` + the capture standard.
2. **No `NSApplication` needed** — a bare `CFRunLoop` delivers activation events. In the app, register sources into **Tauri's main run loop** during `setup` (don't call `run()`). Tauri's loop is a strict superset.
3. **`AXTitleChanged` must be registered on the focused *window*** (re-pointed on window change), not the app element, or in-app title changes never fire.

**One residual manual check:** pure in-place `AXTitleChanged` delivery (a browser-navigation pass — change a tab/page title while the app stays frontmost and confirm `TITLE` events). The machinery is in place and `AXFocusedWindowChanged` delivery is proven; this just exercises the last notification type.

## 3. Decisions made this session (in `context/decisions.md`)

- **D29** — Event-driven capture model: `objc2-application-services` `AXObserver` on the **main run loop**, NSWorkspace activation via `block2::RcBlock`, marshaled to async over a `Send` channel; `accessibility-sys` dropped; no `NSApplication`. Refines D5; proven by Spike ②.
- **D30** — Project identity canonicalized on the **git remote** (`owner/repo`), with folder/title/URL as aliases; **abstain threshold** = assign only on HIGH/MED unambiguous signals else `unassigned`; `ambiguous` (localhost/dashboards) is a distinct third state for Phase-2 correlation. Refines D6; from the project-inference spike.

## 4. What changed on disk this session (read order for next session)

New / edited (all **uncommitted**):
- `spikes/ax-observer/{Cargo.toml,src/main.rs,README.md,.gitignore}` — Spike ②. README has the results table, architecture diagram, Tauri port path, run/test protocol.
- `spikes/browser-url/{Cargo.toml,src/main.rs,README.md,.gitignore}` — Spike ③. Zero-dep pure-std crate shelling osascript; README has the fallback ladder + protocol.
- `spikes/proc-cwd/{Cargo.toml,src/main.rs,README.md,.gitignore}` — Spike ④. `libc`-only crate; README has the `lsof`-verified results + the pid-selection note.
- `spikes/project-inference/{Cargo.toml,src/main.rs,README.md,.gitignore}` — Spike #5. Pure-std crate (shells `git`); README has the precision/abstain results + the canonicalization finding.
- `context/decisions.md` — added **D29** (event-driven capture model) + **D30** (project identity canonicalized on git remote; abstain threshold).
- `context/plans/2026-06-22-product-redesign.md` — Spikes ② + ③ checked off ✅ with summaries; status line refreshed.
- `context/architecture.md` — corrected the AX observer crate (objc2, not accessibility-sys) + marked the title/event model proven.
- `context/standards/capture-and-permissions.md` — resolved open questions #1–#5 + #7 (observer crate, Electron titles, run-loop model, aarch64, Chromium `mode` readability, osascript latency); #6 (Safari private) stays open; refined the AXObserver registration note; updated the crate table.

Still the map: `context/feasibility/2026-06-22-feasibility-audit.md` (risk register **R1–R83**, spike plan **§4**). Per the Spike-#1 precedent, the dated audit table is left as a point-in-time snapshot — spike results are tracked in the plan + spike READMEs, not edited into the audit.

## 5. Code / build state

- Both new spikes build clean for **aarch64**, `cargo clippy -D warnings` + `cargo fmt --check` green, crate roots carry the hard-rule-3 `deny`. `ax-observer` re-signed ad-hoc (`codesign --force --sign -`) so the AX grant sticks (R14). `browser-url` is **zero-dependency** (pure std + osascript).
- AX trust is **already granted** for `ax-observer` (smoke run printed "already granted"). Automation grants for `browser-url` were given to the **terminal** during testing (TCC attributes to the responsible process) — Chrome + Brave; the binary used them.
- Nothing in `src-tauri` changed — the app is untouched; both spikes are fully isolated (`[workspace]` table, not members).
- Existing app: ~25 Rust + 28 TS tests, CI green (Linux/macOS/Windows) with the clippy/fmt/tsc gates from PR #5.

## 6. NEXT SESSION — do these, in order, thoroughly

**Phase 0 is essentially COMPLETE** — the native gate (spikes ①–④) plus the project-inference spike (snapshot) all PASS. What's next is Phase 1:

1. **Wire `tauri-specta`** into the existing app (D27) — exact-pin the RC trio, migrate `get_watcher_status` + the `Result<_, String>` commands → `AppError`, add the binding-freshness gate. The first Phase-1-enabling plumbing; unrelated to the Mac, so it's CI-friendly.
2. **Phase 1 capture impl** — fold the four proven spikes into the real `capture` trait behind a `Fake` (hard rule 5), `#[cfg(target_os="macos")]`-gated. Includes the terminal pid-*selection* step (front-tab shell pid via `proc_listchildpids`+tty/recency, or iTerm2 `path`) that Spike ④ left as mechanism.
3. **Data model for projects** (D30) — `projects` table keyed on the **git remote** with folder/title/URL aliases; persist `unassigned` + the abstain *kind* (`no-signal` vs `ambiguous`) so Phase 2 can correlate ambiguous tooling.
4. **Re-measure project-inference recall** once capture persists multi-day data (the spike measured precision + the abstain threshold on a snapshot, not longitudinal recall).

**Residual manual checks (small, optional, owner = you):**
- Spike ②: in-place `AXTitleChanged` (navigate a browser tab while it stays frontmost; confirm `TITLE` lines). Machinery proven.
- Spike ③: **Safari** URL + private-detection (R16/R18 — Safari wasn't running; safe-default = never emit an unprovable-non-private Safari URL), **Arc** (R17 window/space model), and the **`-1743` deny** fallback (toggle a browser off in Automation settings to see it).

## 7. Re-run the spikes (reference)

**Spike ② (`ax-observer`):**
```sh
cargo build --manifest-path spikes/ax-observer/Cargo.toml
codesign --force --sign - spikes/ax-observer/target/debug/ax-observer   # R14: re-sign so the AX grant sticks
./spikes/ax-observer/target/debug/ax-observer                            # switch apps + change titles; Ctrl-C to stop
```
Idle wakeups: `sudo powermetrics --samplers tasks -i 1000 -n 5 | grep -i ax-observer` while not touching the machine.

**Spike ③ (`browser-url`):**
```sh
cargo build --manifest-path spikes/browser-url/Cargo.toml
./spikes/browser-url/target/debug/browser-url   # switch browser tabs / open incognito|private windows; Ctrl-C to stop
```
First query per browser prompts for Automation — click Allow (per-browser). Reads only, never stores.

**Spike ④ (`proc-cwd`):**
```sh
cargo build --manifest-path spikes/proc-cwd/Cargo.toml
pgrep -x zsh | sort -n | head -6 | xargs ./spikes/proc-cwd/target/debug/proc-cwd   # zsh doesn't word-split $vars — use xargs
```
No TCC grant needed. Compare against `lsof -a -p <pid> -d cwd`.

**Project-inference spike (`project-inference`):**
```sh
cargo build --manifest-path spikes/project-inference/Cargo.toml
./spikes/project-inference/target/debug/project-inference   # runs the heuristic on the real corpus
```
Pure std + shelled `git`. The corpus is editable in `main.rs` to re-test with other signals.

**Spike #1 (`ax-titles`):** same pattern; see `spikes/ax-titles/README.md`. `tccutil reset Accessibility` for a clean grant slate (both READMEs).

## 8. Blockers / open logistics

- **No blockers** for spikes ③/④ — all runnable on the Mac.
- **This session's work (Spikes ② + ③ + doc updates) is uncommitted on `main`'s working tree.** Per the dev workflow (branch from `main`, PR + review before merge), it should land like Spike #1 did (PR #6) — e.g. a `spike/native-capture-events` branch (or one branch per spike), commit, open PR(s). _Not done automatically — confirm before pushing._
- **Leftover test windows:** a Chrome **incognito** window was opened during Spike ③ testing — close it when convenient.
- **Design track** still needs `/design-login` (**not available here**) — or design in-repo against `context/design-system.md`. The audit's design blocker stands: Comms yellow `#F2BC0C` fails WCAG non-text contrast (1.47:1) with color as the only context channel — fix the palette / add a non-color cue before locking (R77).
- **Deletable stale remote branches:** `origin/claude/frontend-design-TwfKi`, `origin/feat/tier1-oss-hygiene` (merged). Still in place.

## 9. Gotchas / environment (important)

- **Disk:** was critically full earlier this redesign; this session it was fine (~18 GB free). The `ax-observer` target adds a few hundred MB. Keep an eye on it; `cargo clean` in `spikes/*/target` reclaims space.
- **objc2 AXObserver specifics (validated in Spike ②):**
  - `AXObserver::create(pid, Some(callback), NonNull<*mut AXObserver>) -> AXError`; the result follows the **Create rule** → `CFRetained::from_raw`. Callback type is `AXObserverCallback = Option<unsafe extern "C-unwind" fn(NonNull<AXObserver>, NonNull<AXUIElement>, NonNull<CFString>, *mut c_void)>`.
  - `observer.add_notification(element, &CFString, refcon)` / `remove_notification(...)`; `observer.run_loop_source() -> CFRetained<CFRunLoopSource>` → `CFRunLoop::add_source(.., kCFRunLoopCommonModes)`.
  - Notification **names** are not re-exported either — build CFStrings from `"AXFocusedWindowChanged"` / `"AXTitleChanged"` / `"AXMainWindowChanged"` / `"AXFocusedUIElementChanged"`.
  - Activation observer: `objc2-foundation` features `NSNotification` + `NSOperation` + `NSString` + `block2`; `addObserverForName_object_queue_usingBlock(.., &block2::DynBlock<dyn Fn(NonNull<NSNotification>)>)`; **keep the returned token alive**.
  - The `refcon` pattern: heap-`Box` the callback context, pass `addr_of!(*ctx)` as `refcon`, reconstruct a **shared** `&Context` in the callback and mutate via `RefCell`/`Cell` (no `&mut` aliasing). Remove the run-loop source in `Drop` **before** the box frees.
  - Provisional pins all held: `objc2` 0.6.4, the `objc2-*` crates 0.3.2, `block2` 0.6.2.
- **R14 rebuild footgun** still applies — re-sign + re-grant after a rebuild that starts erroring `AXERR(CannotComplete/APIDisabled)`. See `spikes/ax-titles/README.md`.

## 10. Testing status

- 25 Rust + 28 TS tests pass; **CI green (Linux/macOS/Windows)** on the merged PRs.
- The native spikes (①②③) are **verified on the real Mac**, not in CI (hosted runners can't grant TCC or run a GUI). Plan unchanged: Phase-1 native code lives behind the `capture` trait with a `Fake` for CI; `#[cfg(target_os = "macos")]`-gate the objc2/osascript impl so non-mac legs stay green (R80).
