# Handoff — 2026-06-25-02 · Full-codebase audit + resolution

**Branch:** `phase6/audit-fixes` (off `main`). **Status:** all fixes landed + verified; ready for review/merge.
**Artifacts:** review report `reviews/2026-06-25-full-codebase-audit.md` (+ raw findings JSON) · impl-plan `impl-plans/2026-06-25-audit-fixes.md` · ADR **D53** · plan.md Phase 6 ticked.

## Where we are now
Ran the project's own `/usageos-review` (D50) against the **entire codebase** as a deep first pass — a Claude 14-subsystem panel + 7 parallel Codex lanes, every finding adversarially verified. **Verdict: healthy — 0 critical, no hard-rule violations.** Then resolved the findings in verified batches on this branch. **Net −184 LOC.** Gates green throughout: **114 Rust + 27 TS tests**, clippy `-D warnings`, fmt, tsc, vitest, bindings fresh.

## What landed (all verified)
**Correctness bugs (each with a test):**
- **Slugless/user categories no longer collapse into "Uncategorized"** — the worst one. They keep a stable identity (`cat-<id>`) and their hex `color` threads through `CategorySlice`/`CategoryRun`/`TimelineRun`/`TimelineSegment` (IPC additive; bindings regenerated); frontend `categoryColorVar(slug, color)` renders the theme token for canonical slugs, the hex for user categories.
- **Capture idle-inflation** — `on_focus` gates on the open/last window's idle gate, so background title-churn can't inflate or spawn phantom spans; `same_window` includes `project_id` (terminal `cd` → new span).
- **Day-boundary clipping** — `get_activity_logs` selects overlapping spans (half-open) + clips to the window; `dates.ts dayBounds` is now half-open + DST-correct. A midnight-crosser counts once per day.
- **`reprocess_logs`** — atomic (one transaction) + literal-substring match identical to live `find_category` (escaped LIKE; no `%`/`_` wildcard divergence).
- **enrich** — `github.com` exact/subdomain (no look-alikes), `127.0.0.1:port` as local, browser tab titles no longer mint fake projects.
- **`leading_project`** tie-break; **icon cache key** collision-free; **empty-pattern** matcher guard (no blackout); **ThemeProvider** load-race; **`CANON_ORDER`** includes `personal`; **A6b** new-category seeds the app; **A7** moved-app un-toggle (incl. substring owner rules — caught by the Batch-2 verification panel); **useViewData** stale-response guard (no wrong-range render).

**Hardening / hygiene / simplification:**
- a11y: `Modal` focus-trap + restore + scroll, `Select` keyboard nav, `LedgerRow` keyboard-reachable.
- Tokens: `--scrim` / `--bar-muted` (3 themes); off-token colors swept; flat-fill skeleton.
- Dedup: `load_lookup_maps` (Rust), `useViewData` + `useCaptureHealth` + shared `ErrorState`/`DateStepper` (frontend).
- Dead `FocusEvent.bundle_id`/`.pid` removed; migration **drift-on-missing** detection added.
- **Decision-archaeology comment sweep** (the audit's biggest cluster) across Rust + TS — narration trimmed to one-line why + cross-ref; 2 stale comments fixed (`0001` rename, migrations README chain).
- Config: scoped `opener` capability; real `Cargo.toml`/`package.json` metadata.
- **`db.rs` (1674 lines) → `db/{mod,events,projects,categories,exclusions,settings}.rs`** — pure re-export split, zero behavior change.

## Deferred (deliberately — see D53)
- **Native (on-device session):** move `osascript`/`pgrep` enrichment off the main run loop + producer full-identity dedupe (A2c/B4). The objc2/AX code is verified on a Mac, not CI — blind run-loop edits risk a UAF (D32/D33).
- **Coordinate with PR #17:** CSP (`csp:null`) and the `tokio` feature trim — #17 owns `tauri.conf.json`/`ai/` and its sidecar timeout needs `tokio::time`.
- **Owner decision:** the Chromium keep-title-when-private-unprovable posture (D33/R18).
- **Perf-gated:** `Arc<Mutex<Connection>>` → R57 (dedicated WAL writer thread) — correct today.
- Migration `CHECK` constraints (needs a table-rebuild migration); `cargo-deny` dep audit; the 2.13 GB memory diagnosis (unchanged Phase-6 item).

## Next steps
1. Review/merge this branch (consider squashing the Batch-1 safety checkpoint).
2. The deferred items above, in their respective sessions / coordinated with PR #17.
3. Add `CategoryEditorModal` component tests (the under-tested file where the audit's near-misses clustered).
