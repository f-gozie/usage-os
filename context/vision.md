# UsageOS — Vision & Product Spec

_Last updated: 2026-06-29. Evolves the shipped v0.1.0 + its Tier-1 OSS-hygiene foundation (tests, CI, migrations, retention) — kept, not rebuilt. Supersedes the original gamification-era specs (now archived in [`context/history/`](history/): `00-PRD-MVP.md`, `01-TECH-CONTEXT.md`, `02-FUTURE-IDEAS.md`), kept for the pivot story._

## Thesis

> Your computer already knows how you spent today. UsageOS tells you — privately.

_(Positioning is platform-agnostic — **macOS is the first platform UsageOS ships on, not the brand**; D60. The v1 build remains macOS-only — see "Distribution" and "Out of scope (v1)".)_

Existing trackers (ActivityWatch, Screen Time, RescueTime, Rize) give you **raw data** — "VS Code 4h, Chrome 3h" — but never **understanding**: VS Code on *what project*, Chrome *researching what vs. doom-scrolling*. The signal is sitting in window titles and URLs; nobody turns it into meaning. UsageOS closes that gap, on-device.

It is a **calm rear-view mirror, not a coach.** It records quietly and waits for you to look. The act of looking is the ritual. (Its sibling project, Nudge, is the coach; this is deliberately the opposite.)

## Who it's for

Initially the developer (dogfood), then people who (a) spend their day at a computer (macOS first), (b) want to understand their own time, and (c) refuse to hand that data to a cloud. The privacy-conscious, the quantified-self crowd, developers — but the experience must be calm and legible, not a science project.

## The core experience

1. **The day-dial** — your day as a 24-hour chronometer (midnight at top), activity inked as arcs colored by context, a "now" hand, totals in the center. The signature; works with zero AI. *This is the soul of the product.*
2. **The recap** — a few honest sentences each evening: where you focused, what pulled you away, where the hours actually went. The smart upgrade. A deterministic template recap always exists; the local model upgrades it into prose.
3. **The timeline** — the same event log as a linear strip, for "what exactly was I doing at 2pm." Secondary to the dial.
4. **The week** — seven mini-dials; read the rhythm of your week at a glance (heavy Wednesday, light Sunday). Requires the fixed-24h scale to be comparable.

## The data model: two axes

- **Category** — the *kind* of work: Work, Browsing, Messaging, Entertainment, Personal (editable). Assigned by rules you own (by app name or a word in the window title), refined by your corrections.
- **Project** — the *what*: `usage_os`, `nudge`, etc. Auto-inferred from window titles (`auth.rs — usage_os`), terminal cwd / git repo, GitHub titles — and correctable. (Inference quality is a Phase-0 spike.)

## The smart layer (optional, on-device, never required)

- **Runtime:** Apple Foundation Models (built into macOS 26, free, private) via a thin Swift sidecar. On older Macs / when disabled, the template recap covers it silently.
- **Categorization:** deterministic rules you own — by app name or a word in the window title — and every correction is remembered (fix one rule and the past re-sorts). On-device embeddings were trialled and shelved (D47, below the rules baseline), so categorization stays out of the smart layer entirely.
- **Recap:** computed lazily when you open the app; optionally one gentle "your day is ready" notification at a wind-down time you set (off by default — the single sanctioned interruption). The model only phrases pre-computed facts.
- **Cloud:** none in v1. A bring-your-own-key cloud option is a possible future toggle behind loud warnings — it would break "nothing leaves," so it is opt-in only, never default.

## Privacy model

- 100% local; no network in the data path (auditable in the open source).
- Window titles stored raw **locally**, with a sensible exclusion list (auto-exclude password managers / banking), per-app "Private" marking (counts as time, no title), and incognito windows never recorded.
- Permissions: **Accessibility** (window titles) + **Automation** (browser URLs). Never Screen Recording. Primed first-run onboarding that explains *why*; the app still runs degraded (app-level data) if declined.

## Non-functional requirements

- Negligible CPU when idle (event-driven, not polling); unobtrusive memory.
- The dial must render today's real data quickly on open.
- Light **and** dark from launch (token-based theming).
- Menubar launcher + main window; the menubar icon is an entrance, not a live HUD.

## Distribution & business

- Open source, MIT, public from day one — auditability *is* the privacy proof.
- Notarized DMG + auto-update (Sparkle/Tauri updater) + Homebrew cask. Mac App Store is impossible (sandbox forbids our permissions).
- Free; optional "Sponsor" link. A paid Pro tier stays possible later but is not a v1 concern.

## Naming

Working name **UsageOS** (zero migration; bundle id `com.usageos.app`). Strongest alternative considered: **Daybook** (warmer, less "OS-trope"). Domain: `usageos.app`. Revisit before public launch; cheap to change.

## Out of scope (v1)

Gamification of any kind; real-time nudges; cloud/account/sync/telemetry; Windows/Linux; Mac App Store; natural-language query of your history (later); long-range trends/month view (day + week only in v1).
