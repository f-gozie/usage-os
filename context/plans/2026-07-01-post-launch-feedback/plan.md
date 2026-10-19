# Post-launch feedback

The rolling home for what real users surface now that v0.1.0 is public. Each item traces back
to a concrete report (tester message, GitHub issue, dogfood note) — this plan collects them so
feedback-driven work has the plan → impl-plan → review → handoff paper trail without inventing
a new plan folder per nit.

**Scope:** app-behavior polish driven by user feedback. Bigger themes that grow a roadmap of
their own (e.g. categorization gaps → `2026-06-27-categorization-v2/`) get spun out, same as
Phase 5 → branding-launch.

## Items

### 1. Menu-bar agent: Dock icon follows the window + start at login (D68)
_Source: first tester feedback (2026-07-01 WhatsApp) — "wish the app didn't have to be open,
just the menu bar" / "I've closed it but it's still showing on my Dock"._

- [x] Dynamic activation policy — Regular while the dashboard is open, Accessory when closed
      (close → no Dock icon; tray/Reopen → icon returns)
- [x] `RunEvent::Reopen` → show the window (Finder/Spotlight re-open while Dock-less)
- [x] Start at login: `tauri-plugin-autostart` (LaunchAgent, `--hidden`), opt-in default OFF
- [x] `--hidden` launch starts in menu-bar mode (window `visible: false` + setup branch)
- [x] Settings → new "Background" group with the toggle
- [x] Onboarding "Background" step (recommended, skippable) between Automation and Updates
- [x] Tests: onboarding walk + enable path, BackgroundSettings toggle, SettingsView mock
- [x] On-device verification in dev (Dock cycle, glance reopen, LaunchAgent plist, `--hidden` launch)
- [x] `/usageos-review` ([review](reviews/2026-07-01-menubar-agent.md) — 5 warnings found + fixed,
      incl. update-restart-relaunches-hidden and toggle-revert-on-failure) + PR
- [x] Bundled-build smoke test (merged as [PR #36](https://github.com/f-gozie/usage-os/pull/36)):
      LS launch shows the window · close → no Dock icon · `open` again fires Reopen → window back
      focused · toggle writes/removes the LaunchAgent plist (bundle path + `--hidden`) · `--hidden`
      launch is menu-bar-only
- [x] **Shipped in [v0.1.1](https://github.com/f-gozie/usage-os/releases/tag/v0.1.1)** — DMG
      (notarized + stapled) + updater artifact + latest.json live; landing + Homebrew cask bumped
- [x] Owner check: in-app update 0.1.0 → 0.1.1 via Settings → Check now — **worked first try**
      (found, downloaded, verified, installed, relaunched)
- [ ] Owner check: a real log-out/in with the toggle on (incl. any Dock-icon flash at login)

### 2. About modal showed a stale version after the first real update
_Source: owner, right after the 0.1.0 → 0.1.1 in-app update (which otherwise worked first try)._

- [x] `AboutModal` had a hardcoded `VERSION` constant the release bump didn't cover — now reads
      `getVersion()` from the running binary (PR #37). Shipped in the re-cut v0.1.1.

### 3. Login Items should say UsageOS, not the signing certificate (D69)
_Source: owner, first run of Start at login on v0.1.1 — macOS attributed the background item to
the signing certificate's name instead of the app; reads unofficial._

- [x] Register via `SMAppService.agent` with the agent plist bundled inside the app
      (`bundle.macOS.files` → `Contents/Library/LaunchAgents/`)
- [x] New `login_item` module (cfg-isolated `objc2-service-management` surface); the
      `get/set_launch_at_login` commands and the whole frontend unchanged
- [x] `tauri-plugin-autostart` removed
- [x] Startup migration removes the legacy bare `~/Library/LaunchAgents` plist and re-registers
- [x] Trampoline agent launch: launchd owns the process it spawns, and unregistering kills it —
      the job respawns the app detached and exits, so toggle-off can never kill a login-launched
      session (cross-model review finding; verified on-device: session survives toggle-off)
- [x] Instance flock replaces the process-list guard (atomic; `--hidden` must win it or exit;
      the updater relaunch retries briefly — verified via `launchctl kickstart`)
- [x] Migration registers the bundled agent before deleting the legacy plist
- [x] Bundled-build verification on a Developer-ID-signed build: toggle registers/unregisters,
      **Login Items shows "UsageOS" with its icon**, agent-launched session survives toggle-off,
      no bare plist ever appears. (SMAppService refuses ad-hoc-signed bundles — sign before
      verifying.)
- [x] `/usageos-review` ([review](reviews/2026-07-01-smappservice-login-item.md) — the Codex lane
      caught the unregister-kills-the-app regression pre-merge) + PR
- [x] **Shipped in the re-cut [v0.1.1](https://github.com/f-gozie/usage-os/releases/tag/v0.1.1)**
      (owner call: replace the release rather than add a third) — tag re-pointed to `c0aeb49`,
      all three assets rebuilt from merged main (PRs #37 + #38), DMG notarized + stapled, cask
      sha256 bumped. Installs from the *first* 0.1.1 build won't self-update (same version) —
      reinstall from the DMG.

## Backlog (unclaimed feedback)

_(add items here as reports come in)_

### 4. Capture permissions die silently across updates — detect, surface, guide the fix (D71)

Found by auditing the author's own 32-day database (2026-07-25/31 sessions): window titles ~0%
since the v0.1.1 install (2 Jul 00:53, to the hour — stale Accessibility grant, checkbox still
"on"); browser URLs dying in whole-day chunks whenever Chrome updates and relaunches (-1743
swallowed as "no URL"). The app reported "healthy" throughout: the error counter never fires for
a missing title, and a menu-bar app nobody opens has no banner surface.

- [x] Latch the real Automation denial in `run_osa` (stderr `-1743` classification + tests)
- [x] `evaluate_health` (pure, tested): regression ("was granted, now gone" → alert once) vs
      never-granted (degrade quietly — the user's onboarding choice)
- [x] Health monitor on the existing 10-min tray thread (first pass ~30s after launch);
      last-seen state + alert flag in `settings`
- [x] Tray affordance: degraded tooltip + "Fix recording permissions…" menu item deep-linking
      to the broken System Settings pane (Accessibility first)
- [x] One macOS notification per regression episode (`tauri-plugin-notification` `=2.3.3`,
      local only)
- [x] `WatcherStatus` + the views' degraded banner name the actual problem, with
      switch-it-off-and-on copy for the stale-checkbox trap
- [x] **Shipped in [v0.1.2](https://github.com/f-gozie/usage-os/releases/tag/v0.1.2)**
      (2026-09-13) — PR #41 merged, tag at `5accb12`, DMG notarized + stapled, cask bumped.
      The 0.1.2 draft had sat since 31 Jul with no DMG, so the landing's download button 404'd
      for six weeks; resolved by this release.
- [x] On-device verification, detection half (2026-09-13): installed 0.1.2 over the degraded
      0.1.1; migration 0008 re-sorted 32 days (Entertainment 8.9 h); the health pass ran within
      36 s and wrote `capture_ax_ok=0`, `capture_auto_ok=1`; the Day view banner names the
      Accessibility loss with the switch-off-and-on copy.
- [x] On-device verification, recovery half (2026-09-13): owner toggled Accessibility off and
      on. The running process did NOT pick the grant up (titles stayed empty, flag stayed 0);
      after quitting and relaunching, the health pass cleared `capture_ax_ok` to 1 within 33 s
      and Chrome titles returned immediately.
- [x] Banner copy now ends "then quit and reopen UsageOS" (both permission banners) — PR #43,
      **shipped in [v0.1.3](https://github.com/f-gozie/usage-os/releases/tag/v0.1.3)** (2026-09-13,
      tag at `6acd59f`, cask bumped, landing link bumped after publish).
- [ ] Follow-up: have the health pass pick up a live TCC change without a relaunch (re-evaluate
      `AXIsProcessTrusted` / retry the Automation probe when the window regains focus?).
- [x] Owner check after the *next* app update (2026-09-13, in-app updater 0.1.2 → 0.1.3):
      Accessibility and Automation both **survived** (`capture_ax_ok=1`, `capture_auto_ok=1`,
      titles flowing straight after relaunch); the SMAppService login item survived too
      (`sfltool dumpbtm` still lists UsageOS, Developer ID team, generation 4 — D69 confirmed).
      So an in-place updater install does not trip the revocation; the 2 Jul loss came from a
      manual DMG replace. The "notification fires on a witnessed loss" path is still unexercised.


## Watch coverage — the app records when it wasn't watching (D72)

Found by the owner on 2026-09-20: **2026-09-16 and 09-17 hold zero rows.** The app exited
cleanly on 09-15 at 17:32:26 and did not run again until the 09-18 23:07 reboot brought it back
via the login item. The Mac was in use on both days (lid opened 3× and 4×, per `pmset -g log`);
there was no crash report and no jetsam kill. In ~90 days and 31,913 rows these are the **only**
two fully-missing days, and the 77.6 h gap is 3.5× the next largest — every other gap is an
ordinary overnight. So: a one-off outage, and a permanent class of silence.

- [x] Migration `0009_watch_coverage.sql` — one row per app run (`started_at`, `last_seen`,
      `ended_at`, `version`). Verified against a copy of the owner's real 31,913-row database.
- [x] `db/coverage.rs` — open / touch / close / reconcile / read / prune, plus the pure
      `coverage_gaps` derivation. 14 tests.
- [x] Lifecycle: claim a session in `setup` before capture starts; reconcile any session a dead
      process left open, at its watermark; close on `RunEvent::Exit` (catches ⌘Q, tray Quit,
      glance Quit and shutdown alike).
- [x] Heartbeat on the existing capture tick, throttled to 60 s. **A session is continuous** —
      sleep suspends the process, so heartbeat gaps are not blindness; only the stretches
      *between* sessions are. Otherwise every night would hatch.
- [x] Read model: `DayView.gaps` + `unwatched_secs`, `DaySlice.unwatched_secs` +
      `coverage_known`, `TimelineView.unwatched_secs`, `RecapFacts.unwatched_secs`.
- [x] The empty-day copy splits into its two honest cases in Rust; a partly-blind day gets the
      gap appended after its own story. The model prompt is unchanged on a fully-watched day, so
      D52's cached recaps still hit.
- [x] Dial renders a blind stretch as a **broken** track — solid faint = you were away,
      interrupted = we weren't looking. Timeline + glance empty states follow.
- [x] Retention prunes coverage with the events it describes, so a pruned day reads as
      unknowable rather than as a failure.
- [x] Gates: `cargo clippy -D warnings`, `cargo fmt --check`, 156 Rust tests, `tsc`, 46 TS
      tests, bindings regenerated (never hand-edited).
- [ ] **Not yet verified on device.** Needs a real run: the app must write a session row, the
      heartbeat must advance, and a quit-then-relaunch must produce a reconciled span with no
      phantom gap. Running `tauri dev` writes to the live database and applies 0009 to it.
- [ ] **Found while verifying D72 on device — a downgrade silently bricks the app.**
      `verify_applied_checksums(conn, !cfg!(debug_assertions))` makes a release build refuse to
      start when the DB records a migration it doesn't ship ("do not downgrade"). That failure
      then takes the silent path: `setup` returns `Err` → `build()` fails → `eprintln!` +
      `std::process::exit(1)`, and in a `--hidden` login launch there is **no window, no tray, no
      notification, and no recording** — indefinitely, with nothing to look at. Any rollback
      (Homebrew cask, a Time Machine restore, running an older build once) arms it. The fix is
      small: surface startup failure natively before exiting, and leave a marker the next
      successful run can explain. Same silent-failure family as D72 itself.
- [ ] Open: should the app self-restart? `KeepAlive` on the bundled agent **cannot** work —
      D69's trampoline `setsid()`s the real process out of launchd's job, so launchd would
      respawn the trampoline forever and never observe the app. A periodic supervisor plus a
      recorded quit intent is the shape that would; it needs its own decision.
- [ ] Open: ⌘Q is live (`tauri::Builder::default()` never disables the default macOS menu) one
      key from the ⌘W that merely hides the window, and the glance has a bare one-click `Quit`
      beside "Open UsageOS →". D72 makes their consequence visible; it does not make them
      harder to hit.

## Calendar — Year → Month → Day (designed, not built)

- [x] Boards: `design/month.html` (the three cell treatments compared; heat blocks ruled out
      because darkness-as-hours makes colour encode two things) and `design/calendar.html` (the
      drill-down: Year as a bar per day, Month as a dial per day, one `Calendar` tab).
- [x] Measured: the R77 ink casing is 1.25% of rendered size, so it is a true 1px only at an
      **80px dial**; at 80px month cells come out 126×130 inside a 984px frame, so the compliant
      size fits the window and the month scrolls. A year cell is ~29px, which is why Year cannot
      use the dial at all.
- [x] **Built (D73).** One `Calendar` tab between Week and Timeline, Year and Month as zoom rungs,
      a breadcrumb as the navigation model, click-through to Day.
- [x] **Both Year layouts ship and the user switches between them** — month rows (continuity) and
      wall calendar (day-of-week alignment). Two icon buttons in the stepper row, each with an
      accessible name *and* a tooltip; the choice persists in `calendar_year_layout`.
- [x] Year uses a stacked bar per day, not a shrunken dial: at a ~29px cell the R77 casing would
      render 0.36px and a 15-minute run would be sub-pixel. Height is geometry, so colour still
      encodes exactly one thing and there is no intensity axis to read as a streak.
- [x] Month keeps the dial at the verified-compliant 80px, in a Sunday-first grid matching the
      shipped `weekDays()`. Two tiles only — time tracked and days not recorded.
- [x] Deliberately absent: year totals, monthly averages, busiest month, category share, any
      month-over-month comparison, and a month recap.
- [x] Fixed 12 h bar ceiling — a bar means the same thing in January and July. Scaling to the
      busiest day would make every year look equally full.
- [x] Year is ONE bounded scan (`db::get_day_category_totals`), not 365 day reads, and never
      polls; Month polls only while it is the current month.
- [x] `vision.md` out-of-scope line amended: the **trends** ban kept verbatim (it is a value), the
      **month view** exclusion dropped (it was sequencing, and v1 shipped).
- [ ] Not boarded or built: keyboard traversal of the year grid, its focus ring, and what a
      screen reader announces cell by cell. Every cell has an accessible name today; 2-D arrow
      navigation does not exist.
- [x] **After the owner used it:** the Calendar opens on the **Month** rung (Year is one click
      up); a day opened from the Calendar or the Week shows a link back to it ("← October 2026",
      "← Week of 12–18 October 2026"), cleared when the tabs are used directly. The Year layout
      default was already the wall calendar; a saved `calendar_year_layout` overrides it, which is
      the likely reason rows showed.
- [x] Day view: hovering a "Where it went" row no longer filters. Entering a row committed the
      filter and leaving it restored the already-committed value, so it stuck; isolation is now
      click-only, which also means a cursor passing over rows while scrolling does nothing.
- [x] Owner checked in the running app: Month default, back link, hover — "look good".

