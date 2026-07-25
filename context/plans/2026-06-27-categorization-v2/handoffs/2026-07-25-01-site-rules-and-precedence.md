# Handoff — site rules + precedence shipped (W1 + W2)

**Date:** 2026-07-25 · **Plan:** categorization-v2 · **PRs:** #39 (rules), #40 (landing analytics) · **ADR:** D70

## How this session started

Not from the plan. The question was "why does UsageOS have no traction, and I don't
even use it myself" — so the session began by reading the **actual database** (32 days,
14,688 spans, 250.0 active hours) rather than the code. The plan's W1/W2 turned out to be
the answer, with numbers attached.

## What the data said

- **Entertainment: 0.02 h out of 250 h**, next to 4.8 h YouTube, 2.5 h football streams,
  1.6 h Prime Video, 1.3 h DStv, 2.0 h Spotify. The dial was demonstrably lying.
- **In-app usage decayed to zero:** 52 min on day 3 → under 3 min/day through July →
  last opened 20 July. Recap cache agrees (46 recaps on 25 Jun, 1/day from 6 Jul).
  The tracker never stopped; the looking stopped.
- Traction, for the record: 24 DMG downloads across both releases, 5 stars, 0 forks,
  0 issues, 3 unique repo visitors in 14 days, last push to origin 2 July. Nothing was
  ever announced — this is a distribution gap, not a product verdict.

## What shipped (see D70 for the full rationale)

W1 + W2 together: `site` as a match field, precedence by field specificity
(site > title > process, then id), host-boundary site matching, mirrored in
`reprocess_logs`, seeded by migration `0008` with a one-time historical reprocess.

**Measured before/after on the real 32 days:** Entertainment 0.03 → 9.43 h,
Browsing 70.43 → 36.72 h, Messaging 5.08 → 9.20 h, Work 171.84 → 192.03 h.

## Gotchas for the next session

1. **Don't run a branch migration against the live app's database.** Doing it here
   bricked the installed v0.1.1: the release drift guard treats a recorded-but-unknown
   migration as a downgrade and hard-errors at startup (correctly — D54). Recovered from
   `usage.db.pre-siterules.bak`; the app is running on schema 7 and **the author's local
   data is NOT yet recategorized**. It will re-sort automatically on the first launch of
   a build containing `0008`. The recategorized copy is preserved outside the repo.
2. **`match_field_tier` is `pub(crate)`** because `events.rs` needs it to keep the bulk
   path in step with the matcher. If a third caller ever appears, that's a smell — the
   two paths should probably share one iterator instead.
3. The four seeded `breaks` process rules (`Reddit`, `YouTube`, `Netflix`, `Steam`) are
   still in the table and still can never match — they're tabs, not apps. Left alone
   deliberately (never delete a user's rules), but they're noise in the Settings list.

## Discovered, NOT fixed — the bigger bug

**Window titles have been empty since 2026-07-02, and browser URLs are intermittent.**

| | |
|---|---|
| Title coverage, 1 Jul | 86.6% |
| Title coverage, 2 Jul | 5.4% |
| Title coverage, since 4 Jul | ~0% |
| `/Applications/UsageOS.app` replaced | **2 Jul 00:53** |

The v0.1.1 install is the inflection point, to the hour. URL capture is separately flaky
(0% on 22–23 Jul, 100% on many others), which is consistent with Automation grants rather
than a latch in `browser.rs` — that code spawns a fresh `osascript` per call and has no
sticky failure state. Most likely a TCC grant invalidated when the bundle was replaced,
never re-prompted because the app launches hidden as a login item.

**For 23 of 32 days the app has recorded apps-only.** That is the user-visible symptom
("I see the app but not the url or details") and it plausibly hurts more than the
categorization bug did. `DegradedBanner` + `useCaptureHealth` already exist — the failure
was *detected* and simply never *surfaced*, because a menu-bar app that nobody opens has
nowhere to put a banner. Next session should reproduce, confirm the TCC theory, and decide
where a degraded state gets announced (tray icon state? notification? both?).

## Next

- [ ] Merge #39 + #40; cut a build so the author's own history re-sorts.
- [ ] PostHog project + `PUBLIC_POSTHOG_KEY` in Cloudflare Pages (both environments).
- [ ] The capture-permission regression above — likely the real v0.1.2.
- [ ] W3 (default exclusions) still open.
