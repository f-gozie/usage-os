# Handoff — 2026-06-25-06 · Phase 4 glance re-architected (debate D56) + on-device-refined; onboarding verification next

**Where we are now:** Phase 4 (shell & onboarding) is **code-complete on `phase4/shell-onboarding`** (6 commits ahead of `main`, **local only — not pushed**). All gates green throughout: **125 Rust + 32 TS, clippy -D warnings, fmt, tsc, vitest, bindings fresh.** The **menubar tray + glance popover is verified on-device and refined to the owner's taste.** What remains before the PR is **onboarding first-run verification** (needs permissions + the onboarding flag reset — see below), plus a couple of quick checks, then `/usageos-review` → push → PR.

## What landed since handoff 05
Handoff 05 covered the initial Phase 4 build (permission seam, onboarding, tray, CSP — D55). Then on-device the glance popover failed (no fullscreen float, transparency halo, donut overflow). A **`/debate` (Codex vs Opus, 2 rounds)** drove a clean redo — **D56**:
- **`src/glance_panel.rs`** — the glance is now a **non-activating `NSPanel`** (in-repo objc2 `object_setClass` reclass to a `define_class!` subclass; `NSPopUpMenuWindowLevel`; `CanJoinAllSpaces | CanJoinAllApplications | FullScreenAuxiliary | Transient | IgnoresCycle`; no `set_focus()`). Floats over full-screen Spaces. No new dep (chose in-repo objc2 over `tauri-nspanel`).
- Then three on-device-feedback fixes (all committed, verified to taste): **solid themed `bg-bg` card** (dropped the system-frost `set_effects` — it ignored the app's paper/warm/black theme and washed out labels); **positioning centres directly under the tray rect** (removed a `monitor_from_point` clamp that threw the popover onto the wrong display); **primary buttons + "Granted ✓" pills use `bg-bar-bg`/`text-bar-fg`** (the always-inverted pair — `bg-edge`/`text-bg` was dark-on-dark on the dark themes).
- Branch commits: `4e2a084` (build) · `c6c7a9a` (docs/D55) · `0dd5687` (first refine) · `ffd0d46` (D56 NSPanel) · `fed70cd` (theme+positioning) · `ee91ee2` (button legibility).

**Verified on-device:** tray icon, glance floats over full-screen, themed correctly (warm theme), positions under the tray on the right display, buttons legible. **Owner: "looks perfect, this is what I wanted."**

## NEXT — onboarding first-run verification (two scenarios the owner wants)
Couldn't test last session because Accessibility/Automation were already granted + `onboarding_completed=true`. **Reset both, then restart and run two scenarios:**

**Scenario A — skip everything:** walk onboarding clicking "Maybe later"/"Skip" → land on the degraded Ready → "Open my day". Observe how capture behaves **app-only / degraded** (no titles, no URLs) — a deliberate contrast with the fully-granted tracking we've seen.

**Scenario B — accept the prompts:** walk onboarding clicking "Grant access" on each step → observe the redirect (System Settings pane opens; toggle; return → the pill flips to "Granted ✓" on window-focus) → "Open my day".

### Reset commands (quit the app via the tray first)
```bash
# 1. Re-show onboarding (DB confirmed at this path; theme is currently "warm"):
sqlite3 "$HOME/Library/Application Support/com.favour.usage-os/usage.db" \
  "DELETE FROM settings WHERE key='onboarding_completed';"

# 2. Revoke the grants so the prompts are valid again:
tccutil reset Accessibility com.favour.usage-os
tccutil reset AppleEvents   com.favour.usage-os
```
**⚠️ Dev-build TCC caveat (the likely snag):** under `tauri dev` the running process's TCC identity is the **ad-hoc-signed dev binary**, not the bundle id `com.favour.usage-os` — so the bundle-id reset above may not clear the dev grant and the prompts won't reappear. If so, either (a) reset broadly: `tccutil reset Accessibility` / `tccutil reset AppleEvents` (no bundle id → all apps), or (b) test a real bundle via `npm run tauri build` and grant/reset the bundled `.app` (bundle id applies cleanly). Worth confirming which identity the dev app registers under before assuming the code is wrong.

After resetting: relaunch (native window changes need a fresh build anyway), then run A and B.

## Then — finish the checklist + ship
- **Hide-on-close keeps tracking:** close the main window → confirm new events still record while hidden → **tray "Quit" actually exits** (Cmd-Q too).
- **Strict CSP:** main window renders, no white-screen (it's `default-src 'self' …` in `tauri.conf.json`).
- **`/usageos-review`** the branch (write `reviews/2026-06-25-phase4-shell-onboarding.md`); fix findings.
- **Push** `phase4/shell-onboarding` + open the PR. (Reconcile plan.md/handoff at PR time — DoD.)

## After Phase 4 merges (from `plan.md`)
Phase 5 — launch (notarized DMG + auto-update + Homebrew, README rewrite). Phase 6 — diagnose the 2.13 GB memory observation, `cargo-deny`, R57 WAL writer. Day-start offset (D14) is the last deferred Phase-4 item.

## Gotchas
- **Native changes need an app restart** (NSPanel reclass / window flags are set at creation — hot-reload won't apply them).
- Glance dismissal is hide-on-`Focused(false)`; if a non-activating panel resolves focus oddly on-device, the fallback is an AppKit outside-click monitor (noted in `toggle_glance`).
- If the rounded popover's **shadow** looks square or corners aren't crisp, the lockdown is a native `contentView` layer `cornerRadius`/`masksToBounds` (would add `objc2-quartz-core`) — only if needed.
- Current theme in the DB is `warm`; the donut center text sizes off string length (25→20→16px) so 2-digit hours don't overflow.
- Branch is **local** — `git push -u origin phase4/shell-onboarding` when ready. Decisions D55 + D56 are committed on the branch; impl-plan `2026-06-25-phase4-shell-onboarding.md` has the full as-built.
