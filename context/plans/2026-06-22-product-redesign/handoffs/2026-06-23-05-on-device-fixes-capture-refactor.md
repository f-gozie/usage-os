# Handoff — on-device fixes, capture rebuilt as a state machine, + the /debate skill

_Written 2026-06-23 (after [`2026-06-23-04-bauhaus-day-dial.md`](2026-06-23-04-bauhaus-day-dial.md)). Read `CLAUDE.md` first, then this. This session ran the Day dial on the real Mac for the first time, fixed what that surfaced, then **rebuilt the capture write-path** as a clean single-writer state machine (decided via a two-agent debate), and turned that debate into a reusable `/debate` skill. All on branch **`phase1/bauhaus-day`** → **[PR #13](https://github.com/f-gozie/usage-os/pull/13)** (pushed; not merged)._

## 1. Where we are (TL;DR)
Phase 1.4 (the Bauhaus Day dial) is **on real data and working end-to-end**. First on-device run exposed two things — the dial showed 0 (a real capture bug) and the design read as a "window-in-a-window" — both fixed (D37). Then, on the question "is this the best span/session architecture?", the capture write-path was **refactored to a single-writer in-memory state machine** (D38), a decision reached by pitting an external **Codex** agent against an **Opus** agent (they converged). Gates green throughout (69 Rust + 7 TS tests, clippy/fmt, bindings byte-identical). Next: **dogfood real days** to tune the idle gate + starter rules, then M2 (Week).

## 2. What happened this session (after handoff 04)
1. **First on-device run (`npm run tauri dev`).** Capture worked (events with titles, correct categorization) but the dial summed to **0** — every span was written `start == end` (zero duration), and sustained single-window work fired no events so never accrued time.
2. **D37 — on-device fixes** (commit `261d729`): close-on-switch + an active heartbeat (interim); **window chrome integrated** (macOS `titleBarStyle: Overlay` + `hiddenTitle`, default size 1180×940 + scroll, drag region, fake titlebar dots removed); fixed the invisible **"Tracking"** indicator (Tailwind opacity modifiers break on `var()` tokens) + wired it to live `get_watcher_status`; **subtle theme switcher** (three swatches); a manual **refresh** button.
3. The user **back-filled** the existing ~13 min of zero-duration test data themselves (chained ends → next starts).
4. **D38 — capture refactor** (commit `93d3941`), decided by a **Codex↔Opus debate**: replaced the two-thread "re-derive the last DB row" design with a **single consumer thread = sole writer** owning the open span in memory (`current` + `last_focus`), **self-ticking via `recv_timeout`** (heartbeat thread deleted). Both reviewers caught the same bug (a chatty live-title tab while away starves a naive `recv_timeout` → balloons the span) → idle is checked on a **wall-clock deadline every loop wake**. Resolved their splits: keep `last_focus` to **resume the same window after idle**; extend to **`now`** (forward-only, gate-checked), not `now−idle`; **TICK=20s, GATE=120s**. Deleted `log_focus`/`log_activity`/`extend_current_span`/`get_last_activity_log` + the gap/stale constants; `update_last_activity_end_time`→`set_span_end`. ~6 knobs → 2.
5. **New `/debate` skill** (user-level: `~/.claude/skills/debate/SKILL.md`) — the reusable two-agent adversarial-review pattern (neutral shared brief, no steering, rounds, you synthesize). Plus a memory pointer.

## 3. Decisions locked this session
- **D37** — spans get real durations (close-on-switch + heartbeat, interim) + the app window *is* the design (Overlay titlebar, sized, draggable; tracking/theme fixes).
- **D38** — capture write-path is a single-writer in-memory state machine; idle on a wall-clock deadline; `last_focus` resume; extend-to-`now`; 2 knobs (TICK=20/GATE=120). Supersedes D37's mechanism. _Informed by the Codex+Opus debate._

## 4. Gotchas / state
- **PR #13 is open, not merged.** Branch `phase1/bauhaus-day` carries the whole redesign (design → migrations D35 → rollup/`get_day` D36 → starter rules → Bauhaus UI + Storybook → docs → D37 → D38). Review/merge when ready, or keep stacking M2 on top.
- **The `/debate` skill is USER-LEVEL** (`~/.claude/skills/`), **not in the repo** — available next session as `/debate`. Move it into the repo's `.claude/` if you want it shared with contributors. It needs the `codex` CLI (`which codex`) for the Codex side; falls back to two `Agent` models otherwise.
- **No DB reset needed** to re-run — schema unchanged; your back-filled data stays. Capture now produces **real durations**: sit in one window → Active grows; switch → clean spans; walk away >2 min → time stops; return to same window → resumes.
- **`GATE=120s` / `TICK=20s` are dogfood-tunable placeholders** (D34a + D38). GATE is the one to watch (breaks stitched in → too high; reading cut off → too low). **Idle is now an untracked gap** (no idle rows written) — the dial's empty track *is* idle.
- **Starter categorization rules** (migration 3) are still opinionated defaults flagged for your veto — judge them against your real apps during dogfooding.
- **Settings / Week / Timeline are still "coming soon" placeholders** (M2–M4). Old Settings UI (categories/rules/retention) stays gone until M4.

## 5. Next steps
1. **Dogfood a few real days** → tune `GATE`/`TICK` (D34a) and the starter rules; this is also the data for locking the run-segmentation thresholds (D34a / the session explorer).
2. **Review + merge PR #13** (or continue stacking).
3. **M2 — Week:** `get_week` → `WeekView` (7 `MiniDial`s, shared engine).
4. **M3 — Timeline:** `get_timeline` (context-run blocks + idle gaps) → `TimelineView` + the session-explorer → lock D34a.
5. **M4 — Settings:** rebuild in Bauhaus (contexts/rules editor, exclusions UI, retention, Export CSV + Delete-all); removes the legacy `Category` IPC naming.
6. `/design-sync` (a Storybook component library exists now); Apple Developer enrollment (off critical path).

**Read order:** `CLAUDE.md` → `context/plans/README.md` → this plan's `plan.md` + this handoff → `context/decisions.md` (D1–D38) → run `npm run tauri dev` + `npm run storybook`. For hard calls, use **`/debate`**.
