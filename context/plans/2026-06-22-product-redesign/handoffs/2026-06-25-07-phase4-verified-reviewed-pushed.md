# Handoff — 2026-06-25-07 · Phase 4 verified on-device + `/usageos-review` done + branch pushed; PR is the next click

**Where we are now:** Phase 4 (shell & onboarding) is **done, reviewed, and pushed**. `phase4/shell-onboarding` is now **on `origin`** (was local-only) — latest commit **`c6507fd`**. All gates green throughout: **125 Rust + 32 TS, clippy -D warnings, fmt, tsc, vitest, bindings fresh.** Onboarding is verified on-device (both scenarios), the branch passed the project's own `/usageos-review`, and every finding is resolved except one the owner deliberately kept. **The only thing left before merge is opening the PR.**

## What landed since handoff 06

### 1. Onboarding first-run verification (the open item from 06) — PASSED
Resolved the dev-build TCC snag first: the `tauri dev` binary is **ad-hoc-signed** (`usage_os-<hash>`), so `tccutil reset … com.favour.usage-os` **and** `tccutil reset … usage_os-<hash>` both fail `-10814` — `tccutil` only matches real bundle ids. The reliable revoke is the **System Settings UI** (Privacy & Security → Accessibility / Automation → remove the `usage-os` row). In practice both grants were already absent, so the scenarios ran clean. Owner drove the clicks; I verified behavior from the DB + dev log (computer-use can't target a bare `target/debug` binary — `request_access` won't resolve it and `screenshotFiltering: native` hides its window, so the owner-drives + I-verify-from-data split is the way for dev builds):
- **Scenario A (skip → degraded):** `onboarding_completed=true`; dev log `[Capture] Accessibility not granted — capture is degraded`; activity rows had **no titles / no URLs**. ✓
- **Scenario B (grant → redirect → pill flip):** owner saw the System-Settings redirect + the `universalAccessAuthWarn` prompt + the pill flip to "Granted ✓". ✓
- **Tray Quit exits** ✓ · **Hide-on-close keeps tracking** ✓ (fresh Claude/Spotify rows recorded with the window closed, process alive) · **CSP renders, no white-screen** ✓.

### 2. `/usageos-review` on the branch — gates green, Codex earned its keep
Report: **`reviews/2026-06-25-phase4-shell-onboarding.md`** (paired to the impl-plan). 0 Criticals, all 6 gates green. Lane A found the 8 hard rules clean (the diff even *improves* isolation — AX-trust `unsafe` consolidated out of `capture/` into the `permissions/` seam). The **Codex cross-model lane (gpt-5.5, xhigh)** caught three things the Claude lanes missed; Claude lanes caught two more.

### 3. All findings fixed (commit `c6507fd`) except #3 (owner kept it)
- **#1 → D57:** capture no longer **force-prompts** for Accessibility at startup (was undermining the skippable onboarding). `capture::start` now calls `warn_if_capture_degraded` (query + log only); prompting lives solely in onboarding/Settings via `request_accessibility`. Removed the now-dead `permissions::prompt_accessibility_trust` re-export.
- **#2:** tray-failure fallback — a `TRAY_READY` flag keeps the main window's normal close-to-quit until the tray is built, so a tray-setup failure can't strand it hidden.
- **#3:** glance footer button radius (`rounded-[7px]` vs Bauhaus hard-edge) — **kept as-is by owner** (the native-popover look).
- **#4:** glance tray-click dismiss-race — 200ms debounce so a click that lands within the focus-loss auto-hide doesn't re-open it.
- **#5 + info:** shared `useWindowFocus()` hook (de-dups `usePermissions` + `Glance`); shared `GrantedPill` (ui/, +story) replacing the duplicated pill; `.catch` parity in Settings; `formatDuration` hoist; `STEPS` module const; stale `set_effects` comment removed; **D56 corrected** (the glance is a solid themed CSS card, not `set_effects` frost — that was dropped on-device).

## NEXT — open the PR, then merge
1. **Open the PR** (branch is pushed): `https://github.com/f-gozie/usage-os/pull/new/phase4/shell-onboarding`, base `main`. Use `/ship` for the body, or summarize: Phase 4 shell & onboarding (tray + glance NSPanel, first-run onboarding, permission seam, strict CSP — D55/D56/D57), verified on-device, reviewed.
2. **Two behaviors changed by the review fixes want a quick on-device re-check** (low-risk, code is straightforward):
   - **Promptless skip (D57):** first launch with Accessibility ungranted should show **no** macOS prompt. To test you must revoke first (Accessibility is currently granted from Scenario B) — use the System-Settings UI removal above, since the dev-binary `tccutil` route doesn't work. Or test a real `npm run tauri build` bundle (clean bundle id).
   - **Tray-dismiss (#4):** click the menubar icon to open the glance, click again → it should dismiss, not re-open.
3. Merge the PR.

## After Phase 4 merges (from `plan.md`)
- **Phase 5 — launch:** notarized DMG + auto-update + Homebrew cask, README rewrite. (Needs the Apple Developer cert.)
- **Phase 6:** diagnose the 2.13 GB memory observation; `cargo-deny`; R57 (WAL + dedicated writer thread).
- **Day-start offset (D14)** is the last deferred Phase-4 item.

## Gotchas / state
- **Dev app is running** (the owner's `npm run tauri dev`, pid changes across restarts; recompiled clean after the fixes). **Accessibility is currently GRANTED** to the dev binary (from Scenario B), so the startup degraded-log no longer prints — that's correct, not a regression.
- **Dev-build TCC identity** (worth remembering): the `tauri dev` binary is ad-hoc-signed `usage_os-<hash>`; `tccutil` can't reset it (`-10814`). Revoke via the System Settings UI, or test a bundled `.app` (real `com.favour.usage-os` id resets cleanly).
- The capture-prompt removal means **upgraded users who never granted now stay silently degraded** until they grant from Settings → Permissions (acceptable — Settings has the grant UI). New users are prompted only when they click "Grant access".
- Branch is **pushed**; the two pre-existing untracked items (`reviews/2026-06-25-full-codebase-audit.claude-findings.json`, `design/logo/`) were **left untracked** — not part of this work.
