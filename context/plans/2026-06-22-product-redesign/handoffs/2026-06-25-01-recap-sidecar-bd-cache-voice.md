# Handoff — Phase 3 recap: sidecar B–D built + wired live, persisted cache (D52), voice tuned

_Written 2026-06-25 (after [`2026-06-24-03-phase2-merged-recap-sidecar-chunkA.md`](2026-06-24-03-phase2-merged-recap-sidecar-chunkA.md)). Read `CLAUDE.md` first, then this. A long session: merged chunk A (PR #15), then built the **whole recap sidecar end-to-end** (chunks B–D + the CI lane), **fixed the bug that made it work live** (sidecar name resolution), added a **persisted recap cache** (D52, decided by a `/debate`), and **tuned the prose voice** (eval-driven). All on `phase3/recap-sidecar-impl`, not yet pushed. **Two ADRs (D51, D52) are written-but-not-in-`decisions.md`** — blocked by a concurrent session; see §5._

## 1. Current state overview
- **Product:** UsageOS — private, on-device macOS "calm rear-view mirror." Phase 3 = the recap (the payoff prose).
- **`main`** is at `f761afb` — **PR #15 merged** (chunk A: the FM spike + the Rust `ai::Narrator` seam, D48–D49).
- **Active branch: `phase3/recap-sidecar-impl`** (4 commits, **NOT pushed, no PR yet**). The recap is now **fully working live**: the Day view shows the template instantly, then upgrades in place to on-device AI prose with the "⌁ Summarized on-device" badge. **Verified on-device** (M2 Pro, Apple Intelligence on).
- Branch commits: `200530b` (sidecar B–D) → `821ef6c` (sidecar-name fix + cache D52 + polish) → `47767cd` (no-"Today" fix) → `385946f` (voice v3). **HEAD is gate-green.**

## 2. Key decisions (this session)
- **D51 (NOT yet in `decisions.md` — must be appended): recap sidecar built end-to-end (D49 chunks B–D).** Productionized the Swift sidecar; one-shot stateless spawn (C2; persistent stdio is open-Q12-unproven); line-buffered stdout (C6); 20s per-call timeout (C7); status-tag→template fallback (C5); off-thread `prewarm()` at launch. **Two production realities the spike's TTY run hid:** stdout must be **unbuffered** (a Tauri child's stdout is a *pipe* → Swift `print` buffers and the read hangs → write via `FileHandle.standardOutput`), and the request is **JSON-wrapped** `{"prompt":"…"}` (the facts prompt is multi-line, would split a line-delimited protocol). **`tauri-plugin-shell` pinned `=2.2.1`** — 2.3.5 forces tauri ≥2.10 whose `tauri-runtime-wry 2.10.1` + `wry 0.54.2` are a broken pair that won't compile; 2.2.1 holds the proven **tauri 2.9.3** stack. **`externalBin` is compile-time-validated on every platform** (tauri-build), so cross-platform CI stages a **stub** sidecar (app never runs in CI — `FakeNarrator`, C19) + a **non-blocking macOS Swift lane** (C20). Lazy async `get_recap` (off the day-load path, D11). Frontend `useRecap` upgrades template→AI in place; fixed a dead `"fm"` badge check → `"foundation-models"`.
- **D52 (NOT yet in `decisions.md` — must be appended): persisted recap cache.** Decided by a **`/debate`** (Codex vs Opus, both independent verdicts converged): **persist successful AI recaps in SQLite, keyed by a content fingerprint of the facts**; **today settles on open + manual ↻ — NO polling, NO TTL/throttle** (both explicitly rejected); only the **model** recap is cached, never the template; the cache is captured-derived data → wiped by `delete_all_data`, pruned by retention. The fingerprint = FNV-1a of `"v{RECAP_CACHE_VERSION}\n"` + the facts prompt; the version covers prompt/instruction changes. Rejected: frontend Map cache (lost on restart), version-counter column, manual cache-bust in reprocess/delete (the fingerprint self-invalidates), auto-refresh today. (The `/debate` brief + both verdicts are summarized in the conversation; the synthesis is the design as built.)
- **The no-"Today" fix** + **voice v3** are prompt/instruction changes folded into the version knob (2, then 3) — see §3. Not separate ADRs (refinements of D51).

## 3. Changes implemented (by area, all on the branch)
- **`sidecar/usageos-ai/`** — the Swift sidecar (Package.swift, `Sources/usageos-ai/main.swift`, `entitlements.plist` [empty, no network — C8], README, `.gitignore`). `main.swift`: unbuffered `FileHandle` output, JSON `{prompt}`→`{status,text,ms}`, `--serve`/default + `--prewarm` modes, `recapInstructions` = **v3 voice** (calm/human, drops "leading category"/"runner-up" scaffolding, anti-verb + faithfulness guards).
- **`sidecar/build.sh`** — `swift build -c release` → `src-tauri/binaries/usageos-ai-$TARGET_TRIPLE`. **Built binaries are gitignored** (`/src-tauri/binaries/*` except `.gitkeep`).
- **`src-tauri/src/ai/sidecar.rs`** — `SidecarNarrator` (spawn name **`"usageos-ai"`** — the basename, NOT `"binaries/usageos-ai"`; see §8), `parse_response`, `prewarm`. `narrate` returns immediately on `Terminated` (dead child) instead of waiting the timeout. 5 tests.
- **`src-tauri/src/ai/mod.rs`** — `pub const GENERATED_BY_MODEL` (so `get_recap` caches only model recaps). `pub mod sidecar;`.
- **`src-tauri/src/rollup.rs`** — `build_recap_facts` (pub(crate); shares `build_day_view`'s aggregation), `recap_fingerprint` + `RECAP_CACHE_VERSION` (now **3**), fingerprint test.
- **`src-tauri/src/db.rs`** — `get_cached_recap` / `put_cached_recap`; one `DELETE FROM recap_cache` in both `cleanup_old_data` (retention) and `delete_all_data` (wipe). 3 cache tests.
- **`src-tauri/migrations/0007_recap_cache.sql`** + registered in `migrations.rs` (version 7).
- **`src-tauri/src/lib.rs`** — async `get_recap` (cache check → hit returns instantly; miss narrates then caches only `foundation-models`); registered in `make_builder`; `tauri_plugin_shell::init()`; manage `SidecarNarrator` + off-thread `prewarm`.
- **`src-tauri/tauri.conf.json`** — `externalBin: ["binaries/usageos-ai"]`, `macOS.entitlements`. **`src-tauri/entitlements.plist`** (app-level, empty/no-network). **`src-tauri/capabilities/default.json`** — `shell:allow-spawn`/`-stdin-write`/`-kill` scoped to `usageos-ai` (sidecar).
- **`src-tauri/Cargo.toml`** — `tauri-plugin-shell = "=2.2.1"` (with the why-pinned comment).
- **Frontend:** `src/lib/tauri.ts` `getRecap`; `src/hooks/useRecap.ts` (fetch once per range, not polled); `src/views/DayView.tsx` (upgrade in place, combined refresh, ↻ tooltip); `src/components/ui/RecapCard.tsx` (badge `=== "foundation-models"` + subtle fade-up animation, keyed on source); `src/index.css` (`recap-in` keyframe, reduced-motion-safe); `RecapCard.stories.tsx` + `RecapCard.test.tsx` (3 RTL tests); `src/bindings.ts` (regenerated — `getRecap`).
- **`.github/workflows/ci.yml`** — "Stage placeholder sidecar binary" step (all runners) + non-blocking `sidecar-swift` job (skips green if SDK < 26).
- **Docs:** `plan.md` ticked (Phase-3 sidecar ✅), impl-plan `2026-06-24-phase3-recap-sidecar.md` as-built section (incl. live-fixes + D52). **`.gitignore`**: `.build/`, `/src-tauri/binaries/*`.
- **`recap-voice.local/`** — the voice eval harness (Swift). **Gitignored via `*.local` — personal, never committed.** Variants × sample days through the real model; how v3 was chosen.

## 4. Progress completed
- PR #15 (chunk A) merged to `main` (this session started by merging it — and **caught + fixed a 49MB `.build/` cache + lost-source mistake** in that PR before merge; main's history is clean).
- Recap sidecar B→D built, wired, **verified live**: badge flips to "⌁ Summarized on-device", prose faithful, `usage_os` verbatim, cache hits instant.
- Cache (D52): past days instant, today settles-on-open, reprocess re-narrates once, wiped on delete-all + retention. 4 cache/fingerprint tests.
- Voice tuned (v3): drops the mechanical scaffolding; e.g. *"You were active for 2 hours 14 minutes, with Messaging taking up most of that time."*
- **Gates green (HEAD):** 115 Rust + 23 TS tests, clippy `-D warnings`, fmt, tsc, vitest, bindings fresh.

## 5. Current blockers
- **`decisions.md` is co-edited by a concurrent `/review`-skill session (its D50, uncommitted in the shared working tree).** So **D51 + D52 were intentionally NOT appended** — adding them would entangle with the other session's uncommitted D50. The review session's files (`CLAUDE.md`, `context/decisions.md`, `context/plans/README.md`, `.claude/`) are **uncommitted and untouched by me**; every recap commit staged only its own files (explicit paths, never `git add -A`). **Once that session commits/clears, append D51 + D52** (content is in the commit messages + the impl-plan as-built section). The pre-push docs tripwire is satisfied via `context/plans/`.
- Otherwise none — the app runs and the recap works.

## 6. Work in progress
- Nothing half-done. Last action was writing this handoff. The branch is clean except the review session's 4 uncommitted files (theirs).

## 7. TODO (remaining)
1. **Append D51 + D52 to `decisions.md`** once the review session frees it (high — Definition of Done).
2. **Push `phase3/recap-sidecar-impl` + open the PR** (user's call). Squash-merge like #14/#15 (one mid-branch commit `47767cd` carried a clippy doc-lint, fixed in HEAD — squash collapses it; CI gates the final tree).
3. **Recap voice:** v3 is good but the small model still drifts occasionally (a stray "runner-up", rare slight over-attribution). Iterate in `recap-voice.local/` if desired; bump `RECAP_CACHE_VERSION` when the prompt changes.
4. **Deferred (not blockers):** opt-in evening "your day is ready" ping; **nested-binary notarization signing** (Phase 5 — Tauri bug #11992, foundation-models.md open-Q10); persistent warm sidecar (only if latency disappoints); the macOS-26 SDK reaching hosted CI runners (the non-blocking Swift lane skips until then).
5. **Later phases:** Phase 4 (menubar/onboarding/dark-mode/day-start D14), Phase 5 (launch), Phase 6 (the unreproduced 2.13 GB memory question + perf/security/hardening).

## 8. Important context / gotchas
- **Sidecar name resolution (the bug that took it from broken→working):** Tauri's `new_sidecar` joins the name **literally** to the exe dir and does **not** re-append the target triple; the bundler/dev-copy places the binary at `<exe_dir>/usageos-ai` (basename, no `binaries/`, no triple). So `sidecar("binaries/usageos-ai")` looked for `target/debug/binaries/usageos-ai` → ENOENT → `Unavailable` → silent template. **The spawn name must be the basename `"usageos-ai"`** (externalBin stays `binaries/usageos-ai` — the *source* path). The Rust-side `app.shell().sidecar().spawn()` does **NOT** check the capability scope (that gates only the JS path) — so a wrong capability won't break Rust spawn, but a wrong *name* will.
- **`externalBin` requires the binary to exist for ANY `cargo build/test/clippy`** (tauri-build validates it on all platforms). Local macOS dev: run **`./sidecar/build.sh`** first (it builds + copies the binary). CI: a stub is staged. If `cargo` ever fails with "resource path `binaries/usageos-ai-…` doesn't exist", that's why.
- **`RECAP_CACHE_VERSION` (rollup.rs) is the cache-bust knob.** Bump it whenever the prompt format OR the Swift `recapInstructions`/temperature change — the fingerprint folds it in, so all cached recaps regenerate. Currently **3** (1=initial, 2=no-"Today", 3=voice).
- **After changing the sidecar Swift or the version:** `./sidecar/build.sh`, then **restart `tauri dev`** (it re-copies the externalBin to `target/debug/` and recompiles the Rust). The running app uses the OLD copied binary until restart.
- **Migration drift + `tauri dev`:** adding migration 0007 needs the dev app **stopped** (the watcher auto-applies it; editing an applied migration trips the drift guard). It's a new file (additive) — no drift if not edited after.
- **`tauri-plugin-shell` is pinned `=2.2.1`** — do NOT bump to 2.3.x until the `wry 0.54.2`/`tauri-runtime-wry 2.10.1` Send/Sync break is fixed (it forces tauri 2.10 which won't compile here).
- **WAL reads:** read the live dev DB with a plain `sqlite3 "$DB"`, not `?immutable=1`.
- **`gh pr edit` is broken** in this repo (projects-classic GraphQL) — use `gh api -X PATCH …/pulls/N`.
- **`recap-voice.local/`** is the personal eval workspace (gitignored). Run voice evals there (`swift run recap-voice`); needs Apple Intelligence on.

## 9. Testing status
- **Green (HEAD):** 115 Rust tests (incl. 5 `ai::sidecar`, 3 `db` cache, 1 `rollup` fingerprint) + 23 TS (incl. 3 RecapCard RTL); clippy `-D warnings`, fmt, tsc, vitest, bindings fresh.
- **Verified on-device:** sidecar narrates as a Tauri child, badge flips, cache hit is instant, no-"Today" holds, v3 voice across tiny/big/single-category/no-project/Entertainment-Personal days.
- **Untestable on standard CI:** the real model path (needs macOS 26 + Apple Intelligence) — covered by `FakeNarrator` + the non-blocking Swift lane.

## 10. Next steps recommendation
1. **Restart `tauri dev`** to eyeball the v3 voice live (the version-3 bump regenerates all cached recaps).
2. **Coordinate `decisions.md` with the review session**, then **append D51 + D52**.
3. **Push the branch + open the PR** (bundle B–D + cache + voice as one PR, the way #14/#15 accumulated). Then the recap line is complete and Phase 3 closes.

**Read order:** `CLAUDE.md` → `context/plans/README.md` → this plan's `plan.md` + this handoff → `context/decisions.md` (D1–D52, once D51/D52 land) → `spikes/foundation-models/README.md` + impl-plan `2026-06-24-phase3-recap-sidecar.md` → `./sidecar/build.sh` then `npm run tauri dev`. For hard calls, use **`/debate`**.
