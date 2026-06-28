# Handoff — 2026-06-28-02 · De-Mac the brand positioning

**Branch:** `claude/mac-branding-review-xzbzi5` · **Decision:** [D60](../../../decisions.md) · **Plan:** branding-launch (Phase 5)

## Why

Owner: _"I think we're over-branding ourselves as a Mac tool. Mac is just the first OS. Potentially [other platforms / other-language ports] later. Multiple references — landing page, assets, twitter, etc."_

So: **de-brand the identity away from Mac, keep concrete availability honest.** Mac is the *first* platform we ship on, not what the product *is*. No architecture/scope change — v1 still ships macOS-only.

## What changed (copy only)

Identity one-liners → **"a private, on-device time tracker"**; possessive **"your Mac" → "your machine/computer/device"**.

- **Landing** (`landing/src/pages/index.astro`): meta description, hero eyebrow + sub, trust strip ("On your device"), privacy lead ("one file on your machine"), "reveal it on disk" (was "in Finder").
- **README.md**: intro reworded; now states **"macOS is the first platform it runs on"**; "your Mac" → "your machine" in What-it-shows + Privacy.
- **In-app**: `Onboarding.tsx` (eyebrow + "Your computer already knows…" + privacy + accessibility why-text), `AboutModal.tsx`, `SettingsView.tsx` (data-location desc), `CategoryEditorModal.tsx` (app-picker desc).
- **Metadata**: `src-tauri/Cargo.toml` description (dropped "Mac app").
- **Contract docs**: `CLAUDE.md` product one-liner + `context/vision.md` thesis ("Your computer already knows…") and who-it's-for, each with a D60 pointer. **Scope statements untouched** ("macOS-first", "No Windows/Linux in v1", "Out of scope (v1): Windows/Linux").

## What deliberately stayed Mac (current truth, not branding)

- **"Download for Mac" CTAs** (landing hero + footer) — only a Mac build exists; generic copy would mislead non-Mac visitors.
- README **platform-macOS badge** + **Requirements: macOS** + **Permissions** section (Accessibility/Automation are real macOS permissions).
- In-app **"Show in Finder"** action (the actual OS affordance).
- All **technical** macOS refs (Foundation Models, objc2, Accessibility/Automation APIs, notarized DMG, CI).

## Gates

`tsc --noEmit` clean (only a pre-existing `baseUrl` deprecation warning) · **32/32 TS tests** pass · **`astro build`** clean. No Rust logic changed (only the Cargo description string) — Rust suite not re-run as nothing in the build path changed. No IPC/bindings change.

## Next steps / open

- Not yet committed/pushed at time of writing — push to `claude/mac-branding-review-xzbzi5`.
- Optional: a `/usageos-review` pass before any PR (DoD recommends it).
- **Assets still to revisit** (out of scope for this copy pass — flagged for owner): the **`og.png` social card** and **`docs/banner.png`** weren't inspected for "Mac" wording/imagery; the **GitHub repo "About"/description** and **Twitter/social bios** live outside the repo (owner-owned). `design/*.html` mockups still say "Download for Mac" — left as-is (internal design refs, not shipped).
- No platform-expansion promise was added anywhere — other platforms remain a "potentially later," matching the owner's tentative framing.
