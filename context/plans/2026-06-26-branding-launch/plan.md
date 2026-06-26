# UsageOS — Branding & Landing (Phase 5 launch front)

**Created:** 2026-06-26 · **Status:** 🟢 M1–M4 built + pushed as [PR #21](https://github.com/f-gozie/usage-os/pull/21) (branch `phase5/identity`); **un-reviewed** — next = `/usageos-review` (with `--features perf`), then M5 (notarized DMG + Cloudflare deploy). Newest handoff: `handoffs/2026-06-26-01-phase5-branding-pushed-review-next.md`. · **Owner:** Favour
**Parent:** the Phase-5 launch of [`2026-06-22-product-redesign`](../2026-06-22-product-redesign/plan.md) — spun into its own plan because it spans sessions and ships as several PRs.

> The app is hardened (Phase 6 merged). This plan gives it a face: a finalized logo, that logo placed everywhere it belongs, a real landing page, and a polished open-source front door — then a notarized download. Everything is built on the frozen Bauhaus design system (`context/design-system.md`) and its token set (`src/styles/tokens.css`), not invented fresh.

---

## Goal

Take **Contexts** — the day-dial mark (a faint idle track + three category arcs: Work blue, Browsing red, Messaging gold, with a midnight gap at top) — and make it the identity. The mark *is* the product, so the brand and the app are literally the same shape. Then carry it across every surface honestly: no fake chrome, no slogans, no AI-tells. Plain, real copy throughout.

---

## Locked decisions (from the 2026-06-26 interview)

| # | Decision | Choice | Note |
|---|---|---|---|
| 1 | **Primary mark** | **Contexts** — track + 3 category arcs, midnight gap | Chosen over 11 other directions. See `design/logo/contexts.html`. |
| 2 | **Wordmark** | **Dial-O**, uppercase Anton — the Contexts dial *is* the O in `USAGE◔S`. **Stands alone — never paired with a separate dial icon** (two dials side-by-side reads as redundant). The standalone Contexts mark is for **icon-only** spots (app icon, favicon, tray, social avatar). | Mark-beside-word only where the O is too small to read as a dial. |
| 3 | **Name form** | **USAGEOS** (poster caps) in the logo + headings; **UsageOS** (one word, never “Usage OS”) in prose. | Domain `usageos.app`. Technical ids stay as-is and are never user-facing: repo `usage_os`, crate/binary `usage-os`, bundle `com.favour.usage-os`. `productName` = `UsageOS`, so the release `.app` shows “UsageOS” in the dock (the kebab “usage-os” only appears under `tauri dev`, which runs the raw binary). |
| 4 | **Mono fallback** | Same arcs, gaps preserved, one ink — for ≤24px (menubar, favicon, etch) | The honest answer to "a color logo can't carry color at 16px." |
| 5 | **Brand color** | **Monochrome base (ink + paper), blue as the only chrome accent.** The 3 category colors (blue/red/gold) live **inside the dial/swatches only** — the dial is the page's one real splash of color. Blue accents = buttons, links, a key underline. **No green, no large color blocks** (the loud-blue version was rejected 2026-06-26). | The page should feel like the app: calm bone-and-ink, the dial pops. |
| 6 | **Brand surface** | **Light-led, dark-ready** — design every surface in paper + a dark from day one | Paper is the default face. |
| 7 | **Positioning** | Lead with **screen-time, honestly** — concrete, relatable, what people search | Not "where your day went" as the headline (that's the soul, not the hook). |
| 8 | **Landing stack** | **Astro**, static output, on **Cloudflare Pages** | Zero-JS by default; dial + theme switcher drop in as a tiny island reusing our pure SVG/CSS. |
| 9 | **Landing** | **Product** framing — lead with a framed real app window (Day view: dial + recap + ledger), then Week + Timeline crops. Chosen 2026-06-26. | `design/landing-product.html` = M2 blueprint. Calm / Monument / Bold / template passes explored + set aside. |
| 10 | **Demo data** | **Mixed** — real app names (Chrome, Slack, VS Code) in screenshots; generic category labels in the hero dial | Real names feel honest and relatable; no problem using them. |
| 11 | **Download CTA** | **Ship a working notarized DMG** | Apple Developer account in hand (Nudge already shipped). UsageOS is Developer-ID + DMG, **not** Mac App Store (sandbox forbids AX + Automation). |
| 12 | **In-app brand** | **Light touch** — wordmark at the edges (titlebar, tray, onboarding, About, recap-card footer); never competing with the dial inside working views | |
| 13 | **Tray icon** | **Live now-hand** — static mono mark whose now-triangle points at the real current time. A clock, not a stats HUD (honors vision §menubar). | |
| 14 | **Splash** | **No blocking splash.** Motion lives in 3 honest places: onboarding's opening beat (first-run only), the real day-load draw (already in the app — it *is* the loader), and the landing hero. **Chosen motion: H · Wordmark builds** (the wordmark settles, the dial draws in as the O). | A fast app must not fake a loading screen. |
| 15 | **Motion tech** | **Pure CSS/SVG in the app** (dep-free, free reduced-motion). Landing starts CSS-only; add **GSAP + ScrollTrigger** only if we design scroll-choreographed reveals. | No speculative deps. |
| 16 | **Scope (this plan)** | Wordmark + icon set · Landing · In-app placement · README/OSS face · (notarized DMG to wire the download) | Auto-update + Homebrew cask + Sponsor link = rest of Phase 5, later. |
| 17 | **Sequencing** | **Identity → Landing → In-app → DMG** | Everything depends on the frozen identity. |
| 18 | **Validation** | **Dogfood in the real app** — wire mark/tray/icon into your build and live with it a few days before pouring it everywhere. | `/debate` reserved for any genuinely contested call. |

---

## Still open / resolving

- [x] **Dial-O numbers — FROZEN (2026-06-26):** diameter **1.02em** · vertical nudge **0.08em** (down) · ring weight **17** (of 100 viewBox) · side space **−0.05em**. These are the wordmark spec; the masters use them.
- [x] **Landing direction — Product (framed app window), chosen 2026-06-26.** Pitch on top, the real Day-view window (dial + recap + stats + ledger) as the hero, then Week + Timeline crops, privacy, numbers, final. `design/landing-product.html` is the M2 blueprint. Supersedes the earlier hero-A sketch; Calm / Monument / Bold / template passes explored and set aside.
- [x] **App-icon tile — bone.** Contexts color arcs on the bone squircle (`#EEEBE1`).
- [x] **Splash motion — H · Wordmark builds (chosen 2026-06-26).** `USAGE_S` settles in, then the dial draws into the gap as the O — the name finishing itself. Refine exact timing/stagger when building onboarding's opening beat (M3). See `design/logo/splash-2.html`.
- [ ] **Tray now-hand** — confirm live-clock vs static once we see it in the real menubar.
- [ ] **Headline copy** — tighten the exact screen-time line (layout A locked; words TBD).

---

## The mark system (reference — built in `design/logo/`)

- **`contexts.html`** — the mark on every surface (app icon, dock, menubar, favicon, landing, titlebar, DMG, notification, OG, README, lockups).
- **`wordmark.html`** — the dial-O lab with live sliders to nail the O's optical alignment.
- **`splash.html`** — three loader motions (A Sweep & ignite · B Orbit & assemble · C Spin & spread).

Four mark states, one system:
1. **Primary** — color arcs on light. The default.
2. **On-ink** — same color, holds on dark.
3. **Mono template** — one ink, gaps preserved, for ≤24px and single-color contexts.
4. **+ Now-hand** — the midnight-gap triangle; the basis for the live tray icon.

---

## Per-surface plan (current → target)

Grounded in the real files (see the surface inventory in this session's history).

### Identity foundation (M1)
| Surface | File(s) | Current | Target |
|---|---|---|---|
| **SVG masters** | `design/logo/` → new `assets/` | mockups only | Clean, optimized SVG masters: mark (color/on-ink/mono/now-hand), dial-O wordmark, mark-beside lockup. The single source the rest pulls from. |
| **App icon** | `src-tauri/icons/*` + `tauri.conf.json` | geometric dial-ish, no master tracked | New **1024 master** (Contexts on bone) → regenerate the full `.icns`/`.ico`/`.png` set via the Tauri icon pipeline. Commit the master. **Frozen spec (2026-06-26): arc weight 9, padding 15, gap 18°, bone tile, track off, no now-hand (static).** Favicon/tray switch to the mono template below ~24px; tray carries the live now-hand. See `design/logo/icon.html`. |
| **Favicon** | `index.html` → `/vite.svg` | Vite placeholder | Custom `favicon.svg` (color) + `.ico` (mono 16/32). Update `<link>` + `<title>`. |
| **Tray icon** | `src-tauri/src/lib.rs` `setup_tray()` | reuses `default_window_icon()` | Dedicated **mono template** tray image; **now-hand** updated at runtime to the current time (Tauri `set_icon`, recomputed on a coarse timer / on focus, not a busy loop). Light/dark via template rendering. |
| **Titlebar wordmark** | `src/components/shell/TitleBar.tsx` | text `UsageOS` | Dial-O wordmark (small mark + `USAGE◔S`), tracking dot unchanged. |
| **Glance popover** | `src/components/glance/Glance.tsx` | text `USAGE`<red O>`S` | Same dial-O wordmark, compact. |

### Landing (M2) — Astro on Cloudflare Pages
- Replace the old cyberpunk `landing/` (Outfit + JetBrains Mono + cyan/purple neon + ◈ — all off-brand). Evolve `design/landing.html`'s structure, **re-led with screen-time copy**.
- Sections: **hero** (live dial + screen-time headline + working Download CTA) → **marquee** (on your Mac · no cloud · no telemetry · open source) → **how it works** (dial / recap / two-axis) → **week** strip → **the numbers** (100% on-device · 0 servers · <1% CPU · MIT) → **privacy** ("nothing leaves this machine") → **final CTA** → footer.
- Reuse `tokens.css` (framework-agnostic) for paper/warm/black. Theme switcher in nav.
- **OG / social card** (1200×630) + favicon + meta. OG is for when you post `usageos.app` on X.
- Deploy: Cloudflare Pages, custom domain `usageos.app`.

### In-app placement (M3) — light touch
- **Onboarding** (`src/components/onboarding/Onboarding.tsx`): the dial-draw becomes the **Welcome** screen's opening beat (first-run only, pure CSS). Keep the dial-O wordmark in the step header.
- **Day-load motion**: confirm the existing arc draw-in is the load state (it is) — no separate splash.
- **About**: there is **no About dialog today** → add a small one (mark + version + "private, on-device" + links: site, GitHub, license, sponsor). Reachable from Settings.
- **Recap card** (`src/components/ui/RecapCard.tsx`) + **empty states**: a quiet wordmark/mark where it earns its place — not on every view.

### README / OSS face (M4)
- Add the **banner** (mark + dial-O wordmark, on paper, dark-variant ready) at the top of `README.md`.
- Real **screenshots** (Day / Week / Timeline) + badges (CI, MIT, macOS, "100% local").
- Set the **GitHub social preview** image (the OG card).

### Notarized DMG (M5 — wires the download)
- Developer-ID sign + **notarize** + staple; build the **DMG layout** (app icon + Applications alias + the drag arrow + branded background — see `contexts.html` DMG mock).
- Wire the landing Download CTA to the release asset.
- *Deferred to rest of Phase 5:* auto-update (Tauri updater/Sparkle), Homebrew cask, Sponsor link.

---

## Copy direction

Voice: plain, honest, a little warm. No slogans, no lines that sound deep, no jargon, no AI-tells. Sentence case in prose; curly quotes. (Holds the `design-system.md` Copy bar + the copy-voice / tweet-voice memories.)

**Headline candidates (screen-time-led — pick one in M2):**
- "See where your screen time actually goes."
- "Your screen time, in plain sight."
- "Honest screen time for your Mac."

Supporting line (privacy, always near the hero): *"It all stays on your Mac. Nothing leaves the machine."*

Throughout: real app names where they help (screenshots), generic category words in the abstract hero dial.

---

## Technical notes

- **Fonts:** Anton + Jost, **bundled locally** (`@fontsource`) — no CDN, even on the landing (hard rule 1: nothing leaves the machine; for the site it's a perf/independence choice, and keeps parity with the app).
- **Tray live icon:** render the mono mark + now-hand to a small image; update via `tray.set_icon()` on a low-frequency tick (e.g. each minute) or on focus — never a tight timer (idle-CPU discipline). Template image so macOS handles light/dark menu bars.
- **Icon pipeline:** keep a tracked `icon-master.png` (1024) + regenerate; document the command so it's reproducible.
- **Astro + Cloudflare:** static output; the dial/theme island is vanilla SVG+CSS (or a tiny script) — likely **zero React** needed. GSAP only if a scroll sequence demands it.
- **Reduced motion:** every animation (app + landing) honors `prefers-reduced-motion`.

---

## Risks & watch-items

- **Color logo at small sizes** — solved by the mono template, but verify the favicon + tray at 16px on a real screen during M1 dogfood.
- **Tray now-hand vs "not a HUD"** — a current-time clock is defensible; if it reads as a HUD on-device, fall back to static mark + draw-in.
- **Notarization lead time** — first notarization + Developer-ID setup can have snags; start M5 prep early even though it sequences last.
- **Don't over-brand the app** — light touch is a constraint, not a suggestion; the dial stays the hero.
- **No unverified claims on the page.** The `<1% CPU` stat was cut — idle CPU isn't rigorously profiled for the *shipping app* yet (Phase-6 tail; the Phase-0 spike showed 0.0% for the *capture path* only). Measure it before any number goes up; until then claim the mechanism (event-driven, no polling), not a figure. The brand is "you can audit this" — an unverified number would undercut it.

---

## ADRs to append to `context/decisions.md` (D59+)

Append as each locks (append-only, don't relitigate):
- **Brand mark = Contexts**; wordmark = dial-O; name form = USAGEOS caps.
- Tri-color mark + single blue accent; light-led / dark-ready.
- Landing = Astro on Cloudflare; screen-time-led positioning.
- Tray = live now-hand; no blocking splash (motion in onboarding + natural load + landing).

---

## Milestones / sequencing

- [~] **M1 — Identity foundation** *(branch `phase5/identity`, code-complete; pending dogfood + review)*:
  - [x] SVG masters → `design/logo/assets/{mark,mark-mono}.svg` + `src-tauri/icons/source/app-icon.svg`.
  - [x] Dial-O `Wordmark` component (`src/components/ui/Wordmark.tsx`, frozen geometry). **Dogfood correction (2026-06-26):** the dial-O muddies / reads skinny at ≤16px, so it lives in the **big app header** (`AppShell`, 36px, where it reads); the **titlebar / glance / onboarding** were reverted to plain readable text. Confirms the rule: dial-O only where it's large; the standalone mark covers icon-only spots.
  - [x] Real **favicon** (`public/favicon.svg`) wired into `index.html`.
  - [x] **App-icon set** regenerated from the master via `tauri icon` (bone tile + Contexts mark); iOS/Android artifacts removed. Reads at 32px.
  - [x] **Tray** mono-template + **live now-hand**: `src-tauri/src/tray_icon.rs` embeds 24 hourly template frames (`design/logo/assets/gen-tray-icons.mjs`), picked by local hour (libc `localtime_r`), refreshed on a 10-min background thread (no busy timer); `setup_tray` sets `icon_as_template(true)`. Added the `image-png` tauri feature.
  - Gates **all green**: `npm run build` (tsc+vite) + 32 TS tests · `cargo check` + `clippy -D warnings` + `cargo fmt --check` + 127 Rust tests + `cargo deny check`. **Next: dogfood (full `tauri dev` restart for the native icon/tray), then `/usageos-review` → commit/PR.**
- [~] **M2 — Landing** *(on the one Phase-5 branch)*: ✅ Astro static site in `landing/` (Product direction — pitch + framed app window, week + timeline crops, privacy, numbers; screen-time copy; live dial + week built client-side; theme switcher; **fonts bundled locally**). ✅ favicon + **OG card** (`public/og.png`) + meta. ✅ Builds clean (`astro build`); previews at `localhost:4321`. ✅ Deploy notes (`landing/README.md`). Replaced the old cyberpunk `landing/`. **Remaining:** wire the real DMG download link (M5); Cloudflare Pages deploy to `usageos.app`; optional — embed Anton in the OG, add a "what it can/can't see" trust block + download specifics.
- [~] **M3 — In-app placement** *(code-complete; pending dogfood)*:
  - [x] **Onboarding Welcome opening beat** — `BuildingWordmark` in `Onboarding.tsx` (USAGE_S rises in → the dial draws into the gap as the O → copy follows), CSS-driven (`index.css .ob-*`), reduced-motion-safe, approved timing (letters 600 · dial 850 · hold 250). First-run only.
  - [x] **Settings → About** — `src/components/settings/AboutModal.tsx` (dial-O wordmark + version + privacy line + Website/GitHub/License/Sponsor links via the opener + credit); wired into `SettingsView`; added `opener:allow-open-url` to the capability. Settings test opener mock updated.
  - [ ] Recap-card / empty-state brand touches — deferred (optional, low value).
  - Gates green: `npm run build` + 32 TS tests + `cargo check` (capability valid).
- [x] **M4 — README / OSS face**: ✅ **fresh README** (`README.md`) — plain, honest, grounded in the current app (banner · badges · what-it-shows · privacy · permissions · install/build-from-source · develop · how-it-works · docs), not the old "in transition" framing. ✅ **banner** (`docs/banner.png`). ✅ added the missing root **LICENSE** (MIT — confirmed the right license with the owner). ✅ **real screenshots** (`docs/screenshots/{day,week,timeline}.png` + onboarding welcome/privacy/permission) captured from the running app and wired into the README. To get a full, clean week I added a **demo-data generator** to the seeder (`perf.rs` `generate_demo_day` + `seed_db --demo` + `--end`): long single-category blocks (clean arcs), this-week dates, **eyemark removed** from the catalog. Captured against a throwaway seeded DB, then **restored the real DB exactly** (backup at `/tmp/usageos-realbak`). **Remaining (owner, ~1 click):** set the GitHub **social preview** to `landing/public/og.png` (repo Settings → Social preview).
- [ ] **M5 — Notarized DMG**: sign + notarize + DMG layout; wire the download. *(auto-update / Homebrew / Sponsor → later Phase 5.)*

**All of Phase 5 ships as one branch/PR** (`phase5/identity`) — the milestones are work-order, not separate PRs (owner's call 2026-06-26). One `/usageos-review` + reconcile (this plan + decisions.md + a handoff) before the PR (Definition of Done).

---

## Immediate next actions

1. **You:** tune `design/logo/wordmark.html` → give me the four numbers; pick a splash motion (A/B/C); react to app-icon tile in the dock.
2. **Me:** mock 2–3 **landing hero** directions (with screen-time copy) for you to choose; then freeze the dial-O spec and start M1.
