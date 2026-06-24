# Handoff — Phase 2 merged to main; Phase 3 recap started (FM spike → viable, narration seam = chunk A)

_Written 2026-06-24 (after [`2026-06-24-02-app-icons-categories-rename.md`](2026-06-24-02-app-icons-categories-rename.md)). Read `CLAUDE.md` first, then this. A long session: committed + **merged** the whole M4 line to `main`, closed out Phase 2 by **shelving embeddings** for app-metadata suggestions + a relatable default taxonomy, built the **recap Rust foundation**, **spiked Foundation Models** (viable), and landed the **recap narration seam** (chunk A) — now open as PR #15 (D47–D49)._

## 1. Current state overview
- **Product:** UsageOS — a private, on-device macOS "calm rear-view mirror."
- **Phases:** 0–1 ✅; **Phase 2 ✅ COMPLETE** (now merged to `main`); **Phase 3 (recap) IN PROGRESS** — step 1 done + on `main`; step 2 (sidecar) chunk A done, chunks B–D pending. Phases 4 (shell/polish), 5 (launch), 6 (optimization/hardening) ahead.
- **`main`** is at `f98aaa8` — **PR #14 squash-merged** (M4 + M4.1 + M4.2 + Phase-3 step 1, D42–D48).
- **Active branch: `phase3/recap-sidecar`** — **PR [#15](https://github.com/f-gozie/usage-os/pull/15) OPEN** (the FM spike + chunk A). The Swift sidecar isn't wired yet, so the seam is **dormant** (app behaves as step 1: the richer template recap).

## 2. Key decisions (this session — full ADRs in `context/decisions.md`)
- **D47 — Phase-2 categorization: embeddings shelved, pivot to app-metadata + relatable defaults.** Spiked `NLEmbedding` (`spikes/embeddings/`): mechanism works (Rust, off-thread, 512-d, no Apple Intelligence) but accuracy is **below the majority-class baseline** (app-name LOO 39% / centroid 41% / title 60% vs 43%) — brand names are out-of-vocabulary noise. So **not built**. Instead: rules stay primary + D44's Assign→rule is the corrections memory; **`LSApplicationCategoryType`→category suggestions** (`apps::suggest_slug` via `NSBundle`, conservative/abstaining) as a one-click "sort into X" in Uncategorized; and a **relatable default taxonomy — Work · Browsing · Messaging · Entertainment · Personal** (5th canonical, `--c-personal` green; migration `0006`, fresh-installs-only).
- **D48 — Recap step 1: a Rust `RecapFacts` fact layer + a purely-factual template.** `compute_recap_facts`/`format`-ready facts (leading + runner-up category, leading project ≥40%, longest stretch ≥15 min with a coarse local time-of-day via `(run.start − day_start)/3600`); `build_day_view` gains `day_start`. Voice **purely descriptive, never evaluative**.
- **D49 — Recap sidecar: Foundation Models VIABLE (spiked), build behind a Rust seam, in chunks.** Spike (`spikes/foundation-models/`) compiled vs the real macOS-26 SDK first try; `available` on this M2 Pro; ~5.4s cold / ~1–2s warm; quality risks (proper-noun mangling, "47m"→"47 million") fixed by firm verbatim/2nd-person instructions (no personal names in the OSS prompt) + `temperature: 0.2` + units spelled out by Rust. **Chunk A = the seam** (this PR); B–D pending.

## 3. Changes implemented (by area)
**Merged to `main` (PR #14):** all of M4.1 (D43–D46, app icons + categories overhaul + Timeline latest-first + Context→Category rename), M4.2 (D47), and Phase-3 step 1 (D48).
**On `phase3/recap-sidecar` (PR #15):**
- **`spikes/foundation-models/`** — standalone Swift CLI (availability gate + one `@Generable` recap call + `--serve` stdio loop) + README verdict.
- **`spikes/embeddings/`** — the M4.2 negative-result spike (rides along on this branch; already on `main` via #14).
- **`src-tauri/src/ai/mod.rs`** — `Narrator` trait (async, `#[allow(async_fn_in_trait)]`) + `FakeNarrator` + `AiError` + `build_recap` (try → fallback). `pub mod ai;` in `lib.rs`.
- **`src-tauri/src/rollup.rs`** — `RecapFacts`/`CategoryFact`/`FocusFact` now `pub`; `render_template_recap` `pub(crate)`; new `format_recap_prompt` + `human_secs_long` (units spelled out, fields labeled).
- **Docs:** D49, plan.md (Phase-3 sidecar `[~]` with chunk-A status), impl-plan `2026-06-24-phase3-recap-sidecar.md`.

## 4. Progress completed
- M4.1 committed + PR #14 updated, then **PR #14 merged to `main`** (squash, `f98aaa8`) after green CI (macOS + Linux).
- Phase 2 closed: embeddings spiked + shelved; app-metadata suggestions; **relatable defaults shipped** (migration 0006, fresh-installs-only) **and applied to the dev DB directly** (Work/Browsing/Messaging/Entertainment/Personal, Personal adopted the `personal` slug + green).
- Recap step 1 (D48) on `main`.
- FM spike → viable (D49); recap seam (chunk A) built, gated, PR #15 opened.

## 5. Current blockers
- **None hard.** Chunk A is green and dormant; the app runs.

## 6. Work in progress
- **PR #15 is chunk A only.** Chunks **B–D** (below) continue on `phase3/recap-sidecar`. Decide whether to merge chunk A alone or accumulate B–D into #15 (like #14 accumulated milestones).

## 7. TODO (remaining)
1. **Recap chunk B** — productionize the spike Swift into the repo (`sidecar/usageos-ai/`): generic prompt, `prewarm()`, `--serve`, `temperature: 0.2`, stable status tags, no network entitlement (C8).
2. **Recap chunk C** — Tauri `externalBin` (binary named `usageos-ai-$TARGET_TRIPLE`) + `tauri-plugin-shell` + capability `shell:allow-spawn` (`sidecar: true`); the real `SidecarNarrator` (spawn, **line-buffered** stdout, status-tag branch, **per-call timeout** — C4–C7); `prewarm()` at launch.
3. **Recap chunk D** — async `get_recap(start,end)` command computed **after** the dial renders (never block the day load — D11); the recap card shows the template instantly then **upgrades** to AI prose with the "⌁ Summarized on-device" vs "≡ Template" badge (`design/day.html`). Expose `compute_recap_facts` (pub(crate)) so the command can build `RecapFacts` from events.
4. **Recap CI lane** — a separate, non-blocking macOS-26 Swift build lane (C20); cross-platform CI stays green via `FakeNarrator` (C19).
5. **Recap voice tuning** — even when correct the prose is a touch generic; tune to the copy bar over real days. Deferred: opt-in evening "your day is ready" ping.
6. **Later phases:** Phase 4 (menubar/onboarding/dark-mode/day-start D14), Phase 5 (launch), Phase 6 (the unreproduced 2.13 GB memory question + perf/security/hardening).

## 8. Important context / gotchas
- **Migration drift from `tauri dev` (NEW gotcha, cost us a crash this session):** `tauri dev`'s watcher auto-applies a new migration to the **live dev DB** on rebuild; editing that migration afterward trips the startup drift guard ("migration N changed after it was applied"). Fix: stop the app, `DELETE FROM schema_migrations WHERE version = N`, restart (it re-applies the final version). **Stop the dev stack before adding/iterating a migration.** (Memory `tauri-dev-migration-drift`.)
- **WAL reads:** read the live DB with a plain `sqlite3 "$DB"` connection, **not** `?immutable=1` (which ignores the `-wal` and shows a stale snapshot — bit me twice).
- **cargo vs `tauri dev`** still collide on `target/`; stop the stack for `cargo` gates. **Testing the chunk-C sidecar spawn needs the dev app running** — that conflicts with cargo, so expect a stop/start dance.
- **tokio features = `rt`/`time`/`sync` only (no `macros`)** — async tests use a hand-rolled `block_on` via `Builder::new_current_thread()` (see `ai/mod.rs`). Adding `tokio-plugin-shell` may pull more tokio features.
- **OSS prompt hygiene:** never bake personal/project names into the sidecar prompt (the spike leaked "usage_os"/"nudge" as examples — fixed to generic). Names arrive only as runtime facts.
- **`gh pr edit` is broken in this repo** (projects-classic GraphQL deprecation) — use `gh api -X PATCH …/pulls/N` to edit PR title/body. `gh pr create`/`gh pr merge` work fine.
- **Dev DB state:** the dev `usage.db` was hand-migrated (category renames + `personal` slug) and its v6 migration record cleared; backups at `usage.db.pre-rename.bak` / `usage.db.pre-migfix.bak`.

## 9. Testing status
- **Green:** 106 Rust tests (incl. 5 `ai` seam tests) + 20 TS tests; clippy `-D warnings`, fmt, tsc, vite + storybook; bindings fresh. The FM spike is verified **on-device** (model `available`, prose returned, warm latency measured).
- The real sidecar path is **untestable on standard CI** (needs macOS 26 + Apple Intelligence) — covered by the `FakeNarrator` + (chunk C) a separate macOS lane.

## 10. Next steps recommendation
1. **Continue on `phase3/recap-sidecar`** with chunk B → C → D as a focused push (app **down from the start**, room to iterate on `externalBin` bundling + the spawn).
2. The spike already proved the **recipe** (firm instructions, temp 0.2, units spelled out) and the **seam** is in place — chunk C's risk is the Tauri sidecar bundling (exact target-triple binary name + path), not the model.
3. Decide PR #15's merge timing: bank chunk A now, or accumulate B–D first.

**Read order:** `CLAUDE.md` → `context/plans/README.md` → this plan's `plan.md` + this handoff → `context/decisions.md` (D1–D49) → `spikes/foundation-models/README.md` + `impl-plans/2026-06-24-phase3-recap-sidecar.md` → `npm run tauri dev`. For hard calls, use **`/debate`**.
