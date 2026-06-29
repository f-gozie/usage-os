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
