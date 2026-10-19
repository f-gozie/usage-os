# 2026-09-20-02 — Calendar built (D73), then reworked after the owner tried it

Continues `2026-09-20-01-watch-coverage.md`. Branch `feat/watch-coverage`, committed, **not pushed**.

## What landed

- **D73 — Calendar.** One tab between Week and Timeline; Year and Month as zoom rungs; a
  breadcrumb as the navigation model; click-through to Day. **Both Year layouts ship** behind a
  two-icon switch (month rows / wall calendar), persisted in `calendar_year_layout`.
- Backend: `get_month`, `get_year`; Year is **one bounded scan** (`get_day_category_totals`) that
  splits a midnight-crossing span between both days, never 365 day reads. Year never polls.
- `feat/today-button` is merged in (the Calendar's stepper needs `onLatest`).
- `vision.md` out-of-scope line amended: the **trends** ban kept verbatim, **month view** dropped.

## The owner tried the first build and said it wasn't good enough. They were right.

> not 100% parity… the icon switcher, the hover states, the responsiveness… clicking and the
> animations, transitions… we need a buttery smooth experience and our architecture needs to
> reflect that

The first build had **never been looked at in the running app** — the automation couldn't target
the unbundled dev binary, and it shipped anyway. That is exactly what `usageos-mockup` calls drift.

**Architecture, so smoothness is structural rather than hoped for:**
- A per-day view-model is built **once per data load** (`yearModel.ts`), not inside each cell's
  render, with one shared `Intl.DateTimeFormat`.
- `useMinuteNow` — `Date.now()` in render had been handing all 365 cells a new prop every render.
- Cells are `memo`'d and carry **no callbacks**; each grid has one delegated listener (`data-day`).
- Motion is CSS-only, `transform`/`opacity` only, staggered by one `--i` custom property; the
  existing global reduced-motion block collapses all of it.

**Parity defects found by measuring the real render:** wall calendar used viewport breakpoints
(`lg:` = 1024px) that never fire in a ~1000px window → shipped 3 columns, board draws 4; wall bars
hugged the left of the cell (button shrank to content); row bars were a fixed 13px (skinny in a
wide window); the picker used the OS tooltip instead of the board's ink chip; outgoing/incoming
tooltips overlapped mid-fade. All fixed.

**A copy bug found only by looking:** every day of history was labelled "before UsageOS was
recording" with its hours dropped — because every historical day has unknown coverage. Unknown
coverage means the coverage *log* didn't exist, not that nothing was recorded. Fixed and pinned.

## How things were verified — and what was NOT

| Claim | How it was checked |
|---|---|
| Year bars, wall layout, month alignment, real data | Native app, **zoomed to native resolution** |
| No element overlays the picker / month names / bars | `elementFromPoint` in a real browser engine |
| Layout switch both ways | Trusted pointer clicks (browser); keyboard Tab+Space (native app) |
| Month drill-down, breadcrumb back, bar → Day | Bubbling clicks on the inner `<span>`, real engine |
| Wall bars centred | Measured: **0px** off-centre across 40 cells |
| Row bars scale with the slot | Measured: 12.5px in 29.3px (0.43), 34px row per the board |
| Cells don't re-render on parent re-render | Render-count test **plus a control** that must rise |

**Not verified, stated plainly:**
- **How hover, press and transitions *feel* with a real mouse in the real WKWebView.** The
  background automation cannot hover, and taking over the screen while the owner slept was not
  appropriate. "Buttery" is the owner's call to make; the structure that should produce it is in.
- The final three parity fixes were measured in a browser engine, **not re-viewed in a rebuilt
  native bundle** — another release build was skipped on purpose (load average was 22–38 with
  other sessions working).
- Pointer clicks into the native app via the automation never registered after the Calendar
  mounted (the tool marks them "unverified"). The browser evidence says the app is fine; a real
  mouse in the real app is still the final word.
- One test failed **once** in a full run (first test of `CalendarView.test.tsx`, under a release
  compile with swap exhausted) and never again in 8 runs. The error text was not captured, so
  "5s timeout under load" is an **inference**. The file has an explicit, commented timeout.

## Mistakes made this session, so they aren't repeated

- Wrote a render-cost test whose filter ended in `&& false` — it could not fail. Caught on
  re-read and replaced with a real counter plus a control case. A green rigged test is worse than
  none: it would have been reported as evidence.
- Overwrote `Dial.test.tsx` without reading it, destroying two tests. Restored; mine appended.
- **Three times** diagnosed from a 0.5–0.6-scale screenshot and was wrong each time ("the year is
  empty" — it was mid-entrance; "wall bars are scattered dashes" — they were fine). Zoom to native
  resolution or measure the DOM before concluding anything.
- A waiter used an unanchored `grep error` and reported a healthy build as failed (`thiserror`).
- Two `pgrep -f` loops matched their own command line and never exited.

## ⚠️ Two things the owner needs to know

**1. The build was NOT installed, deliberately.** `tauri build` here produces an **ad-hoc** signed
bundle (`Signature=adhoc`, no team); the installed app is Developer ID + hardened runtime. Running
the ad-hoc bundle from `target/` showed the consequence on screen: *"Window titles aren't being
recorded."* Installed over `/Applications`, it would have broken the SMAppService login item (D69)
and silently killed Accessibility (D71). The Developer ID key **is** in the keychain and was **not
used**: signing a release artifact under the owner's Apple identity was not something that was
asked for, and it still would not have been notarized. Shipping this needs the real release path.

**2. A GitHub personal access token was exposed in this session's output.** A `pgrep -fl` printed
the full argv of unrelated processes, and an MCP server process carries a `github_pat_…` token in
its arguments. It was not used for anything and its validity was not tested. It is visible to any
local process via `ps` regardless of this session, so **consider rotating it** and moving it out
of process arguments. Process listings here now use names only (`pgrep -x`, `ps -o comm`).

## To test

```bash
./scripts/dev-with-restore.sh
```

Backs up the DB, runs dev, and on exit **always** reverts the schema and relaunches the installed
app — because a release build silently refuses to start against a newer migration. Dev builds are
ad-hoc signed too, so expect the "window titles aren't being recorded" banner while it runs.

## Machine state at handoff

Installed Developer-ID app running, database open, **schema 8**, integrity ok. No dev servers, no
stray processes. Backups: `usage.db.pre-coverage.bak`, `usage.db.pre-calendar.bak`.

## Gates

clippy 0 · fmt clean · **166 Rust tests** · tsc clean · **89 TS tests** (17 files) · bindings
regenerated, never hand-edited.
