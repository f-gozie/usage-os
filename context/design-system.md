# UsageOS Design System — "Bauhaus"

_Machine-readable design contract. The Claude Design project is the living visual source of truth; this file is what the coding agent obeys. They must stay in sync — when they drift, the rendered Claude Design project wins and this file is stale._

> To be fully designed end-to-end in Claude Design (all components, both themes, all states) **before** UI code begins. This file seeds that work and records the locked tokens.

## Principles

- **Bauhaus, not generic.** Bold geometric type, primary inks, hard edges, strong asymmetric grid, flat (no gradients/shadows/glow). Earn every element; restraint over decoration.
- **Calm, never anxious.** No gamification, no alarm-red shaming. A drift into Reddit is shown matter-of-factly.
- **The dial is the signature.** One memorable idea executed precisely. Don't dilute it with competing flourishes.
- **Light and dark are both first-class** (token-based). Dark Bauhaus is *designed*, not auto-inverted.
- Banned (AI tells): purple-on-white gradients, Inter/Roboto/system-font defaults, the "card with a left accent border."

## Color tokens

Light ("paper"):
- `--paper #EEEBE1` · `--card #F6F3EA` · `--ink #161616` · `--muted #6A6A66` · `--rule rgba(22,22,22,.16)`

Primaries (the inks — encode meaning, used for contexts + accents):
- `--blue #1B45BE` · `--red #E0241B` · `--yellow #F2BC0C` · `--ink #161616`

Context → color (the dial/legend/ledger):
- Deep work → blue `#1B45BE` · Research → red `#E0241B` · Comms → yellow `#F2BC0C` · Breaks → ink `#161616`
- Idle → faint hollow (track gray `#D7D3C7`) · "now" marker → ink (or red on the week minis)

Dark (to be finalized in Claude Design — direction only): near-black paper, ink→bone text, the three primaries stay saturated and loud on the dark field; the parchment warmth inverts to a warm-neutral dark, not pure black.

## Type

- **Display / headlines / big figures:** Anton (poster grotesque, all-caps display).
- **Body / labels / UI:** Jost (geometric humanist, the Futura-adjacent workhorse).
- Never Inter / Roboto / system default. No Space Grotesk.
- Numerals in dials/stats: Anton for hero figures; Jost for inline data.
- Sentence case for prose; the poster headlines and small eyebrow labels may use caps as a deliberate Bauhaus device.

## Shape & motion

- Hard edges. Rounded corners only where functional and full-bordered; no single-side accent borders.
- Borders: 2–3px solid ink for structure (this boldness is intentional, not the thin hairlines of generic UI).
- Flat fills only — no gradients, drop shadows, blur, or neon.
- Motion: one orchestrated load (dial arcs draw in with stagger, now-hand drops, week minis cascade). Hover: arc lifts + others dim; legend isolate. Purposeful, not decorative.

## Components (to design in Claude Design)

Dial (24h, fixed, midnight top, arcs + hour ticks at 0/6/12/18 + now pointer + center figure) · mini-dial (week) · recap card (with "summarized on-device" tag) · stat tiles · legend chips (click-to-isolate) · ledger rows (bar + mono figures, hover-highlight) · linear timeline strip · top nav / Day·Week·Settings tabs · date navigator · toggles · inputs · menubar item · empty state · first-run + permission-priming screens · settings rows · context/rules editor · detail inspector.

## States to cover (no drift)

Each component, in: light + dark; loading / empty (no data yet) / partial-day / full-day; permission-granted vs degraded; AI-on (prose recap) vs AI-off (template recap); private/excluded entries (no title); hover / focus / active.

## Reference

Working prototypes that established this language: `prototype/index.html` (app) and `prototype/landing.html` (marketing). The Claude Design build supersedes them as the source of truth.
