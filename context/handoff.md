# Handoff — start here for the next session

_Updated 2026-06-22 at the end of the **native-spikes + tauri-specta session**. Read this, then `CLAUDE.md`, then the redesign plan (`context/plans/2026-06-22-product-redesign.md`), then `context/feasibility/2026-06-22-feasibility-audit.md`. Supersedes the prior (Spike ②) handoff._

## 1. Where we are (TL;DR)

**Phase 0 is COMPLETE and Phase 1 has started.** The whole native capture gate plus project inference passed, and the first Phase-1 plumbing (generated IPC) is wired and green. Feasibility verdict moved from GO-WITH-CAVEATS → **GO** for the capture layer.

Capture is fully de-risked — all three project signals proven, on a proven event-driven model:
- **Spike #1** — AX window **titles**, Accessibility only / Screen Recording OFF (merged earlier, PR #6).
- **Spike ②** — **event-driven capture model**: NSWorkspace activation + per-PID AXObserver on the main run loop → Tokio over a `Send` channel (R6/R8–R11/R13).
- **Spike ③** — browser **URL** + incognito exclusion via Apple Events; `mode` read first (D8 enforced live) (R15/R17/R21).
- **Spike ④** — terminal **cwd** via `proc_pidinfo`, no `EPERM`, no TCC grant (R22).
- **Project-inference spike** — zero false assignments on real signals; abstain threshold set (R23/R26/R27).
- **tauri-specta IPC** — the Rust↔TS boundary is now **generated** (hard rule 2), all gates green.

## 2. PR / branch state — READ THIS FIRST

- **`main`** (`origin/main` → `34318af`) now contains **PR #7** (merged this session): the 4 spike crates (`spikes/ax-observer`, `browser-url`, `proc-cwd`, `project-inference`) + the Phase-0 doc updates (D29/D30, plan check-offs, standards).
- **[PR #9](https://github.com/f-gozie/usage-os/pull/9) is OPEN against `main`** — the tauri-specta IPC migration (branch `phase1/tauri-specta-ipc`), CI running at handoff time. **This handoff is committed on that branch.** _(PR #8 was the same work but GitHub auto-closed it when the stacked base branch was deleted on #7's merge; #9 supersedes it.)_
- **Next session: confirm #9's CI is green and merge it**, then continue Phase 1 from `main`.

## 3. Decisions locked this session (`context/decisions.md`)

- **D29** — Event-driven capture model: `AXObserver` from **`objc2-application-services`** (not `accessibility-sys`) on the **main run loop**, NSWorkspace activation via `block2::RcBlock`, marshaled to async over a `Send` channel; no `NSApplication`.
- **D30** — Project identity canonicalized on the **git remote** (`owner/repo`) with folder/title/URL aliases; **abstain threshold** (assign only on HIGH/MED unambiguous signals, else `unassigned`); `ambiguous` (localhost/dashboards) is a distinct third state for Phase-2 temporal correlation.
- **D27 RESOLVED** — the tauri-specta trio is **`rc.20`, NOT the standard's provisional `rc.24`** (see §5).

## 4. What's in the IPC migration (PR #9)

Full generated Rust↔TS boundary (D17/hard rule 2). All gates green locally: `cargo build`, **26 Rust tests**, `clippy -D warnings`, `fmt --check`, `tsc --noEmit`, **28 TS tests**, freshness gate md5-deterministic.
- All 11 commands `#[specta::specta]` + a typed `AppError` (`thiserror`+`Serialize`+`specta::Type`) replacing `Result<_, String>`.
- `get_watcher_status` (`serde_json::Value`→`WatcherStatus`) + `get_settings` (tuples→`Setting`) → named structs; `db.rs` boundary structs derive `specta::Type`.
- `tauri_specta::Builder` replaces `generate_handler!`; `export_bindings` `#[test]` emits **`src/bindings.ts`** (`BigIntExportBehavior::Number` → timestamps are `number`; header carries `// @ts-nocheck` because the generated file's events/channel boilerplate trips the app's `noUnusedLocals`).
- Frontend `src/lib/tauri.ts` delegates to the generated `commands`, unwraps the typed `Result`, and **re-exports the generated types as the single source**. Consumers fixed for new shapes (`Setting[]`, `category_id: number | null`).
- **Linux-only binding-freshness CI gate** (`.github/workflows/ci.yml`): regenerate + `git diff --exit-code src/bindings.ts`.
- Events deferred (commands-only) — tauri-specta #211 `never` bug stands; the dial/recap are pull-based so this costs nothing.

## 5. The rc.20 finding (important — don't relearn this the hard way)

The standard's provisional **rc.24 pin does NOT build**: specta rc.24 removed `#[specta(rename)]` on containers, but the current tauri (**2.9.3**, the newest 2.x) still uses it in `ipc/channel.rs`. Cargo's resolver *allows* the combo; compilation fails. Bumping tauri doesn't help. **Verified building pins:** `tauri-specta = "=2.0.0-rc.20"`, `specta = "=2.0.0-rc.20"`, `specta-typescript = "=0.0.7"`, `thiserror = "2"` (transitively `specta-macros 2.0.0-rc.17`). Revisit rc.24 only when tauri ships a release built against newer specta. Recorded in D27 + `context/standards/tauri-ipc.md` (which now also closes open questions #1–#7,#9).

## 6. NEXT — the full sequence to v1 (critical path + parallel tracks)

**Recommended immediate next: Phase 1.1 — the data model** (CI-friendly, no Mac, foundational; makes D30 real schema).

**Phase 1 — Capture → the dial** (in order):
1. **Data model / migrations v5+** — `projects` (keyed on git remote + aliases, D30), `sites`, richer `contexts`, event columns (`url`, `site`, `project_id`, `is_private`), `exclusions`; typed repository fns + tests. _Pure Rust + SQLite._
2. **The real `capture` trait** — fold spikes ①–④ behind a mockable `Fake`, `#[cfg(target_os="macos")]`-gated; replace the polling watcher. Includes terminal pid-*selection* (front-tab shell pid via `proc_listchildpids`+tty/recency, or iTerm2 `path`). _Mac._
3. **Sensitive handling** — exclusion list, per-app Private (time, no title), incognito never recorded (D8). _Mac._
4. **The fixed-24h dial** from real data, click-to-inspect. _Gated on the design system._

**Phase 2 — Enrichment**: embedding categorization (NLEmbedding) + corrections memory; contexts/rules editor; week view (7 mini-dials) + linear timeline.

**Phase 3 — The recap**: `RecapFacts` in Rust → deterministic `TemplateRecap` → Swift Foundation Models sidecar + availability/fallback (audit spike #6). _Needs the Apple Developer cert._

**Phase 4 — Shell & polish**: menubar launcher; primed onboarding/permission priming (run degraded if declined); dark-mode parity; idle-CPU perf pass.

**Phase 5 — Launch**: notarized DMG + auto-update + Homebrew cask; README rewrite for the new product; sponsor link; finalize name/domain.

**Parallel tracks (start early):**
- **Design system** — gates the dial (1.4). Full Bauhaus, both themes, all states; **fix the R77 blocker first** (Comms-yellow `#F2BC0C` fails WCAG non-text contrast 1.47:1; fix palette / add a non-color cue). Needs `/design-login` (not available in this CLI env) or in-repo design against `context/design-system.md`.
- **Apple Developer Program ($99/yr)** — hard external dependency with **enrollment lead time**; needed for the FM sidecar (Phase 3) AND distribution (Phase 5). Enroll now so it's never the blocker.

**Recommended near-term, off the critical path:**
- **Standards consolidation pass** — promote the spike-validated standards (`capture-and-permissions.md`, `tauri-ipc.md`) to authoritative (per-doc "Validated by Spikes …" status header), and **quarantine** the still-unproven parts (`foundation-models.md`; Safari private-detection R18; distribution R37/R65–R69) into a small explicit "Pending verification" block — so the docs read as standards, not perpetual drafts. Cheap, high-leverage for future sessions. (User asked for this; do it before/around Phase 1.1.)

## 7. Residual manual checks (small, optional, owner = Favour, all on the Mac)
- Spike ②: in-place `AXTitleChanged` (navigate a browser tab while it stays frontmost; confirm `TITLE` lines). Machinery proven.
- Spike ③: **Safari** URL + private-window safe-default (R16/R18 — Safari wasn't running), **Arc** (R17), and the **`-1743` deny** fallback (toggle a browser off in Automation settings).

## 8. Gotchas / environment

- **rustc was bumped 1.87 → 1.96** locally (via `rustup update stable`) — a transitive `darling 0.23` (from specta-macros) needs ≥ 1.88. CI uses `dtolnay/rust-toolchain@stable` (unpinned) so it already has it. If a fresh machine fails the specta build, check `rustc --version` ≥ 1.88.
- **zsh does NOT word-split unquoted `$vars`** (unlike bash) — `./bin $PIDS` passes the whole string as one arg. Use `xargs` or a zsh array. (Bit me twice this session; cost a couple of confusing reruns.)
- **`gh pr edit` is broken** here — it hits the deprecated Projects-classic GraphQL field and errors. Use the REST API: `gh api repos/f-gozie/usage-os/pulls/<n> -X PATCH -f base=main`. (But you can't change the base of a *closed* PR — see the #8→#9 saga in §2.)
- **Deleting a stacked PR's base branch auto-CLOSES the dependent PR** (doesn't retarget). For future stacks, retarget the child to `main` *before* deleting the parent's branch.
- A **Chrome incognito window** I opened during Spike ③ testing may still be open — close it.
- Disk was fine this session (~18 GB free); the spike `target/` dirs add a few hundred MB. `cargo clean` in `spikes/*/target` reclaims space if needed.
- **objc2 / AXObserver specifics** (validated, Spike ②): `AXObserver` is in `objc2-application-services` 0.3.2; notification names aren't re-exported (build CFStrings); keep the `block2::RcBlock` token alive; remove the run-loop source in `Drop` before the boxed `refcon` frees. Pins held: `objc2` 0.6.4, `objc2-*` 0.3.2, `block2` 0.6.2.

## 9. Re-run references

```sh
# Spikes (all isolated crates under spikes/, on main):
cargo build --manifest-path spikes/ax-observer/Cargo.toml      # Spike ② — switch apps; needs codesign --force --sign - for AX grant
cargo build --manifest-path spikes/browser-url/Cargo.toml      # Spike ③ — switch browser tabs; click Automation Allow prompts
pgrep -x zsh | sort -n | head -6 | xargs \
  ./spikes/proc-cwd/target/debug/proc-cwd                      # Spike ④ — no TCC grant; compare to lsof
./spikes/project-inference/target/debug/project-inference      # inference heuristic over the real corpus

# App (on phase1/tauri-specta-ipc, or main after #9 merges):
cd src-tauri && cargo build && cargo test && cargo clippy --all-targets --all-features -- -D warnings && cargo fmt --all -- --check
cargo test --manifest-path src-tauri/Cargo.toml export_bindings   # regenerate src/bindings.ts (then it's git-clean)
npx tsc --noEmit && npm test                                      # frontend (run npm install first if node_modules is incomplete)
```
Each spike has a `README.md` with results + protocol. `tccutil reset Accessibility` for a clean AX grant slate.

## 10. Read order for the next session
1. This file → 2. `CLAUDE.md` (rules) → 3. `context/plans/2026-06-22-product-redesign.md` (the roadmap; Phase 0 ✅, Phase 1 underway) → 4. `context/decisions.md` (D1–D30) → 5. `context/standards/{capture-and-permissions,tauri-ipc}.md` (now largely validated) → 6. the spike READMEs / `context/feasibility/2026-06-22-feasibility-audit.md` as needed.

## 11. Testing / CI status
25→**26 Rust** + **28 TS** tests pass; CI green (Linux/macOS/Windows) on #7; #9 CI running at handoff. Native spikes verified manually on-device (hosted CI can't grant TCC / run a GUI) — Phase-1 native code lives behind the `capture` trait with a `Fake` for CI, `#[cfg(target_os="macos")]`-gated (R80). The IPC migration + binding-freshness gate run fine on headless Linux.
