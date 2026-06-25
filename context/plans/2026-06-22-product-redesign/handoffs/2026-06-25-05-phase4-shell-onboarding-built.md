# Handoff — 2026-06-25-05 · Phase 4 shell & onboarding built (code-complete, on-device pending)

**Where we are now:** Phase 4 is **code-complete on `phase4/shell-onboarding`** (commit `4e2a084` + docs) and **not yet pushed/PR'd** — it's gated on **on-device verification** (native tray/AX/AppleEvent/CSP paths, per D32/D33). All CI-checkable gates are green: **125 Rust + 32 TS tests, `clippy -D warnings`, `cargo fmt`, `tsc`, `vitest`, bindings fresh**. (Earlier this session: the recap branch was reviewed + squash-merged as `c58767e` — see handoff 04.)

## What landed (Phase 4 — D55; impl-plan `2026-06-25-phase4-shell-onboarding`)
Decisions were owner-set in plan mode: **tray + hide-on-close** (Dock retained), **one PR**, **share-donut glance** (not the time-dial — crowds at small size; owner reviewed the busy-15h stress test in `design/menubar.html`).
1. **Permission seam** (`src-tauri/src/permissions/`, cfg-gated + non-macOS stub): `get_permissions`/`request_accessibility`/`request_automation`/`open_settings_pane`. Accessibility = `AXIsProcessTrusted`; Automation = `AEDeterminePermissionToAutomateTarget` (reads the TCC grant **without** sending an Apple Event; `ask_user=true` prompts without launching). Capture's AX prompt factored into the seam. Pure `aggregate_automation` unit-tested.
2. **Onboarding** (`src/components/onboarding/`): the `design/onboarding.html` flow, skippable → degraded Ready; `usePermissions` re-reads on window focus; first-run gate in `App.tsx` (`onboarding_completed` setting); re-grantable from **Settings → Permissions**. RTL tests.
3. **Tray + glance** (`lib.rs` `setup_tray`; `src/components/glance/Glance.tsx`): core tray; left-click toggles the `#/glance` borderless webview (donut + Active/Top/Focus + top-3 from `getDay`); right-click Open/Quit; main-window close → hide (tracking continues); `show_main_window`/`quit_app` commands; `glance` capability.
4. **CSP**: `tauri.conf.json` `csp` null → strict `default-src 'self'; … object-src 'none'; frame-src 'none'`.

## Next steps
1. **You (on Favour's Mac) — run the on-device checklist** before anything ships (full list in the impl-plan):
   - Onboarding launches fresh; Accessibility "Grant" opens the pane + pill flips on return; Automation "Grant" prompts the running browser; Skip → degraded Ready; "Open my day" completes + never re-shows.
   - Tray icon appears; left-click → glance with correct today numbers; "Open UsageOS" shows the window; **closing the window keeps tracking** (events still recorded while hidden); **tray Quit** exits.
   - Strict CSP doesn't white-screen (app + glance); fonts/icons load. Dark-mode parity across all views + onboarding + glance; idle CPU ~0%.
   - Watch: tray-popover **positioning** across displays (the one piece most likely to need tuning — `position_glance` in `lib.rs`); the `AEDeterminePermissionToAutomateTarget` FFI behaviour (status accuracy + the prompt).
2. **Then:** `/usageos-review` the branch (write `reviews/2026-06-25-phase4-shell-onboarding.md`), fix findings, **push + open the PR**.
3. After merge: remaining Phase 4 = **day-start offset (D14)**; then Phase 5 (launch: notarized DMG + auto-update + Homebrew) and Phase 6 (memory diagnosis, cargo-deny, R57 WAL writer).

## Gotchas
- The branch is **local only** (not pushed) — `git push -u origin phase4/shell-onboarding` when ready.
- Tray `rect` is `tauri::Rect` (crate-root re-export), **not** `tauri::tray::Rect` (private → E0603). Noted in case of future edits.
- Stale local branches `phase6/audit-fixes` (merged #18) + `spike/foundation-models` are safe to delete.
- The untracked `design/logo/` + `reviews/*.claude-findings.json` are not part of this work — leave them.
