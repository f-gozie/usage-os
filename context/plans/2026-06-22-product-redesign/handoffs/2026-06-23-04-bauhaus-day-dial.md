# Handoff — Phase 1.4: the Bauhaus Day dial, end-to-end

_Written 2026-06-23 (after [`2026-06-23-03-design-system.md`](2026-06-23-03-design-system.md)). Read `CLAUDE.md` first, then this. This session **ported the frozen design into a real React UI on real data** — the dial now works end-to-end — and hardened the DB migration system on the way. All work is on branch **`phase1/bauhaus-day`** (committed locally, **not pushed, no PR yet**)._

## 1. Where we are (TL;DR)

UsageOS (private on-device Mac time-tracker; Tauri + Rust + React; Bauhaus redesign). **Phase 1.4 — the fixed-24h Day dial from real data, click-to-inspect — is ✅ DONE.** Backend-first, clean cutover, no mock/stale code. The cyberpunk dashboard is gone. Gates green (69 Rust + 7 TS tests, clippy/fmt, tsc, vite + storybook build, bindings fresh). **Not yet run on-device** — that's the first thing next session.

## 2. What this session produced (branch `phase1/bauhaus-day`, 6 commits on `main`)

1. **`design: …`** — committed the prior design session's output (the `design/` HTML library + `design-system.md` + D34 + sessionization exploration) that was sitting unstaged.
2. **`refactor(db): harden migration runner + squash to a clean baseline`** (D35) — per-file `.sql` migrations under `src-tauri/migrations/`, a top-level `migrations` module that applies each in a **transaction** + verifies a **FNV-1a checksum** on boot (drift guard), **forward-only**. Squashed v1–v8 → `0001_initial_schema.sql` (clean baseline, contexts `slug` inline) + `0002_seed_canonical_contexts.sql`. Fixed a real atomicity bug in the old runner.
3. **`feat(rollup): add get_day read-model`** (D36) — pure `rollup.rs` (per-axis aggregates + **context-runs** + deterministic **template recap**, D34) + the `get_day` command (project names joined server-side); regenerated `bindings.ts`. 7 rollup unit tests.
4. **`feat(db): seed starter categorization rules`** — migration 3 maps common dev apps → the 4 contexts so the dial is colourful out-of-the-box. **Opinionated (browsers→Research) — flagged for your veto; editable in M4.**
5. **`feat(ui): Bauhaus Day view + dial; retire the cyberpunk dashboard`** — the whole frontend: `styles/tokens.css` (frozen 3-theme contract), Tailwind rewired to tokens, **fonts bundled locally via `@fontsource`** (no CDN — hard rule 1), `ThemeProvider`, shell (`AppShell`/`TitleBar`/`TabNav`/`Footer`/`ThemeSwitcher`), UI primitives (`Button`, `SegmentedControl`, `Chip`, `StatTile`, `LedgerRow`, `RecapCard`, `DetailInspector`, `Skeleton`, `DegradedBanner`), the **`Dial`** (SVG, ported from the mockup: context-run arcs + 1px casing, hour ticks, now-triangle, centre figure, hover-dim + tooltip, click→inspector, staggered draw-in honouring reduced-motion) + `lib/geometry.ts`, `useDayData` + **`DayView`** (recap/dial/stats/legend-isolate/inspector/ledger; loading/empty/error/degraded; day prev-next nav). **Cutover** deleted the cyberpunk components + dead `lib/stats`/`lib/time`; removed `recharts`/`lucide-react`/`@radix-ui/react-tabs`/`class-variance-authority`.
6. **`chore(ui): Storybook + RTL component testing`** — Storybook 10 (react-vite, Paper/Warm/Black toolbar, telemetry off) + stories for every primitive and the dial; React Testing Library + jsdom + a `Dial` test; widened vitest `include` to `.tsx`.

Docs reconciled in lockstep: `plan.md` (Phase 1.4 ticked, status updated), `decisions.md` (**D35**, **D36** appended), the impl-plan `impl-plans/2026-06-23-bauhaus-ui-day.md`, and this handoff.

## 3. Decisions locked this session (don't relitigate)

- **D35** — migrations: hand-written per-file SQL, hardened in-house runner (transactions + checksums + forward-only), clean-slate squash. Contexts table keeps the name `categories` (D31 rename still deferred) but gains a `slug` carrying the colour-token identity.
- **D36** — the dial's numbers are a **Rust read-model** (`get_day`); React only renders. CSS-var theming, local fonts, no state lib, Storybook + RTL. Starter rules + cutover scope (Week/Timeline/Settings are placeholders).

## 4. Gotchas / state (read before continuing)

- **Branch `phase1/bauhaus-day` is local only — not pushed, no PR.** 6 commits on top of `main`. Push + open the PR when you're ready (the docs are already reconciled for it).
- **DELETE THE DEV DB before running on-device.** The schema is a clean-slate squash; an existing `usage.db` from earlier capture testing has the old v1–v8 `schema_migrations` (no `checksum` column) and will conflict. It's throwaway capture-test data. (App-data dir → `usage.db`.)
- **Not yet run on-device.** Everything compiles/tests/builds, but `cargo tauri dev` on the Mac hasn't been exercised this session. First action: delete dev DB → run → confirm the dial renders the real captured day, then **eyeball the starter rules** against your real apps and tune.
- **Starter rules are opinionated** (migration 3; browsers→Research is a coarse default). Veto/edit freely — they're just a first-run convenience until the M4 rules editor.
- **Settings/Week/Timeline are "coming soon" placeholders.** The old Settings UI (categories/rules/retention) is **gone** until M4 rebuilds it in Bauhaus. Capture runs on seeded defaults meanwhile.
- **MiniDial not built** (Week, M2). **Degraded banner** is wired to watcher health, not Accessibility-grant detection (needs a new command — M4/onboarding).
- **`/design-sync` still blocked** the same way (needs interactive `/login`); now unblocked *technically* (a Storybook component library exists) — run it from an interactive session after this lands.
- Header shows weekday + date; the mockup's "▸ Now HH:MM" line wasn't added (the dial's now-triangle covers it).

## 5. Next steps (next session)

1. **On-device run** — delete dev DB, `cargo tauri dev`, verify the real-data dial + recap + inspector; tune the starter rules to your apps. (Parallel: this is also the start of **D34a dogfooding**.)
2. **Push `phase1/bauhaus-day` + open the PR.**
3. **M2 — Week:** `get_week` (per-day per-context sums) → `WeekView` (7 **MiniDial**s, shared dial engine).
4. **M3 — Timeline:** `get_timeline` (context-run blocks + idle gaps + expand) → `TimelineView`; build the **session explorer** → tune + **lock D34a thresholds**.
5. **M4 — Settings:** Toggle/Input/Select/Radio/Modal primitives; contexts/rules editor (restores rule editing), exclusions UI + capture-time wiring polish (Phase 1.3), retention picker, **Export CSV** + **Delete-all**. Removes the legacy `Category` IPC naming.
6. **`/design-sync`** from an interactive session.

**Read order:** `CLAUDE.md` → `context/plans/README.md` → this plan's `plan.md` + this handoff → `context/decisions.md` (D1–D36) → the impl-plan → open `design/index.html` + run Storybook (`npm run storybook`) for the component library.
