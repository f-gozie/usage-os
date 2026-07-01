# Handoff — 2026-07-02-01 · SMAppService login item built + reviewed, PR #38 open

## 1. Current state
- **[PR #38](https://github.com/f-gozie/usage-os/pull/38) open** on `feat/smappservice-login-item`
  (3 commits). All gates green. **[PR #37](https://github.com/f-gozie/usage-os/pull/37)** (About
  version fix) is still open too — both ship in v0.1.2.
- Source: owner feedback minutes after the v0.1.1 update — the background-items notification
  attributed the login item to the signing certificate's name (personal Developer ID), not the
  app. **Docs rule held throughout: every artifact describes this generically; no personal names
  anywhere in the repo (verified `git grep` clean).**

## 2. What landed (D69, as-built)
- Agent plist bundled at `Contents/Library/LaunchAgents/com.usageos.app.agent.plist`
  (`bundle.macOS.files`), registered via `SMAppService.agent` → **Login Items shows "UsageOS"
  with its icon**. `tauri-plugin-autostart` removed (lock −6 packages). Commands/bindings/frontend
  untouched. Legacy-plist migration on startup (register first, delete after).
- **Trampoline agent launch** — the review's big catch (Codex + Lane B, cross-model): launchd
  kills the process it owns on `unregister`, so toggle-off inside a login-launched session would
  have killed the app. The job now respawns the app detached (`setsid`; plist env
  `USAGEOS_AGENT_LAUNCH`, handoff `USAGEOS_DETACHED`) and exits — launchd never owns the session.
- **Instance flock** (`Application Support/com.usageos.app/instance.lock`) replaces the racy
  process-list guard; `--hidden` must win it or exit; the updater relaunch retries ~5s
  (`USAGEOS_SHOW_AFTER_RESTART`) because it overlaps its exiting predecessor.

## 3. Verified on-device (Developer-ID re-signed bundle — SMAppService refuses ad-hoc)
Login Items attribution ✓ · kickstart/RunAtLoad → trampoline exits 0, no duplicates ✓ ·
agent-launched session runs detached in menu-bar mode, Reopen works ✓ · **toggle-off from that
session: app survives, agent unregisters** ✓ · no bare `~/Library/LaunchAgents` plist ever ✓.

## 4. FIRST for the next session / owner
1. Merge **PR #37** and **PR #38** (REST merge: `gh api -X PUT repos/f-gozie/usage-os/pulls/<n>/merge -f merge_method=merge`).
2. Cut **v0.1.2** (release flow notes in handoff 2026-07-01-03 §4 — including the DMG staple
   check and the landing URL bump). This release also tests D69's watched expectation: the agent
   registration should survive the 0.1.1→0.1.2 in-place update.
3. Owner: real log-out/in with the toggle on — expect the notification/Login Items to say
   UsageOS; the session that starts is the trampoline's detached child.

## 5. Gotchas (this session)
- **SMAppService needs a real signing identity** — ad-hoc bundles fail to register (that's the
  attribution working as designed). Local verification: re-sign the `tauri build` bundle with the
  release identity first.
- **`launchctl print gui/$UID/com.usageos.app.agent`** is the ground truth for registration;
  `kickstart` simulates the RunAtLoad race on demand.
- The release webview's AX tree is unreliable (`entire contents` often empty) — real clicks; and
  a mis-aimed click can open modals (Escape doesn't close the category editor — click CANCEL).
- A session restart killed the first review panel mid-flight — relaunched lanes are cheap;
  Codex writes its JSON only at the end (no partial output).
