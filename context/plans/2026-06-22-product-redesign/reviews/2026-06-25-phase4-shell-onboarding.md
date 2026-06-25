# Review — Phase 4 shell & onboarding

**Date:** 2026-06-25 · **Scope:** branch (`phase4/shell-onboarding` vs `main`) · **Files:** 24 (≈12 code, rest docs/design)
**Plan:** [plan.md](../plan.md) · **Impl-plan:** [2026-06-25-phase4-shell-onboarding.md](../impl-plans/2026-06-25-phase4-shell-onboarding.md)
**Codex:** ran (gpt-5.5, xhigh) — 3 findings folded in

## Merge gates
| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy -D warnings | ✅ |
| cargo test | ✅ 125 passed, 1 ignored |
| tsc --noEmit | ✅ |
| vitest | ✅ |
| bindings fresh | ✅ |

## Findings
**Verification:** 8 verified · 1 downgraded (Codex Critical→Warning) · 0 cross-model-confirmed (lanes found disjoint issues — wider coverage)

### Critical (must fix before merge)
_None._

### Warnings (should fix)
- `src-tauri/src/capture/macos/mod.rs:411` — **[codex]** Capture **auto-prompts** for Accessibility at startup (`prompt_accessibility_trust()`) whenever the grant is missing — independent of the new onboarding flow. This conflicts with Phase 4's skippable onboarding: a user who clicks "Maybe later" can still get the macOS system prompt at first launch. **Pre-existing** behavior (the diff only refactored the inline AX call into the `permissions` seam), but Phase 4 is where it should be reconciled (the fn comment even says "full priming is Phase 4, see D21"). Note macOS only shows the system dialog **once** per install (afterwards the call returns false silently), so the conflict bites on the very first launch. _Fix:_ make capture startup **query + log degraded mode only** (drop the `prompt_accessibility_trust()` call); leave prompting exclusively to `request_accessibility` from onboarding/Settings. **Touches first-run UX — owner's call.**
- `src-tauri/src/lib.rs:637-647` + `:691` — **[codex]** Hide-on-close has **no tray-failure fallback**. The `main` close handler unconditionally `prevent_close()` + `hide()`, but `setup_tray` is non-fatal (logs on `Err`). If the tray fails to build, the app runs with its only window hidden and no in-app Open/Quit path (Cmd-Q still exits, so not a full lock-up). _Fix:_ gate prevent-close on tray-setup success (shared `AtomicBool`), or add a Dock/Reopen handler that calls `show_main`.
- `src/components/glance/Glance.tsx:154,161` — **[codex]** Glance footer buttons use `rounded-[7px]`, deviating from the Bauhaus **hard-edge** rule ([design-system.md:65](../../../design-system.md): "Square by default; the app window frame is the only meaningful radius (5px)"). Onboarding's own buttons are square (`border-2 border-edge`). This is a third ad-hoc radius alongside the glance card's `rounded-[16px]`. _Fix:_ square the buttons to match the system, **or** document a sanctioned "native popover" exception/token. The owner verified the glance to taste (D56) — so this may be intentional; **owner's call.**
- `src-tauri/src/lib.rs:508-547` — **[lane B]** Tray-click ↔ focus-loss **dismiss race**. `toggle_glance` toggles on `is_visible()` while the panel's `Focused(false)` handler also hides it; on macOS a tray click could deliver the panel's focus-loss first → the toggle then re-shows instead of dismissing. **On-device-dependent** and **already documented** in-code ([lib.rs:541-542](../../../../src-tauri/src/lib.rs:541)) with the AppKit outside-click-monitor fallback. The panel is non-activating (never `set_focus`), so it may not receive focus events at all. _Action:_ verify the specific "click tray while open → dismisses" case on-device; if it re-opens, add a last-hide debounce or the outside-click monitor.
- `src/hooks/usePermissions.ts:26-47` + `src/components/glance/Glance.tsx:34-51` — **[lane C]** The `onFocusChanged` refresh-on-focus idiom (dynamic `import`, `active`/`unlisten` guard, `.catch`) is duplicated **verbatim** across both files. _Fix:_ extract a tiny `useWindowFocus(onFocus)` hook; both call sites collapse to one line.

### Info
- `src/views/SettingsView.tsx:225,235` — **[lane B]** The two `request_*().then(refetch)` calls omit `.catch(() => undefined)` that every other call site in the diff has. Infallible today (commands always `Ok(())`), so latent only — append `.catch` for parity.
- `src-tauri/src/lib.rs:522,531-534` — **[lane B]** Stale comments reference a `set_effects` "native popover material" that was dropped (the visible chrome is the CSS card in `Glance.tsx:79`). Delete the stale wording; reconcile the D56 note.
- `src/components/glance/Glance.tsx:248-252` — **[lane C]** `formatDuration(activeSecs)` computed twice in one node; hoist once.
- `src/views/SettingsView.tsx` `GrantedTag` vs `Onboarding.tsx:256-259` — **[lane C]** The "Granted ✓" pill class string is copy-pasted; optional shared `GrantedPill`.
- `src/components/onboarding/Onboarding.tsx:18` — **[lane C]** `STEPS` rebuilt each render; lift to a module constant. (Step machine itself is appropriately sized — not over-built.)
- `src/components/glance/Glance.tsx:102` — **[lane C]** "No activity tracked today yet." → optional warmer "Nothing tracked yet today." Microcopy otherwise reads human, no AI-tells.

## Auto-fixes applied
_None._ Every Warning is a behavior/design/refactor change (none are provably-safe-mechanical), and the headline one is a first-run-UX decision for the owner. Reported for manual/owner decision instead.

## Manual TODO
- [ ] **Decide** capture startup-prompt: drop `prompt_accessibility_trust()` at `mod.rs:411` so skip is truly promptless (recommended), or keep + document.
- [ ] Tray-failure fallback for hide-on-close (`lib.rs`).
- [ ] Glance button radius: square or document the popover exception (owner taste).
- [ ] On-device: verify tray-click-to-dismiss doesn't re-open the glance.
- [ ] Extract `useWindowFocus()` (de-dup `usePermissions` + `Glance`).
- [ ] Info nits (SettingsView `.catch` parity; stale `set_effects` comments; minor reuse/microcopy).

## Definition of Done
- [x] plan.md ticked for what landed (Phase 4 row updated)
- [x] decisions.md ADRs appended (D55 permission seam/CSP, D56 NSPanel glance)
- [x] impl-plan present ([2026-06-25-phase4-shell-onboarding.md](../impl-plans/2026-06-25-phase4-shell-onboarding.md)) · handoff to follow (verification + this review)
- [x] docs move with code — diff touches `context/plans/` + `decisions.md`, so the pre-push tripwire would **not** fire

## Plan compliance
Alignment: **good** — scope matches Phase 4 (menubar tray, glance NSPanel, first-run onboarding, permission seam, CSP). No scope creep. The diff is a net **isolation improvement** (AX-trust `unsafe` consolidated out of `capture/` into the single `permissions/` seam). The one architectural gap is the capture↔onboarding prompt reconciliation above.

---

## Resolution (applied 2026-06-25 — gates re-run green: 125 Rust + 32 TS, clippy/fmt/tsc/vitest, bindings unchanged)
All findings addressed except #3, which the owner kept as-is.

- ✅ **#1 capture prompt** — `capture::start` no longer force-prompts; `warn_if_capture_degraded` queries + logs only. Prompting lives solely in onboarding/Settings. Removed the now-dead `permissions::prompt_accessibility_trust` re-export. **→ D57.**
- ✅ **#2 tray-failure fallback** — `TRAY_READY` flag; main window keeps normal close-to-quit until the tray is up.
- ⏸️ **#3 glance button radius** — **kept rounded by owner decision** (the native-popover look). Not changed.
- ✅ **#4 dismiss race** — debounce: a tray click within ~200ms of the focus-loss auto-hide doesn't re-open the glance. _Still wants the on-device click-tray-to-dismiss confirmation._
- ✅ **#5 duplicated focus hook** — extracted `src/hooks/useWindowFocus.ts`; used by `usePermissions` + `Glance`.
- ✅ **Info** — shared `GrantedPill` (ui/, + story) replacing the duplicated pill; `.catch` parity in Settings; `formatDuration` hoisted; `STEPS` lifted to module const; stale `set_effects` comment removed; D56 corrected. The microcopy tweak was reverted (owner kept the glance copy as-is).

**Tests:** no new CI tests — the new Rust is native glue (tray gate, glance timing, degraded log) verified on-device per D32/D33, not in CI; the new TS is a thin Tauri-listener hook + a presentational component (story-covered) — consistent with the project's existing standard.

**Re-verify on-device after this:** "Maybe later" is now promptless at first launch (D57), and tray-click dismisses the glance without re-opening (#4).
