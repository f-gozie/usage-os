# Design

UsageOS is designed **by eye, in the browser**. Every screen is first built as a plain
HTML/SVG/CSS mockup here — no framework, openable directly in a browser — so layout, type,
colour, and motion can be tuned against the real [design tokens](../context/design-system.md)
before any of it is implemented in the React app. Many mockups ship with live sliders to nail a
value by eye. The design system itself is the contract; these files are where it gets pressure-tested.

## Shipped-UI mockups (the current references)

`day.html` · `week.html` · `timeline.html` · `settings.html` · `settings-categories.html` ·
`onboarding.html` · `menubar.html` · `components.html` · `foundations.html` — the in-app views
the shipped UI is built from. `timeline-variants.html` / `timeline-icons.html` /
`category-palette-5.html` are the option-explorations behind specific choices.

## Built from these boards

**`calendar.html`** — the Year → Month → Day drill-down. The live board: one `Calendar` tab with
Year and Month as zoom rungs, a breadcrumb as the navigation model, and an explicit
“not recorded” state at both scales. **`month.html`** is the earlier Month-only round; it's kept
for the three-way comparison of cell treatments (mini-dial vs stacked bar vs heat block) and the
reason heat blocks were ruled out. **Both are now built** (D73) — the shipped Calendar follows these boards, with two differences
worth recording: the app is **Sunday-first** (matching `weekDays()`; the boards drew Monday-first,
and the app wins), and both Year layouts ship behind a user-facing switch rather than one being
chosen.

Two things these boards measured that are worth keeping: an **80px dial** is the smallest size at
which R77's 1px ink casing is still a real pixel (the casing is 1.25% of rendered size), and at
80px the month cells come out 126×130 inside a 984px frame — so the compliant size fits the
window, at the cost of the month scrolling. Below that, the casing silently stops doing its job.

> **Palette drift:** most boards here predate D47 and still carry the four-category palette —
> only `category-palette-5.html`, `menubar.html`, `month.html` and `calendar.html` have
> `--c-personal`. Take your token preamble from one of those, not from the nearest file.

## `logo/` — the mark system

The **Contexts** dial-mark on every surface (`contexts.html`), the dial-O wordmark lab with live
alignment sliders (`wordmark.html`), and the splash-motion studies. See the
[branding plan](../context/plans/2026-06-26-branding-launch/plan.md).

## Landing explorations (superseded)

`landing-*.html` are the directions explored for the landing page — **`landing-product.html`** was
chosen; `landing-calm` / `landing-monument` / `landing-bold` and the others were set aside. The
**shipped** landing is the Astro site in [`../landing/`](../landing/), not these. They're kept for
the record of how the page was found, not as live source.

## `assets/`, `social/`

Exported icons and social/OG imagery generated from the mockups.
