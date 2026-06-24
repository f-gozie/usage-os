# Impl-plan — Bauhaus UI rewrite, end-to-end (Phase 1.4 → the dial)

_Approved 2026-06-23 (plan-mode). The as-built detail behind the Day-view PR(s). Roadmap lives in `../plan.md`; the why lives in `context/decisions.md`._

## Context — why this, why now

Backend is ready for a real UI: capture is on-device-verified, the data model landed (migrations v5–v8), tauri-specta IPC is wired. The design system is **frozen** as 10 interactive HTML pages in `design/` + the contract in `context/design-system.md`. Only the frontend is still cyberpunk (`src/components/*`, recharts, neon tokens).

Direction (from the user):
- **Backend-first, real data.** Aggregation + context-run segmentation live in **Rust** (Hard Rule 6 — numbers in Rust), exposed as typed commands. The frontend consumes them. No throwaway TS aggregation layer.
- **Clean cutover, no stale/mock code.** A Bauhaus view landing removes its cyberpunk predecessor + dead deps in the same change. No mock data in the app's runtime path — fixtures live only in Storybook/tests. Empty/degraded states replace "fake a day."
- **Storybook**, multi-theme, as per-component verification (and unlocks `/design-sync` later).
- **nudge is a reference, not a template** — take what earns its place, build the rest ourselves.

Out of scope: the Swift Foundation Models recap sidecar (Phase 3) — we ship the deterministic Rust **template recap** now; FM prose layers in behind the `ai` trait later.

## Binding constraints (from the frozen contract)

- One colour axis = **context** (deep/research/comms/breaks → `--c-*`); **projects identified by NAME, never colour**; idle = `--track`.
- **Dial arcs + Timeline = context-runs**; project split shown *inside* a block as a text line (`usageos 1h 3m · nudge 39m`), never a bar, never text inside a proportional segment; off-project time counts to its context.
- Dial = fixed 24h, midnight top (D14); 1px ink casing in `paper` (R77); colour never the only cue.
- 3 themes (paper/warm/black), token-only; Anton + Jost; hard edges, flat fills; honour `prefers-reduced-motion`.
- D34a thresholds NOT locked → Rust named constants, tuned later by dogfooding (no React churn).
- IPC generated (`src/bindings.ts`, regen via `cargo test export_bindings`); commands-only (D27).

## Build decision made during implementation — migration system (Path A)

Before adding the contexts schema, the migration system was hardened (it was an inline `const MIGRATIONS` array in `db.rs` with a real atomicity bug — DDL and bookkeeping ran un-transacted). Decision (user-approved):
- **Hand-written SQL stays** (SQLite ALTER limits + Hard Rule 4 raw-SQL + audit-the-source ethos rule out derived/ORM migrations).
- **Path A — harden in-house, zero new deps:** per-file `.sql` under `src-tauri/migrations/` (`include_str!`'d), a top-level `migrations` module runner, each migration applied **in a transaction**, a **stable FNV-1a checksum** stored per migration and verified on boot (shipped migrations are immutable; an edit is a loud startup error), **forward-only** (no down-migrations) — documented in `migrations/README.md`.
- **Clean-slate squash:** since nothing has shipped, the historical v1–v8 chain is collapsed into one readable baseline (`0001_initial_schema.sql`) with the contexts `slug` column + all enrichment columns inline, plus `0002_seed_canonical_contexts.sql`. No baseline cutover needed; existing dev DBs are throwaway (delete before running).
- Contexts: internal table name stays `categories` (D31 rename still deferred); `slug` (`deep|research|comms|breaks`) carries the colour-token identity; canonical contexts seeded. New IPC vocabulary is "context."

## Target backend surface (new typed commands)

Add a Rust read-model/rollup layer (SQL stays in the repository; rollup consumes typed reads, returns view structs; project names joined server-side):
- `get_day(start, end) -> DayView` — `active_secs`, `idle_secs`, `contexts: Vec<ContextSlice{slug,name,secs,pct}>`, `runs: Vec<ContextRun{context_slug,start,end,projects:Vec<ProjectSlice{name,secs}>,apps:Vec<String>}>`, `recap: Recap{facts,text,generated_by:"template"}`.
- `get_week(start, end) -> Vec<DayMini>` — per-day per-context sums (M2).
- `get_timeline(start, end) -> Vec<TimelineEntry>` — context-run blocks + idle gaps, expandable (M3).
- Segmentation in a pure, unit-tested module with named D34a constants (`EXCURSION_ABSORB`, `IDLE_GAP_ENDS_RUN`, `SUSTAINED_SHIFT_SUBSPLIT=off`).

## Frontend architecture

```
components/{ui,dial,shell}/   views/   lib/   hooks/   providers/   styles/   test/
```
- **Theming:** frozen token blocks verbatim → `styles/tokens.css` as `:root[data-theme=…]`; `ThemeProvider` sets `data-theme` on `<html>` + persists via the Rust settings table; Tailwind rewired to map utilities → CSS vars (neon removed).
- **Dial:** pure data-driven SVG (300×300, midnight top), geometry in `lib/geometry.ts`; arcs = context-runs by `--c-{slug}` + casing; now-triangle; centre figure (no hub dot); hover-dim + tooltip; click → inspector; draw-in gated on `useReducedMotion`. MiniDial shares the engine.
- **Data:** plain hooks + context (no state lib). `useDayData(date)` wraps `get_day` + 30s refresh; views are thin containers; components are presentational + Storybook-able.
- **From nudge:** CSS-var `[data-theme]` theming, folder/test discipline, `useReducedMotion`, vitest+RTL mock patterns. **Not** taken: framer-motion-everywhere, a state lib, CVA, monorepo, its visual language.

## Milestones (backend-first within each; cutover atomic)

- **M1 — Day/dial:** migration hardening + contexts; rollup + `get_day` (+ Rust tests) + bindings; token/theme foundation + shell; Day-view UI primitives + stories; Dial + DayView (loading/empty/degraded, no mock); Storybook + RTL setup; **cutover** (delete ActivityChart/StatsCard/TimeRangeSelector/dashboard JSX/neon/recharts; App = shell + state routing).
- **M2 — Week:** `get_week` → WeekView (7 mini-dials).
- **M3 — Timeline:** `get_timeline` → TimelineView; build the session-explorer → **dogfood + lock D34a**.
- **M4 — Settings:** Toggle/Input/Select/Radio/Modal + rows; wire contexts/rules editor, exclusions UI + capture-time wiring (Phase 1.3), theme/retention pickers, Export CSV + Delete-all; remove legacy `Category`/rule UI + dead commands.
- **M5 (later, Phase 3):** FM recap sidecar (template→prose behind `ai` trait); `/design-sync` push.

## Verification

- Rust: `cargo test` (migrations incl. checksum-drift + transaction, rollup/segmentation, repository), `cargo clippy -D warnings`, `cargo fmt --check`.
- Bindings fresh: `cargo test export_bindings` → no diff.
- Frontend: `tsc --noEmit`, `npm test` (pure-logic + RTL), `npm run storybook` (3 themes).
- On-device: `npm run tauri dev` — real captured day renders; empty state with no data; degraded banner if Accessibility off; arc click → inspector. (Delete the throwaway dev DB first — schema is a clean-slate squash.)
- Docs lockstep (DoD): tick `plan.md` Phase 1.4; append ADRs (migration hardening; read-model/contexts-slug); new `handoffs/` entry.
