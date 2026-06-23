# Exploration — what is a "session"? (OPEN, fundamental)

_Status: **SHAPE LOCKED → Direction A (context-runs + project-as-detail); thresholds still OPEN, pending dogfood.** Raised 2026-06-23 while reviewing the Timeline mock; the three directions were mocked (`design/timeline-variants.html`) and **A was chosen** (B/C rejected on sight). Recorded as **D34**. The Timeline + dial now render context-runs (`design/timeline.html`, `design/day.html`). What remains open is the **threshold tuning** (excursion-absorb, idle gap, sustained-shift sub-split) — to be settled by **dogfooding real capture data**, not theory._

## Why this matters

Capture produces a second-by-second stream of raw focus events (`app, title, url, site, project_id, context_id, is_idle, start, end`). Every product view is a way of **segmenting / aggregating that stream**. The Timeline mock assumed clean ~90-min single-app blocks — a fiction. Real usage (the user's own words) is rapid switching between apps **and** between projects (usageos ↔ nudge ↔ eyemark), often many times a minute. So "what is one unit of the day?" is load-bearing for the Timeline and the Recap, and it's currently undefined.

## The core tension: two axes that move independently

The data has **two orthogonal axes** (D2): **context** (kind of work — Deep / Research / Comms / Breaks) and **project** (what — usageos / nudge / none). They change **independently**:

- Hold **context** steady, flip **project**: a heads-down coding stretch bouncing usageos ↔ nudge.
- Hold **project** steady, flip **context**: usageos work going code → read docs → Slack a teammate about it → back to code.

So **no single "session boundary" captures both.** Any rigid key fragments one axis or hides the other.

## Naive rules and how they fail

- **N1 — segment by (context ∧ project):** the user's concurrent-projects case **shatters** into dozens of tiny segments. ✗ (this is the problem we hit)
- **N2 — segment by context only:** clean blocks; "Deep work 9:00–10:30" reads well, project hopping doesn't fragment it — but the block **hides** that it was two projects (must show as inside-detail). △
- **N3 — segment by project only:** project-hopping shatters it, **and** it orphans all no-project time (Slack #general, generic browsing, YouTube). ✗

## What each view actually needs

- **Dial, Ledger, Week** = **aggregates** (sum by context / by project). **Robust to any session definition** — they don't need sessions at all. The dial colours by *context*, so 40 rapid app switches that are all Deep work already collapse into one clean arc.
- **Timeline + Recap** = the only places that need **segmentation**. The Recap is prose, so it can *describe* interleaving in words ("you split the morning between usageos and nudge"). The Timeline is the one view that needs a concrete unit.

## Candidate models (to test on real data)

- **S1 — Context-runs + project detail:** Timeline unit = a run of the same *context*; inside, show the project split + apps. Consistent with the dial; project hopping never fragments; project shown as texture. _Lean default._
- **S2 — Focus blocks:** top-level unit = a continuous **active stretch** bounded by idle/away (and maybe by Breaks). Inside: context mix %, project mix %, and **fragmentation** (switch count, longest unbroken sub-run). Matches how people remember days ("a solid 90-min morning, lunch, then…"); interleaving is just "mixed." Possibly too coarse alone → nest S1 inside it (block → context-run → raw event, 3 levels).
- **S3 — Two-axis tracks:** Timeline shows two parallel bands — a context band and a project band — read independently. Honest about orthogonality; heavier UI.
- **S4 — Center-of-gravity / hysteresis:** a session = a stretch with a *dominant* (context, project); brief excursions absorbed; ends only when the dominant shifts and *stays* shifted. Smart, but needs tuned thresholds → exactly what dogfooding sets.

Whatever wins needs a **hysteresis / min-duration** rule so sub-minute flickers (a 20s Slack peek, a Spotify song change) are absorbed, not turned into segments.

## Edge cases the rule must survive

1. Interleaved projects, same context (usageos ↔ nudge deep work) — the core case.
2. Same project, changing context (code → docs → Slack-about-it → code): does that Slack count as "usageos time" or "Comms time"? (project-vs-context priority)
3. No-project time interspersed (Slack #general, generic browsing, YouTube) — where does it sit between project work?
4. Micro-excursions (30s Slack, song change, quick Stack Overflow lookup) — absorb into the surrounding block.
5. Work-related "leisure" (a YouTube tutorial that's actually for work) — Breaks or Research?
6. **Active distraction** (Reddit for 20 min while "working") vs **idle/away** (lunch, no input) — different things; both are off-project.
7. Long single-context blocks (a 1-hour Zoom; a 2-hour deep stretch).
8. Two windows/instances of the same app on different projects.
9. Ambiguous project (localhost, dashboards) — D30 abstains; correlate to the project active in the editor at the same time?
10. Day boundary / overnight (the D14 4 AM offset).
11. Mostly-idle or very-short days.
12. Context-label uncertainty (rules now, embeddings later) — segmentation must not amplify a wrong label.

## Resolution plan — dogfood, then decide

The capture pipeline is **built and on-device-verified**. So:

1. **Run capture for ~3–5 real days** (the user's machine). It records the raw stream silently.
2. **Export the events** (the planned CSV export, or a quick DB dump).
3. **Build a "session explorer"** — replays the real events under each candidate rule with tunable params (min-duration, excursion-absorb threshold, idle-gap, project-as-boundary on/off, hysteresis window) and renders the resulting Timeline/blocks.
4. **Compare against the user's lived memory** of those days; tune until the segmentation matches how the days actually felt.
5. **Encode the winning rule + write the ADR;** update Timeline + Dial (draw context-runs).

The **visual design system is not blocked** by this — the Timeline's *visual* pattern (grouped blocks, expandable to detail) holds regardless of the exact rule; the rule is a read-time parameter. Freeze the visuals; resolve the rule in parallel via dogfood.

## Open questions for the user (asked 2026-06-23)

1. Concurrent projects: one work block with project shown inside, vs separate per project, vs two parallel tracks?
2. Off-project work (Slack/docs *about* usageos): counts to the project, or to the kind-of-work (Comms/Research)?
3. What should the day's story optimise for: time-per-project, time-per-kind-of-work, focus-vs-scatter, or a plain narrative?
