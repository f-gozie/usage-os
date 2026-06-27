# usageos.app — landing site

The marketing site for UsageOS. Static [Astro](https://astro.build), deployed to **Cloudflare Pages**.
Fonts (Anton/Jost — both OFL-1.1) are **self-hosted** in `public/fonts/` (with their licenses) and
preloaded — no CDN, so no visitor IP leaks to a third party, consistent with the product's promise.
The dial + theme switcher are tiny client scripts; the rest is pre-rendered HTML. Design source:
`../design/landing-product.html` (the Product direction).

## Develop

```bash
cd landing
npm install
npm run dev      # http://localhost:4321
npm run build    # → dist/  (static)
npm run preview  # serve the built dist/
```

## Deploy (Cloudflare Pages)

Git-integrated auto-deploy: push to the production branch → Cloudflare builds and deploys;
other branches/PRs get preview URLs. In the Pages project settings:

- **Production branch:** `main`
- **Root directory:** `landing`
- **Build command:** `npm ci && npm run build`
- **Build output directory:** `dist`
- **Node version:** `22` (pinned via `landing/.nvmrc`; or set `NODE_VERSION=22`)

**Custom domain:** `usageos.app`. The zone is already on Cloudflare (nameservers
`raegan`/`lakas.ns.cloudflare.com`), so Pages → Custom domains → add `usageos.app`
auto-creates the record and provisions TLS — no nameserver change needed.

## Notes

- The brand surface is monochrome (ink + paper) with **blue as the only accent**; the three category
  colours live **inside the dial** only. No green. See `../context/design-system.md`.
- `public/og.png` is the social card (1200×630). Regenerate from `../design/` if the brand changes.
- The Download CTA points to the notarized DMG once it ships (Phase 5 tail).
