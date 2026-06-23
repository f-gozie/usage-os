# Handoff — start here for the next session

_Written 2026-06-22 at the end of the **Phase-0 spike session**. Read this, then `CLAUDE.md`, then `context/feasibility/2026-06-22-feasibility-audit.md`, then the redesign plan. Supersedes the prior (vision/planning) handoff._

## 1. Where we are (TL;DR)

Phase 0 of the redesign is **mostly done and the project is de-risked**. The single biggest unknown — *"do AX window titles work for Chromium/Electron apps without Screen Recording?"* — is **answered YES on the real Mac**. The feasibility verdict was **GO-WITH-CAVEATS**; the heaviest caveat is now gone, so it leans **GO**.

Three PRs **merged to `main`** this session (`origin/main` → `dcf9d8e`):
- **[#4]** feasibility audit + 5 standards docs + canonical-doc reconciliation
- **[#5]** hard-rule-3 cleanup (no prod panics) + missing CI gates
- **[#6]** the isolated AX-titles capture **Spike #1** (PASS, documented)

[#4]: https://github.com/f-gozie/usage-os/pull/4
[#5]: https://github.com/f-gozie/usage-os/pull/5
[#6]: https://github.com/f-gozie/usage-os/pull/6

## 2. The headline result — Spike #1 PASSED (R4 retired)

`spikes/ax-titles/` (isolated crate, now on `main`) proved AX returns **real, non-empty focused-window titles** for **every app class**, with **Accessibility only + Screen Recording OFF**:

| Class | Apps confirmed | Result |
|---|---|---|
| Chromium | Chrome, Brave | `REAL` (Chrome title carries the full page/tab title) |
| Electron | Cursor (VS Code fork), Claude, Notion, Figma, WhatsApp, Spotify | `REAL` |
| Native | Finder | `REAL` |
| Terminal | iTerm2 | `REAL` |

**Findings that shape the next work:**
1. **A run loop is REQUIRED** to track app switches. `NSWorkspace.frontmostApplication` is stale without one; the spike pumps `CFRunLoop::run_in_mode` as a stopgap. → confirms audit **R6**; Spike ② must build the real observer model.
2. **System-wide `AXFocusedApplication` returns `AXERR(CannotComplete)` from a plain CLI** — read the title by **frontmost pid** instead (`AXUIElement::new_application(pid)` → `AXFocusedWindow` → `AXTitle`).
3. **Titles already carry rich project/page signal** (Cursor → "nudge"; Chrome → GitHub PR / Gmail subject) → boosts project inference (D6) even before URLs.
4. **…but titles carry sensitive content** (a Gmail subject + email leaked in a Chrome title) → **D8 (exclusion list / per-app Private / incognito-never-recorded) is load-bearing, not optional.** Also strip stray bidi/control marks (WhatsApp had a `‎` LTR mark).

## 3. Decisions made this session (in `context/decisions.md`)

- **D26** — Embeddings run **in Rust** via `objc2-natural-language` (NLEmbedding); the Swift sidecar stays **Foundation-Models-only**. Refines D10/D16.
- **D27** — **Exact-pin the `tauri-specta` RC trio** (`=2.0.0-rc.x`); commands-only at launch (event-payload bug #211).
- **D28** — Feasibility verdict **GO-WITH-CAVEATS**; Phase 1 product code gated on the native spike (now: make-or-break ✅ passed). Flags the hard-rule-3 + recharts items.
- **Correction:** `recharts` is **used** by `src/components/ActivityChart.tsx` (the current dashboard) — *not* unused as the audit claimed. Remove only when the Bauhaus dial replaces that dashboard (Phase 1+).

## 4. What's on `main` now (read order for the next session)

- `context/feasibility/2026-06-22-feasibility-audit.md` — **the map**: exec verdict, risk register **R1–R83** (assumption → verdict → evidence → what the spike must prove → which decision), per-area read, prioritized spike plan **§4**, 66 citations.
- `context/standards/{capture-and-permissions,tauri-ipc,rust,testing-and-ci,foundation-models}.md` — conventions. **PROVISIONAL** (desk-grounded; spikes confirm native/version claims).
- `context/decisions.md` — D1–D28.
- `context/plans/2026-06-22-product-redesign.md` — Phase 0 progress; Spike #1 marked ✅.
- `spikes/ax-titles/README.md` — Spike #1 results table + findings + re-run protocol.

## 5. Code state (merged, working)

- **Hard rule 3 enforced:** no production `.expect()`/`.unwrap()`/`panic!`. `lib.rs` setup returns `Result`; `db::now_unix()` helper replaces the `SystemTime` expects; `#![cfg_attr(not(test), deny(clippy::unwrap_used, expect_used, panic))]` on both crate roots.
- **CI gates live** in `.github/workflows/ci.yml`: `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `npx tsc --noEmit` — on top of the existing 3-OS (`ubuntu`/`macos`/`windows`) test matrix. **All green on all three PRs.**
- **Binding-freshness gate deferred** until `tauri-specta` is actually wired (would gate on nothing today).
- Tests: ~25 Rust + 28 TS (the TS suite is pure-logic; **React Testing Library is net-new**).

## 6. NEXT SESSION — do these, in order, thoroughly

The remaining native capture spikes (audit §4), each on the real Apple-Silicon Mac, then project inference. **Be thorough; record each result in the spike README + flip the relevant R-items in the feasibility doc/plan.**

1. **Spike ② — event-driven capture model (R6, R8–R13).** The *real* architecture: `NSWorkspace` `didActivateApplicationNotification` + `AXObserver` (`kAXFocusedWindowChanged` / `kAXTitleChanged`) on a proper run loop; per-PID observer rebuilt on app switch; results marshaled to a `Send` channel without blocking Tokio; debounce chatty title changes (C3); measure idle wakeups (`powermetrics`). The spike's pump was a stopgap — this replaces it. This is what Phase 1's `capture` trait impl will use.
2. **Spike ③ — browser URL + incognito exclusion (R15–R21, D8).** `osascript` for Chrome/Brave/Arc (`URL of active tab of front window`) + Safari (`URL of front document`); **check `mode of front window` == "incognito" BEFORE reading the URL**; Safari private-window safe-default; `-1743` graceful per-browser fallback; needs `NSAppleEventsUsageDescription`. D8 is now proven load-bearing.
3. **Spike ④ — `proc_pidinfo` cwd read (R22).** Can a non-root, unsandboxed binary read another process's cwd on a live iTerm2/Terminal pid? If `EPERM`, the terminal-cwd branch of project inference dies — settle early.
4. **Project-inference spike (R23, R26, R27).** Measure precision/recall + false-positive rate on real titles/cwd/repos; abstain threshold. Boosted by the finding that titles already carry project signal.

After these, Phase 1 (capture → the dial) is unblocked.

## 7. Re-run Spike #1 (reference)

```sh
cargo build --manifest-path spikes/ax-titles/Cargo.toml
codesign --force --sign - spikes/ax-titles/target/debug/ax-titles   # R14: re-sign after rebuild so the AX grant sticks
./spikes/ax-titles/target/debug/ax-titles                            # switch apps; Ctrl-C to stop
```
README has the full test protocol + `tccutil reset Accessibility`.

## 8. Blockers / open logistics

- **No blockers** for the next spikes — all runnable on the Mac.
- **Design track** still needs `/design-login` (**not available in this environment**) — or fall back to designing in-repo against `context/design-system.md`. Audit found a real **design blocker:** Comms yellow `#F2BC0C` fails WCAG non-text contrast (1.47:1) and color is the only context channel — fix the palette / add a non-color cue before locking (R77).
- **Deletable stale remote branches:** `origin/claude/frontend-design-TwfKi`, `origin/feat/tier1-oss-hygiene` (merged). Left in place this session.

## 9. Gotchas / environment (important)

- **Disk was critically full** this session (231 MB free / 100%) — resolved via `cargo clean` on `src-tauri/target` + removing finished agent worktrees (~9 GB reclaimed; ~20 GB free now). The spike agent hit a transient "No space left." **Keep an eye on disk.**
- **Process lesson (orchestration):** a large background **workflow (50 agents) got interrupted ~7 min in** (environmental — session compaction / sleep / app backgrounding) and wedged at a barrier. Recovery that worked: salvage completed results from the run journal, then re-run the remainder as a **smaller, hardened** run (bounded web calls per agent) with **worktree-isolated** subagents. **Prefer smaller, hardened, isolated runs over one mega-workflow.**
- **objc2 family quirks (validated in the spike):** AX attribute-name constants are **not** re-exported — build `CFString`s from `"AXFocusedWindow"`/`"AXTitle"`. `NSWorkspace`/`NSRunningApplication` accessors are **safe** (not `unsafe`) in 0.3.2. To confirm exact API: grep `~/.cargo/registry/src/index.crates.io-*/objc2-*-0.3.2/src/generated/`. Provisional pins **held** (`objc2` 0.6.4, the `objc2-*` crates 0.3.2).
- The standards docs are **provisional desk research** — the spikes are what turn them into fact (Spike #1 just did that for capture-and-permissions.md's title path).

## 10. Testing status

- 25 Rust + 28 TS tests pass; **CI green (Linux/macOS/Windows)** on all three merged PRs, including the new clippy/fmt/tsc gates.
- The native AX path is **manually verified** (Spike #1 on the real Mac), **not in CI** (hosted runners can't grant TCC or run a GUI). Plan: future native code lives behind the `capture` trait with a `Fake` for CI; `#[cfg(target_os = "macos")]`-gate the objc2 impl so non-mac legs stay green.
