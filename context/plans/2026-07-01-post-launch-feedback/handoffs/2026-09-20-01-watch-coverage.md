# 2026-09-20-01 — Watch coverage (D72), the Today button, and the Calendar boards

## Where this started

The owner opened the Day view and found **2026-09-16 and 09-17 completely empty** — not quiet,
empty — and the app said about them exactly what it says about a day you chose to rest.

## What the forensics found

- Last row before the gap: **09-15 17:32:26**. First after: **09-18 23:08:30**.
- The Mac **was in use** on both days — `pmset -g log` shows lid-open wakes, 3 on the 16th and
  4 on the 17th.
- **No crash report** in either DiagnosticReports directory, and **no jetsam kill** of
  `usage-os` (it appears in jetsam process snapshots but was never the victim).
- `kern.boottime` puts the next reboot at **09-18 23:07:28** — one minute before the app
  returned. It came back only because the login item fired at boot.
- The updater is **ruled out**: `last_update_check` is 09-18 23:08:49, nowhere near the 15th.
- In ~90 days and 31,913 rows these are the **only** two fully-missing days, and the 77.6 h gap
  is 3.5× the next largest — every other gap is an ordinary overnight.

So: the process **exited cleanly and nothing restarted it for three days.** The exact trigger is
unrecoverable — the unified log only persists back to 09-19 — but three one-click clean-exit
paths are live and all fit: **⌘Q** (`tauri::Builder::default()` never calls
`enable_macos_default_menu(false)`, so Tauri's default macOS menu ships a working Quit, one key
from the ⌘W that merely hides the window), the **glance popover's bare `Quit`** beside
"Open UsageOS →", and the **tray's `Quit UsageOS`**.

**It happened again during this very session** — the app stopped writing at 03:02 on 09-20 and
the process was gone, with no crash report and (this time) no unified-log data at all for the
window. Two occurrences in five days. Which makes the real finding not the exit but the silence.

## What shipped — D72, branch `feat/watch-coverage`

The app now records **its own presence**, so blindness is derived rather than guessed.

- `0009_watch_coverage.sql` — one row per run (`started_at`, `last_seen`, `ended_at`, `version`).
- `db/coverage.rs` — open / touch / close / reconcile / read / prune + the pure `coverage_gaps`
  derivation. 14 tests.
- Lifecycle in `lib.rs`: claim a session in `setup` before capture starts; reconcile any session
  a dead process left open **at its watermark**; close on `RunEvent::Exit`.
- Heartbeat on the existing capture tick, throttled to 60 s.

**The load-bearing decision: a session is continuous.** Sleep suspends the whole process, so
heartbeats stop for hours every night. Treating that as blindness would hatch every night on
every surface and the mark would become noise. Real gaps are the stretches **between** sessions —
exactly the shape of the 09-15→09-18 outage.

Read model: `DayView.gaps` + `unwatched_secs`, `DaySlice.unwatched_secs` + `coverage_known`,
`TimelineView.unwatched_secs`, `RecapFacts.unwatched_secs`. The empty-day copy splits in Rust
into its two honest cases; a partly-blind day gets the gap **appended** after its own story. The
model prompt is unchanged on a fully-watched day, so **every recap cached under D52 still hits**
(there is a test pinning that). The dial draws a blind stretch as the same track deliberately
**broken** — solid faint = you were away, interrupted = we weren't looking.

## On-device verification (2026-09-20, `tauri dev` against the real database)

1. Migration 9 applied cleanly to the live **31,919-row** database.
2. Session row opened at 03:00:51 on startup.
3. Heartbeat advanced to 03:01:51 — **exactly the configured 60 s**.
4. `SIGTERM` does **not** fire `RunEvent::Exit`: the row stayed `OPEN` at its watermark
   (03:02:51, 18 s before the kill). That is the designed behaviour — an unclean death leaves
   the watermark and the next start reconciles it. Only a real app quit closes the row exactly.
5. The resulting live gap crossed `GAP_MIN_SECS` at **312 s** and became reportable, on real data.

## On-device verification, continued — the whole chain, on the real machine

6. Restarting produced exactly the outage shape, detected:

   ```
   session 1  03:00:51 → 03:02:51   (reconciled closed at its watermark)
                   ↓  420 s unwatched  ↓
   session 2  03:09:51 → open
   ```

   with `[Coverage] closed 1 session(s) left open by a previous run` in the log. 420 s clears
   `GAP_MIN_SECS`, so this is a gap the dial draws and the recap mentions.

## The machine was left restored — read before re-running dev

Running `tauri dev` migrates the **live** database to schema 9, and
`verify_applied_checksums(conn, !cfg!(debug_assertions))` makes a **release** build refuse to
start when the DB records a migration it doesn't ship. So while schema 9 was applied, the
installed v0.1.3 **could not start** — and would have failed *silently*: `setup` → `Err` →
`eprintln!` + `std::process::exit(1)`, which in a `--hidden` login launch means no window, no
tray, no notification, no recording.

**This was reverted at the end of the session.** Verified after: schema **8**, **31,932 activity
rows**, `watch_coverage` dropped, `PRAGMA integrity_check` **ok**, and the installed app
relaunched and writing rows again. Backup kept at
`~/Library/Application Support/com.usageos.app/usage.db.pre-coverage.bak`.

To re-test, run `npm run tauri dev`, and revert afterwards before relying on the installed app:

```sql
DROP TABLE IF EXISTS watch_coverage;
DELETE FROM schema_migrations WHERE version = 9;
```

This is **not just a dev artifact** — any rollback (Homebrew cask, a Time Machine restore,
running an older build once) arms the same silent brick. Recorded as an open plan item: surface
startup failure natively before exiting, and leave a marker the next successful run can explain.

## Also landed

- **Today button** — branch `feat/today-button`, committed (`2a6dada`), not pushed. Added to the
  shared `DateStepper`, so Day/Timeline get "Today" and Week gets "This week". Always rendered
  and disabled at the latest range, like the existing "next", so the row never reflows.
- **`.claude/skills/usageos-mockup/SKILL.md`** — the project's own mockup skill, adapted to this
  repo's actual layout (`design/*.html` flat, `context/design-system.md`, verdicts to
  `decisions.md` + `design/README.md`; deliberately no `design/approved/`).
- **`design/month.html`** and **`design/calendar.html`** — the Month-only round (three cell
  treatments compared, heat blocks ruled out) and the Year → Month → Day drill-down, with a
  Month-rows / Wall-calendar icon toggle on the Year rung.

## Open, for the next session

1. **Revert the live DB** (above) or build+install the coverage version — the machine is in a
   state where the installed app won't start.
2. Decide the Calendar questions: year layout (rows vs wall), the bar-height ceiling
   (fixed 12 h vs scale-to-busiest), and whether Calendar is one tab or two.
3. `vision.md:63` scopes "long-range trends/month view" out of **v1**. v1 shipped; amend the line
   as part of building Calendar — keep the ban on *trends*, drop it for the month view.
4. Self-restart is **not** decided. `KeepAlive` cannot work: D69's trampoline `setsid()`s the real
   process out of launchd's job, so launchd would respawn the trampoline forever and never observe
   the app. A periodic supervisor plus a recorded quit intent is the shape that would.
5. ⌘Q and the one-click glance `Quit` are untouched. D72 makes their consequence visible; it does
   not make them harder to hit.
6. `DayView.tsx:59` — the tile labelled **"Focus"** computes `(deep + research) / active`, i.e.
   Work **+ Browsing**. Doom-scrolling in Chrome counts toward Focus, and it is the only scoreboard
   in an app that cut scoring on purpose.

## Gates at handoff

`cargo clippy -D warnings` clean · `cargo fmt --check` clean · **156 Rust tests** · `tsc` clean ·
**51 TS tests** (14 files) · bindings regenerated, never hand-edited.

Committed locally, **not pushed**: `feat/watch-coverage` (`30b0fd4` the feature, `eb02c29` the
design boards + mockup skill) and `feat/today-button` (`2a6dada`).

**Machine health noted during the session:** swap was 7.2 GB of 8 GB used with a load average
above 8. That is a hostile environment for a background app, and worth keeping in mind as a
possible contributor to the app dying twice in five days — though no jetsam report ever named
`usage-os` as the victim.
