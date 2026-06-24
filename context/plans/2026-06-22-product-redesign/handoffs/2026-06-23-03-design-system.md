# Handoff — Bauhaus design system built & frozen (design session)

_Written 2026-06-23 at the end of the **design session** (after [`2026-06-23-02-process-and-freshness`](2026-06-23-02-process-and-freshness.md)). Read `CLAUDE.md` first, then this. **No product/Rust/TS code changed this session** — this was the parallel design-system track (the critical-path blocker the prior handoffs flagged). It is now **done**: the full Bauhaus system is designed end-to-end as interactive HTML, the spec is frozen, and the open product question it surfaced (sessionization) is decided in shape._

## 1. Where we are (TL;DR)

UsageOS (private on-device Mac time-tracker; Tauri + Rust + React; Bauhaus redesign). Backend capture pipeline ✅ on-device-verified (prior handoffs). **This session delivered the design system** — the thing that was gating all remaining Phase-1 UI. It now exists as **9 interactive, themeable HTML pages in `design/`** (open `design/index.html`), and `context/design-system.md` is **rewritten/frozen** as the machine-readable contract. **R77 resolved**, **D34 (segmentation) decided**, **copy pass done**. `main` unchanged; **everything this session is UNSTAGED** (see §4). Next real work: **port the mockups into Bauhaus React components** (Phase 1.4 — the dial), then the Claude Design push.

## 2. What this session produced (design + docs only)

- **`design/` — the interactive HTML design library (the living visual source of truth):**
  - `index.html` (gallery hub) · `foundations.html` · `day.html` · `week.html` · `timeline.html` · `settings.html` · `components.html` (controls/inputs/modals/states) · `onboarding.html` · `landing.html` · `timeline-variants.html` (the A/B/C exploration behind D34).
  - Self-contained (Google Fonts via CDN, like `prototype/*.html`), all with a live **Paper/Warm/Black** theme switcher, all interactive. **`prototype/` is the historical seed; `design/` supersedes it.**
- **`context/design-system.md` — FROZEN & rewritten** as the contract: exact token tables for all 3 themes, the R77 resolution, type/spacing/motion, the full component inventory, states-to-cover, the data-representation rules, and copy/voice.
- **`context/decisions.md` — appended D34** (segmentation model).
- **`context/plans/2026-06-22-product-redesign/explorations/2026-06-23-sessionization.md` — NEW** (the full "what is a session" framing + the dogfood plan).
- **`plan.md`** — updated: design-system track status, Phase-4 data controls (Export CSV + Delete-all + retention picker — design-surfaced), day-start offset tagged deferred (D14), and Timeline = context-runs.
- **Memories written** (`~/.claude/.../memory/`): `design-session-workflow` (deliver HTML, not `show_widget`) and `copy-voice-matters`.

## 3. Decisions locked this session (don't relitigate)

- **R77 → Option A** (the contrast blocker is closed): every dial arc gets a **1px ink casing** (`--casing`, light only — dark arcs already clear 3:1), Comms tuned to gold **`#EAB308`** in `paper` (stays loud `#F2BC0C`/`#F5C518` in dark), and **colour is never the only cue**.
- **Three themes**: `paper` + `warm` (warm charcoal) + `black` (near-black). **Canonical dark is TBD but irrelevant** — both ship as user-selectable for ~zero cost (pure token sets). Inverted bars use **`--bar-bg`/`--bar-fg`** (not `--edge`/`--bg`, which go dark-on-dark in dark themes).
- **D34 — segmentation:** dial/ledger/week are **per-axis aggregates** (robust); the **Timeline + dial arcs render context-runs** (continuous same-context stretches) with the **project split shown as labelled durations inside the block**; **off-project work (Slack/browsing — no project signal) counts to its context**, shown "no project" (we never guess a project — D30). **Thresholds (excursion-absorb / idle-gap / sustained-shift) are OPEN → D34a, resolved by dogfooding real data**, not theory.
- **Two principles we learned the hard way** (now in `design-system.md`): **(1) colour encodes context only — projects are identified by NAME, never colour** (open-ended set); **(2) never put text inside a proportional segment** (it clips on narrow/long/many — label outside, in a line that can wrap).
- **Copy voice**: plain, honest, accurate, human. **No slogans, no jargon (no "Screen Recording"), no insider lingo ("real, not inferred"), no niche-tech examples (`db.rs`)**. Sentence case; curly quotes.

## 4. Gotchas / state (read before continuing)

- **EVERYTHING IS UNSTAGED.** `git status` will show the whole `design/` folder (untracked) + `context/design-system.md`, `decisions.md`, `plan.md`, the new exploration dir — all modified/untracked. **Nothing was committed** (the user didn't ask). **First action next session: branch + commit the design work + docs.**
- **Claude Design push is BLOCKED — twofold, and it's NOT a quick fix:**
  1. **Auth:** `DesignSync` needs design-system authorization that **this CLI session doesn't have**; **`/design-login` is unavailable in this environment** (the user confirmed). The push needs an **interactive session where `/login` (Claude subscription) works**.
  2. **No artifact to push:** the `/design-sync` skill converts a **compiled React component library** (Storybook or package shape + `dist/`, exposed as `window.<name>.*`). We have **HTML mockups + a spec**, and `src/components/` holds only the **old cyberpunk** components. So `/design-sync` is correct but **premature — it runs AFTER the React port** (build the real Bauhaus components first, ideally with a small Storybook for per-component verification, then sync).
- **`show_widget` is useless to the user** — they can't interact with it in their app. Always deliver real HTML files. (Saved as a memory.)
- **Dial changes from the prototype:** the vestigial **centre hub dot was removed**; arcs are **context-runs** (one blue arc per deep stretch, not per-app slivers) and clicking an arc opens the **session** in the inspector.
- **The project-split bar saga:** tried it 3 ways (two-greys → labelled-inside → confusing) and **dropped it** — the timeline shows the split as a plain `usageos 1h 3m · nudge 39m` line. Don't reintroduce a bar.
- The data in the mockups is a **realistic interleaved day** (usageos ↔ nudge bouncing); `day.html` + `timeline.html` share it.

## 5. Next steps (next session)

1. **Commit** the design work + docs (branch first).
2. **Build the Bauhaus React components** from `design/` + `design-system.md` — **the dial first** (Phase 1.4, D3 — the soul), against the generated tauri-specta client. Then Day → Week → Timeline → Settings. This replaces the old cyberpunk `src/components/*` and lets `recharts` finally be removed (plan §Phase-0).
3. *(recommended)* add a **lightweight Storybook** for the new components — unlocks `/design-sync`'s per-component screenshot verification.
4. **Then `/design-sync`** from an interactive session (where `/login` works) to push the real components to Claude Design.
5. **Dogfood D34a** — run capture for a few real days, then build a "session explorer" to replay the raw events under candidate thresholds and tune to lived memory; lock the thresholds. (Parallel to the build.)
6. **Wire Settings backends:** Export CSV + Delete-all (new IPC commands), retention picker, exclusions/per-app Private UI (schema done), contexts/rules editor (commands exist).
7. **Design the menubar popover** — the one component not yet mocked (flagged in `design-system.md`).
8. **Apple Developer enrollment** — off critical path; gates Phases 3/5 + signing/TCC; start early.

**Read order:** `CLAUDE.md` → `context/plans/README.md` → this plan's `plan.md` + this handoff → **open `design/index.html` in a browser** → `context/design-system.md` → `context/decisions.md` (D1–D34) → the sessionization exploration.
