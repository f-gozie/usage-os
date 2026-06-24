# UsageOS Design System — "Bauhaus"

_Machine-readable design contract. **Frozen 2026-06-23** at the end of the design session. The **living visual source of truth is the interactive HTML library in `design/`** (open `design/index.html` — the gallery hub); this file is what the coding agent obeys. When they drift, the rendered `design/` library wins and this file is stale — fix this file._

> Built end-to-end as standalone, themeable HTML (`design/*.html`) so it can be viewed, inspected and clicked in a real browser. The `/design-login` push to Claude Design was unavailable from the CLI during the session; the `design/` library is self-sufficient regardless.

## Principles

- **Bauhaus, not generic.** Bold geometric type, primary inks, hard edges, strong asymmetric grid, flat (no gradients/shadows/glow). Earn every element; restraint over decoration.
- **Calm, never anxious.** No gamification, no alarm-red shaming. A drift into Reddit is shown matter-of-factly.
- **The dial is the signature.** One memorable idea executed precisely. Don't dilute it with competing flourishes.
- **Three themes, all first-class** (token-based): `paper` (light) + `warm` (warm charcoal) + `black` (near-black). Dark is *designed*, not auto-inverted.
- **Banned (AI tells):** purple-on-white gradients, Inter/Roboto/system-font defaults, the "card with a left accent border."

### Hard-learned rules (these bit us — obey them)

1. **Colour encodes ONE thing: category** (kind of activity — a fixed 5-item vocabulary: Work · Browsing · Messaging · Entertainment · Personal, D47). **Projects are identified by NAME (text), never colour** — they're an open-ended set, so colour doesn't scale and would clash with the category palette. (D34)
2. **Never put text inside a proportional segment.** A label sealed in a bar segment clips the moment the segment is narrow, the names are long, or there are many of them. Labels go *outside*, in a line that can wrap. (Within-context project share = a plain `usageos 1h 3m · nudge 39m` line, not a bar.)
3. **Write for someone who isn't a developer.** No permission jargon ("Screen Recording"), no insider lingo ("real, not inferred"), no niche-tech examples (`db.rs — usage_os`). Plain, honest, accurate. See Copy & voice.
4. **Theme-safe inverted bars.** The ink titlebar / footer / settings headers invert via `--bar-bg`/`--bar-fg`, not `--edge`/`--bg` (those produce dark-on-dark in the dark themes).

## Colour tokens

Every screen is driven by CSS variables on `[data-theme]`. Exact values (the contract):

| Token | `paper` | `warm` | `black` | Role |
|---|---|---|---|---|
| `--bg` | `#EEEBE1` | `#1A1916` | `#0E0E0E` | page background |
| `--surface` | `#F6F3EA` | `#24221A` | `#171719` | cards / raised |
| `--fg` | `#161616` | `#EDE9DD` | `#F4F4F2` | primary text |
| `--muted` | `#6A6A66` | `#938E81` | `#8C8C8A` | secondary text |
| `--edge` | `#161616` | `#4A453B` | `#2F2F2F` | structural borders (2–3px) |
| `--rule` | `rgba(22,22,22,.16)` | `rgba(237,233,221,.14)` | `rgba(244,244,242,.12)` | hairline separators |
| `--track` | `#D7D3C7` | `#39342B` | `#252525` | idle / empty arc + bars |
| `--c-deep` | `#1B45BE` | `#3358E0` | `#3A6BF0` | **Work** (slug `deep`) |
| `--c-research` | `#E0241B` | `#F0463C` | `#FF4B40` | **Browsing** (slug `research`) |
| `--c-comms` | `#EAB308` | `#F2BC0C` | `#F5C518` | **Messaging** (slug `comms`) |
| `--c-breaks` | `#161616` | `#CFC9B8` | `#C6C6C4` | **Entertainment** (slug `breaks`; ink in light → bone in dark) |
| `--c-personal` | `#2E8B57` | `#46B97E` | `#4FCB8A` | **Personal** (slug `personal`; D47) |
| `--now` | `#161616` | `#EDE9DD` | `#F4F4F2` | the "now" triangle |
| `--casing` | `#161616` | `transparent` | `transparent` | arc outline (R77) |
| `--on-ink` | `#EEEBE1` | `#1A1916` | `#0E0E0E` | text on a context fill |
| `--bar-bg` | `#161616` | `#100F0C` | `#000000` | inverted bar bg |
| `--bar-fg` | `#EEEBE1` | `#EDE9DD` | `#F4F4F2` | inverted bar text |

**Category → colour (the dial / legend / ledger / timeline):** Work → `--c-deep` · Browsing → `--c-research` · Messaging → `--c-comms` · Entertainment → `--c-breaks` · Personal → `--c-personal`. Idle = faint `--track`. (Token names keep the original slugs — D46/D47 — so they don't drift when display names change.)

### R77 — resolved (Option A, locked)

The locked Comms yellow `#F2BC0C` fails WCAG 1.4.11 non-text contrast on paper (1.47:1). Resolution **A** (chosen over "darken to mustard" and "outline only"):
1. **Every dial/mini-dial arc carries a 1px ink casing** (`--casing`) in light — `--edge` vs `--bg` ≈ 16:1, so the arc clears 3:1 regardless of fill. In dark themes `--casing` is `transparent` (bright arcs already clear 3:1 on the dark field).
2. **Comms is tuned to a richer gold `#EAB308` in `paper`** for legibility in small chips/swatches; it stays loud (`#F2BC0C`/`#F5C518`) in the dark themes.
3. **Colour is never the only cue** — legend chips, ledger rows, the timeline and the inspector always carry the text label too (WCAG 1.4.1).

## Type

- **Display / headlines / big figures:** **Anton** (poster grotesque, all-caps). Hero figures (dial centre, stats) and section headers are Anton.
- **Body / labels / UI / data:** **Jost** (geometric humanist), weights 400/500/600/700.
- Never Inter / Roboto / system default. No Space Grotesk.
- **Scale (approx):** display 34–80px · hero figure 28–44px · section head 16–22px · body 15–17px · label/UI 12–14px · eyebrow 10–12px (uppercase, `.14–.22em` tracking).
- **Sentence case for prose.** Caps only as the deliberate Bauhaus eyebrow device. **Curly apostrophes/quotes** (’ “ ”), never straight.

## Shape, spacing & motion

- **Hard edges.** Square by default; the app window frame is the only meaningful radius (`5px`). No rounded single-side accent borders.
- **Borders:** 2–3px solid `--edge` for structure (intentionally bold); 1px `--rule` for internal separators.
- **Flat fills only** — no gradients, drop shadows, blur, or neon.
- **Spacing:** generous; section padding ~22–28px; the app frame max-width 1000px (app) / 1080px (gallery & landing).
- **Motion:** one orchestrated load (dial arcs draw in with stagger via `stroke-dashoffset`, the now-triangle, week minis cascade). Hover: the arc lifts (stroke-width up) and others dim; legend/ledger isolate. The "Tracking" dot pulses (2.4s). Purposeful, not decorative. **Honor `prefers-reduced-motion`.**

## Components (built — see `design/`)

Window frame (ink titlebar: red/comms/blue dots + centred wordmark + pulsing "Tracking") · top nav (Day · Week · Timeline · Settings tabs) · **theme switcher** (Paper/Warm/Black segmented) · date navigator · **the dial** (24h fixed, midnight top, **context-run** arcs + casing, hour ticks at 0/6/12/18, now-triangle, centre figure; hover tooltip; click → session inspector) · **mini-dial** (week; casing scaled to 1px; theme-aware now-triangle) · **recap card** (AI-on prose + "⌁ Summarized on-device"; AI-off "≡ Template") · stat tiles · **legend chips** (click-to-isolate) · **ledger rows** (bar + Anton figures, hover-dim) · session inspector · **timeline session blocks** (context-run → `usageos 1h 3m · nudge 39m` line → apps line; click expands to granular per-app/per-project rows) · settings groups (inverted header bar) · **contexts/rules editor rows** · **exclusion rows** (tag + pattern + Exclude/Private pill) · toggles · segmented controls · pills/tags · buttons (primary/secondary/danger/ghost) · inputs (text: default/focus/error/disabled) · select (closed + open menu) · time picker · radio · colour swatches · **modals** (add context, add exclusion, delete-all confirm with type-to-confirm) · skeleton loaders · **degraded banner** · **onboarding stepper** (progress, permission cards, grant/skip) · landing page sections.

**Not yet designed (pending):** the **menubar popover** (vision §menubar). Design it before Phase-4 shell work.

## States to cover (no drift)

Each component, across: `paper` + `warm` + `black`; loading / empty / partial-day / full-day; permission-granted vs **degraded** (app-level only); **AI-on (prose) vs AI-off (template)**; private/excluded entries (no title); hover / focus / active / disabled.

## Data-representation rules

- **Aggregates (dial centre, stats, ledger, week) sum per axis** — robust to any segmentation.
- **Timeline & dial arcs render context-runs** (continuous same-context stretches); the project split shows as labelled durations inside the timeline block; off-project time (Slack, browsing — no project signal) counts to its **context**, shown as "no project." (D34)
- **Segmentation thresholds** (excursion-absorb, idle-gap, sustained-shift) are **not yet locked** — resolved by dogfooding real capture data (D34a). The block/expand layout holds regardless.

## Copy & voice

The app's payoff is prose, so the chrome must sound like a person, not a model. **Plain, honest, accurate, a little warm. No slogans, no lines that sound deep, no jargon, no insider lingo, no niche-tech examples.** Say the thing. Sentence case; curly quotes. Examples of the bar we hold: "Today" not "Today, in sequence"; "It can't see your screen" not "never Screen Recording"; "Get usageos" not "Own your time."

## Reference

- **`design/index.html`** — gallery hub → Foundations · Day · Week · Timeline · Settings · Components & states · Onboarding · Landing (+ `design/timeline-variants.html`, the A/B/C exploration behind D34).
- `prototype/index.html` + `prototype/landing.html` — the original prototypes that seeded the language (historical; the `design/` build supersedes them).
- Decisions: R77 + palette + D34 segmentation in `context/decisions.md`; segmentation framing in `context/plans/2026-06-22-product-redesign/explorations/2026-06-23-sessionization.md`.
