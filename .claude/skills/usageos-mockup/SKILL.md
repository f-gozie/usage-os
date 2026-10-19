---
name: usageos-mockup
description: Design any UsageOS surface as an approval-ready HTML board in design/ before it gets built. Use for "mockup", "design this", "what should it look like", "show me how it'd work", "give me options", and for any request that adds or reworks a screen, view, card, panel, sheet, overlay, empty/error state, or motion that carries meaning — including when that surface is buried inside a plan or a feature request that never says "design". Not for copy-only edits, Rust/data work, building an already-approved unchanged design, or fixing shipped UI that drifted from an approved board.
---

# UsageOS mockup

UsageOS is designed **by eye, in the browser**. A surface becomes a standalone HTML file in
`design/` — no build step, openable by double-click — and gets tuned against the real tokens
before a line of React is written. The board is the spec: what ships matches the approved board.

This skill is the UsageOS-specific version of the general `mockup` skill. Where they differ,
this one wins, because it names this project's actual paths and its actual scars.

---

## Where things live (this project's layout — do not invent another)

```
design/
  index.html          gallery hub — every board links from here
  foundations.html    tokens, type scale, colour, motion
  components.html     the component library + states
  <surface>.html      one file per surface: day · week · timeline · settings · onboarding · menubar
  <surface>-variants.html   option-explorations behind a specific choice
  assets/app-icons/   real exported app icons — use these, never emoji or redrawn glyphs
  logo/               the Contexts dial-mark system
context/design-system.md   the machine-readable contract the coding agent obeys
context/decisions.md       where a design verdict is recorded (append-only ADR, D1…)
```

**There is no `design/approved/` and no `design/system.md`.** Do not create them. This project
already records verdicts in two places and a third would be drift:

- **The why** → append an ADR to `context/decisions.md`.
- **Which board is live, which were set aside** → a line in `design/README.md`, the way
  `landing-*.html` and `timeline-variants.html` are already handled there.

If a board changes a token or adds a component, update `context/design-system.md` in the same
step — that file is the contract, and a stale contract is a build failure (CLAUDE.md).

---

## Step 0: read the contract, every time

Before drawing anything:

1. **`context/design-system.md`** — the full token table, the type scale, the hard-learned rules.
2. **The nearest existing board** in `design/`. Copy its `<style>` preamble verbatim rather than
   retyping tokens from memory. Its `:root` / `[data-theme]` blocks are the source.
3. **The React component that owns the surface**, if one exists (`src/views/`, `src/components/`)
   — for real prop names, real strings, real states, real nav items.
4. **`context/decisions.md`** for anything the surface touches. A board that reopens a locked
   decision without saying so wastes a round.

Never hard-code a palette, radius, or nav list from memory, including from an older board.
Boards are snapshots and snapshots rot — see the drift note at the bottom of this file.

---

## Gates

Five. A board that misses one has failed, however good it looks.

**1. Approval before implementation.** No React for a visual change until the user says yes in
words. Under time pressure the *board* gets smaller — fewer frames, rougher composition. The
approval is not skipped. Do not infer a waiver from a short reply or from urgency.

**2. All three themes, from the same tokens.** `paper` · `warm` · `black` are all first-class.
Dark is *designed*, not auto-inverted. Put a working theme switcher on every board (copy the one
from an existing file) and actually click through all three before presenting. A light mode that
special-cases values inside a component is a defect — theme values belong in the token layer.

**3. The accessibility floor, shown on the board.**
- **Colour is never the only cue** (WCAG 1.4.1). Every arc, chip, ledger row and block carries its
  text label too. This is locked by R77 and is not negotiable for a new surface either.
- **Every dial/mini-dial arc carries the 1px `--casing`** in `paper`; `transparent` in the dark
  themes. This is how Comms yellow clears 3:1 non-text contrast (R77 Option A).
- Board the layout at **large text** and show it surviving.
- Contrast legible in all three themes. Check the yellow. It is always the yellow.

**4. No emoji as UI, no real user data.** Use `design/assets/app-icons/` and real SVG. Content is
realistic but synthetic — believable window titles, plausible durations, no real people.

**5. The words are product copy.** Every string is a real proposal, held to the voice below.

---

## Voice (the chrome has to sound like a person)

The app's payoff is prose, so the chrome cannot sound like a model. **Plain, honest, accurate, a
little warm. No slogans, no lines that sound deep, no jargon, no insider lingo, no niche-tech
examples.** Say the thing.

- Sentence case for prose. Caps only as the deliberate Bauhaus eyebrow device.
- Curly apostrophes and quotes (’ “ ”), never straight.
- "Today" not "Today, in sequence". "It can't see your screen" not "never Screen Recording".
- Write for someone who is not a developer. No permission jargon, no `db.rs — usage_os` examples.
- Never "notarized" or any trust jargon in user-facing copy.

---

## The board

**Every state, not the happy path.** One pretty default frame is a failed mockup. For UsageOS the
states that actually matter:

- **empty** — and specifically: *a day with nothing recorded.* Distinguish "you didn't use the
  Mac" from "we weren't watching." These are not the same day and must not look the same.
- **partial day** (it's 11am, the dial is a third full) and **full day**
- **loading** — known chrome renders immediately; only data-dependent values get a skeleton,
  inside the real component shape. Headers and labels never shimmer.
- **error / retry**
- **degraded** — a permission that was granted is now gone (the banner, the tray affordance)
- **the recap** on a full day, a day still going (“so far”), a tiny day and a partly-unwatched day — the app writes it from the facts (D74), so there is no AI-on/off pair
- **private / excluded entries** — time and app only, never a title (D8)
- hover · focus · active · disabled

**Board the comparison.** Asked for the empty state, board the populated one too. Asked for
collapsed, board expanded. The user is judging a difference; show both sides.

**Truthful chrome.** The ink titlebar, the Day · Week · Timeline · Settings nav, the pulsing
"Tracking" dot. Show what exists on that screen today. Confirm a route exists before drawing it.

**Realistic density.** Long awkward window titles, a crowded 14-hour day, a project name that
doesn't fit. Never "Item 1", never a suspiciously tidy three-row list. Placeholder content is the
most common reason a board bounces.

**Notes live outside the app frame.** Nothing inside the simulated window is annotation.

---

## Make it tunable

This is the UsageOS house habit and it is worth the extra effort: when a board has a value that
can only be judged by eye — an arc width, a gap, a tick length, a cascade delay, a type size,
an opacity — **ship a live `<input type="range">` for it with a numeric readout**, wired to the
CSS variable it drives.

The point is that the reviewer tunes it themselves and reports back a number, instead of a round
trip per pixel. Group the sliders in a panel outside the app frame, label them in plain words,
and print the current value beside each one. A "copy current values" button is a nice touch.

---

## Motion

One orchestrated load, then quiet. Arcs draw in with stagger via `stroke-dashoffset`; the
now-triangle lands; week minis cascade. Hover lifts the arc (stroke-width up) and dims the rest.
The Tracking dot pulses on a 2.4s cycle.

**Always honour `prefers-reduced-motion`**, and board the reduced-motion reading alongside the
animated one. If the motion carries meaning, show both — that is not a trade-off.

Never animate opacity over a material. Never add a second competing flourish to a surface that
already has the dial — the dial is the signature and diluting it is the one unforgivable move.

---

## Restraint, and what's banned

- **Bauhaus, not generic.** Bold geometric type, primary inks, hard edges, strong asymmetric grid.
- **Flat only.** No gradients, no drop shadows, no blur, no glow, no neon.
- **Square by default.** The app window frame's `5px` is the only meaningful radius.
- **Borders:** 2–3px solid `--edge` for structure; 1px `--rule` for internal separators.
- **Type:** Anton for display and hero figures, Jost for everything else. Never Inter, Roboto,
  system default, or Space Grotesk.
- **Banned as AI tells:** purple-on-white gradients; the card with a coloured left accent border;
  generic icons with no specific idea (a bolt, a sparkle).
- **Calm, never anxious.** No alarm-red shaming, no scores, no streaks, no goals, no XP. A drift
  into Reddit is shown matter-of-factly. If a frame would make someone feel judged, it's wrong.

---

## Data-representation rules (these bit us — obey them)

1. **Colour encodes ONE thing: category.** A fixed five: Work (`--c-deep`) · Browsing
   (`--c-research`) · Messaging (`--c-comms`) · Entertainment (`--c-breaks`) · Personal
   (`--c-personal`). Idle is faint `--track`. (D46/D47 — token slugs keep the original names so
   they don't drift when display names change.)
2. **Projects are identified by NAME, never colour.** They're an open-ended set; colour doesn't
   scale and would clash with the category palette. Within-context project share is a plain
   `usageos 1h 3m · nudge 39m` line, not a bar. (D34)
3. **Never put text inside a proportional segment.** It clips the moment the segment is narrow or
   the name is long. Labels go outside, in a line that can wrap.
4. **Aggregates sum per axis** (dial centre, stats, ledger, week) so they're robust to any
   segmentation.
5. **Theme-safe inverted bars.** The ink titlebar, footer and settings headers invert via
   `--bar-bg`/`--bar-fg`, never `--edge`/`--bg` — those give dark-on-dark in the dark themes.
6. **Never invent a value, never show a stale zero for "unknown."** On a quiet day, hiding a
   section usually beats an empty box.

---

## Open it before presenting it

Render the file and look at it — browser or screenshot tool. Check for missing fonts, clipping,
overflow, weak contrast, stale chrome, and a theme you forgot to click. Compare against the live
React component. **Do not report that it renders without having looked at it.**

Then add it to `design/index.html` so it's reachable from the hub. A board nobody can find is a
board that rots.

---

## The ask, and the record

Ask for approval explicitly. Name the decisions the board asks the user to make, and say what you
deliberately left open.

Only after an explicit yes:
1. Append an ADR to `context/decisions.md` — the winning option, **the rejected options and why**,
   locked strings, locked behaviour. A record listing only the winner has thrown away the useful
   half; the rejection reasons are what stop the same idea coming back in three months.
2. Update `design/README.md` to say which board is live and which are superseded.
3. Update `context/design-system.md` if a token, component or rule changed.
4. Note it in the active plan's `plan.md` (CLAUDE.md, Definition of Done).

---

## Lessons from this project's rounds

Each with its reason — the reason is what makes it reusable.

- **"Make it more realistic"** almost always means placeholder content or missing app chrome.
- **Controls do not move between states.** A state may remove a control, but nothing slides into
  the empty slot.
- **Known chrome never shimmers.** Only data-dependent values get a skeleton, inside the real
  component shape, so the swap is a clean crossfade.
- **One control vocabulary everywhere.** One back control, one card, one primary button, one
  section-label style.
- **Icons need a specific idea.** Generic icons get vetoed for being generic.
- **An empty box on a quiet day reads as failure.** Hide the section instead, or say plainly why
  it's empty.
- **The dial is the signature.** Every new surface is asked: does this compete with the dial? If
  it does, it loses.

### Known drift — check before copying a board

The `design/` library is the visual source of truth, but it is **not uniformly current**. As of
this writing only `category-palette-5.html` and `menubar.html` carry the `--c-personal` token,
though Personal has been a canonical category since D47. Eighteen of the twenty boards still show
the old four-category palette.

So: **take your token preamble from a board you have verified, not from the nearest one.** If you
touch an old board, bring its palette forward while you're in there, and say you did.
