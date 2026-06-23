# Impl-plan — Timeline view (M3)

_Branch `phase1/bauhaus-day` (stacked on PR #13). Backend-first, mirroring Day/Week. Design source of truth: `design/timeline.html`. This is the **session explorer** that surfaces run-segmentation so D34a can be tuned + locked after dogfooding._

## Goal
A per-day **agenda of context-runs**: each run a row (start time · context-colour spine · context name + project/apps lines · duration + time-range + chevron), **click-to-expand** → every app-switch inside it (time · app · project · duration). **"Away · Nm"** rows for idle gaps ≥ threshold; a **"Now"** row at the current time. Legend chips up top. Read-time rollup over `events` (no schema change); numbers in Rust (hard rule 6).

## The data gap
`ContextRun` (the dial's unit) has `apps: string[]` but not the per-event detail the expand needs. So the Timeline gets its own richer read-model — a run **plus its inner segments** — without bloating `get_day`/the dial.

## Backend (Rust)
- **`rollup.rs`** — new types + builder:
  - `TimelineSegment { start, end, app, project: Option<String>, secs }` — one focused-window event inside a run (the expand detail).
  - `TimelineRun { context_slug, context_name, start, end, secs, projects: Vec<ProjectSlice>, apps: Vec<String>, segments: Vec<TimelineSegment> }`.
  - `TimelineView { runs: Vec<TimelineRun> }` (idle gaps + the now-row are derived on the frontend from run bounds + current time, like the mockup).
  - `build_timeline(events, contexts, projects) -> TimelineView` — same segmentation as `build_runs` (context change **or** gap ≥ `IDLE_GAP_ENDS_RUN_SECS`), but retains each event as a segment; `projects` aggregated from segments (reuses `ProjectSlice`/`NO_PROJECT`). NOTE: uses the **current** segmentation only — the D34a excursion-absorb / sustained-shift refinements are explicitly out of scope; this view is the tool to evaluate them.
  - Tests: run coalescing keeps segments; context change + idle-gap split; segment project = `None` when unresolved.
- **`lib.rs`** — `get_timeline(db, start_time, end_time) -> Result<rollup::TimelineView>` (per-day reads like `get_day`). Register in `collect_commands!`. Regenerates `bindings.ts`.

## Frontend (React)
- **`lib/tauri.ts`** — `getTimeline` + re-export `TimelineView`, `TimelineRun`, `TimelineSegment`.
- **`hooks/useTimelineData.ts`** — mirror `useDayData` (load + 30s refresh on today).
- **`views/TimelineView.tsx`** — the agenda:
  - Header: "Today"/date label + the context legend chips.
  - For each run: a `TimelineRow` (collapsible) — start time, colour spine, context name, project label (projects w/ durations, or apps when no project), apps line + switch count, duration + `hh:mm–hh:mm`, chevron. Expanded → the segment rows (time · app · project · duration).
  - Between runs with a gap ≥ a display threshold → an **Away · Nm** row.
  - A **Now** row (triangle + line) when viewing today.
  - Loading/empty/degraded states like Day/Week.
- **`components/timeline/TimelineRow.tsx`** (+ story + test) — the one reusable piece (collapsible run row). Keep `TimelineView` orchestration thin.
- **`App.tsx`** — replace `<Placeholder title="Timeline" />` with `<TimelineView date … />`; the header date slot already handles the chrome (reuse the Day-style two-line date).

## Gates / done
`cargo clippy -D warnings`, `cargo fmt`, `cargo test` (incl. fresh `bindings.ts`), `tsc`, `npm test`, vite + storybook builds. Update `plan.md` (tick Timeline) + a handoff. **Then dogfood → tune + lock D34a** (the segmentation thresholds) using this view.

## Explicitly NOT in this task
The D34a threshold *changes* themselves (excursion-absorb / sustained-shift / final idle-gap value) — those are tuned against real data *using* this view, then locked in a follow-up. Embedding categorization + rules editor (M4). Cross-day timelines.
