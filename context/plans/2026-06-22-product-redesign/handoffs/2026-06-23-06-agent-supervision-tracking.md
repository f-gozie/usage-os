# Handoff — agent-supervision tracking: per-app idle gate (D39), Claude→Deep, headline tile

_Written 2026-06-23 (after [`2026-06-23-05-on-device-fixes-capture-refactor.md`](2026-06-23-05-on-device-fixes-capture-refactor.md)). Read `CLAUDE.md` first, then this. This session started from a real dogfooding question — "I watch agents run for 15+ min; does that count, and why does the dial look off?" — diagnosed it against the live DB, shipped two concrete fixes, then ran a **second Codex↔Opus `/debate`** on the tracking model and implemented the outcome (**D39**). All on branch **`phase1/bauhaus-day`** → **[PR #13](https://github.com/f-gozie/usage-os/pull/13)** (still open, not merged)._

## 1. Where we are (TL;DR)
The "is it even tracking?" worry was **categorization + the idle model, not a capture bug** — the recorder is honest. Confirmed by a read-only DB query: the agent-supervision time *was* recorded, just filed as **Research** (the `Claude` app) with long no-input stretches gated out. Fixed the two concrete things, then settled the deeper "how should watching an agent count?" question by debate → **D39: no agent-detection; a capped per-app idle gate that folds supervision into Deep work; the consumer is the single idle source.** Gates green throughout (**73 Rust + 7 TS**, clippy/fmt/tsc). Next: **reopen the app** (a migration re-applies), then dogfood the new gate, then M2 (Week).

## 2. What happened this session (after handoff 05)
1. **Diagnosis (read-only DB query).** Most-recent session: `Claude` → Research ~20m, `usage-os` (the dev app) → Uncategorized ~7m, browsers → Research, real editor/terminal → ~14s. So Deep work read as a near-zero "14s." Tracking correct; the *defaults* don't know what "watching an agent" is.
2. **Headline tile fix** ([DayView.tsx](src/views/DayView.tsx)): the hard-coded **"Deep work"** stat (often ~0 → looked broken) now shows the **day's leading context** (name + time + colour).
3. **Claude → Deep work** — new migration [`0004_recategorize_claude_as_deep.sql`](src-tauri/migrations/0004_recategorize_claude_as_deep.sql): re-points the `Claude` rule Research→Deep **and** backfills already-recorded rows (scoped to ones auto-filed as Research). The claude.ai *web* tab is unaffected (it matches browser process rules, not `Claude`).
4. **⚠️ Self-inflicted crash + recovery (lesson).** I edited `0004` to add the backfill *after* the live `tauri dev` had already applied the rule-only version → the checksum **drift guard** (working as designed) panicked on next boot ("migration 4 changed after it was applied"). Fix: reset the dev DB's `schema_migrations` row (`DELETE … WHERE version = 4`) so the final `0004` re-applies cleanly; verified on a DB copy (moves 32 `Claude` rows Research→Deep, nothing else). **Lesson: never edit a migration after it's been applied — add a new one or reset the local record.**
5. **`/debate` (2 rounds, Codex + Opus) on the tracking model → D39.** Round 1 split (Opus: fold into `active` via a capped per-app gate; Codex: a separate "supervising" dimension). Round 2: **Codex conceded to Opus's position** — they converged on **Position A**. Synthesis: a separate dimension can't tell "watching" from "away" either, so it makes a falsely-precise claim and costs a whole new axis/UI/mental model; a *capped* per-app gate is simpler and bounds the over-count.
6. **Implemented D39** ([capture/mod.rs](src-tauri/src/capture/mod.rs)): `gate_secs_for(&app)` — default `GATE_SECS=120`, `PATIENT_GATE_SECS=600` for `PATIENT_APPS` (Cursor/Code/Xcode/iTerm/Terminal/Warp/Ghostty/Zed/Claude), resolved in `on_tick`; **never disabled** (the cap bounds phantom focus to ~10 min). Reconciled the **single idle source**: deleted the vestigial producer-side idle (`FocusEvent.is_idle` field + macOS `IDLE_THRESHOLD_SECS=180` + polling `is_user_idle`) that the consumer never read. New tests for the per-app gate.

## 3. Decisions locked this session
- **D39** — agent-supervision time: **no agent detection** (no privacy-safe signal separates watching from away; declined the over-engineering trap); a **capped per-app idle gate** folding supervision into **Deep work** (Position A, via the Codex↔Opus debate); the **consumer is the single idle source** (producer-side idle removed). Claude desktop app Research→Deep (`0004`). Headline tile → leading context.

## 4. Gotchas / state
- **PR #13 is still open, not merged.** It now also carries the headline tile, migration `0004`, and D39 (per-app gate + idle-source reconciliation).
- **Reopen to apply.** The dev DB's `schema_migrations` version is currently **3** (I reset row 4). On next `npm run tauri dev` it re-applies the final `0004` (32 `Claude` rows move Research→Deep) **and** rebuilds with the D39 code — both land together, clean boot. If any *other* machine applied a rule-only `0004`, it needs the same one-line reset.
- **`PATIENT_GATE_SECS=600` / `PATIENT_APPS` are D34a dogfood-tunable** (like `GATE=120`). Watch two directions: `GATE` for over-count in normal apps; `PATIENT_GATE` for walk-aways *inside editors/terminals* leaking in (pull toward 300–480 if so).
- **`activity_logs.is_idle` is now always `false`** (idle = untracked gap since D38). The `rollup` idle branch never fires — harmless/forward-compatible, left in place.
- The **patient allowlist is deliberately decoupled from categorization** (a browser doing "research" is not a patient surface), so it's a small hardcoded const, not "all deep-work apps." User-editable allowlist is an M4 Settings concern.
- **Starter rules** (migration 3) are still opinionated defaults flagged for your veto — keep judging them while dogfooding.

## 5. Next steps
1. **Reopen the app**, confirm: Claude time shows under Deep work; the headline tile shows the leading context; sitting hands-off in an editor/terminal/Claude keeps accruing (up to ~10 min) where before it stopped at 2.
2. **Dogfood a few real days** → tune `GATE` / `PATIENT_GATE` / starter rules (D34a); this is also the data for locking run-segmentation (D34a / session explorer).
3. **Review + merge PR #13** (or keep stacking).
4. **M2 — Week:** `get_week` → `WeekView` (7 `MiniDial`s, shared engine).
5. **M3 — Timeline:** `get_timeline` → `TimelineView` + session-explorer → lock D34a.
6. **M4 — Settings:** rebuild in Bauhaus (contexts/rules editor incl. a user-editable patient allowlist, exclusions, retention, Export CSV + Delete-all).

**Read order:** `CLAUDE.md` → `context/plans/README.md` → this plan's `plan.md` + this handoff → `context/decisions.md` (D1–D39) → run `npm run tauri dev` + `npm run storybook`. For hard calls, use **`/debate`**.
