# usageos.app — landing site

The marketing site for UsageOS. Static [Astro](https://astro.build), deployed to **Cloudflare Pages**.
Fonts (Anton/Jost) are bundled locally; the dial + theme switcher are tiny client scripts. The rest
is pre-rendered HTML. Design source: `../design/landing-product.html` (the Product direction).

## Develop

```bash
cd landing
npm install
npm run dev      # http://localhost:4321
npm run build    # → dist/  (static)
npm run preview  # serve the built dist/
```

## Deploy (Cloudflare Pages)

Connect the repo, then set:

- **Build command:** `npm install && npm run build`
- **Build output directory:** `dist`
- **Root directory:** `landing`
- **Node version:** 20+

Custom domain: `usageos.app`.

## Notes

- The brand surface is monochrome (ink + paper) with **blue as the only accent**; the three category
  colours live **inside the dial** only. No green. See `../context/design-system.md`.
- `public/og.png` is the social card (1200×630). Regenerate from `../design/` if the brand changes.
- The Download CTA points to the notarized DMG once it ships (Phase 5 tail).
