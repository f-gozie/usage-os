# Review — Full-codebase audit (initial deep pass)

**Date:** 2026-06-25 · **Scope:** entire codebase (not a diff — an initial whole-tree audit) · **Files:** ~95 source files, ~8.4k production LOC
**Plan:** [plan.md](../plan.md) · **Maps to:** Phase 6 (Optimization / hardening / code-review sweep) — no single impl-plan (this is a cross-cutting audit)
**Codex:** ran (7 scoped lanes, `codex-cli 0.130.0`) · **Method:** 14-subsystem Claude panel (rules + simplify lanes) → adversarial verify → synthesis, plus 7 parallel Codex lanes → adversarial verify of every substantive Codex finding against real code + `decisions.md`.
**Raw data (this folder):** `*.codex-findings.json` (106) · `*.codex-verdicts.json` (37 adjudicated) · `*.warnings.json` (48 merged warning-level). _(The verbose 580 KB Claude-panel intermediate was kept local, not committed.)_

> This is the "deep audit / initial pass" the owner asked for — weighted heavily toward **simplification, over-engineering, comment-archaeology, AI-smells, and trial-and-error cruft**, on top of the usual hard-rules + correctness gate. The codebase is in genuinely good shape: **no critical issues, no serious hard-rule violations.** The yield is one real correctness cluster, a set of bounded warnings, and a large but mechanical simplification surface.

---

## Merge gates (all green)

| Gate | Result |
|---|---|
| `cargo fmt --check` | ✅ |
| `cargo clippy --all-targets --all-features -D warnings` | ✅ |
| `cargo test` | ✅ 106 passed, 1 ignored |
| `tsc --noEmit` | ✅ |
| `vitest run` | ✅ 20 passed |
| bindings fresh (`export_bindings` + `git diff`) | ✅ |

Hard rule #8 satisfied. Because the gates are green, every finding below is something the linter **cannot** catch.

## Hard-rule spot-checks (independent, ground-truth)

| Rule | Verdict | Evidence |
|---|---|---|
| ① Privacy / no network | ✅ clean | No `reqwest`/`ureq`/`hyper`/`std::net`/`tokio::net`/`URLSession`/`fetch` in any data path; frontend has zero network. Only matches are test-string URLs. |
| ② IPC generated | ✅ clean | 24 `#[tauri::command]`, all 24 `#[specta::specta]` and present in `collect_commands!`. `bindings.ts` fresh. |
| ③ No panics in prod | ✅ clean | Both crate roots carry `#![cfg_attr(not(test), deny(unwrap_used, expect_used, panic))]`; clippy `-D warnings` passes → no prod `unwrap`/`expect`/`panic` (the 219 hits are all test code). |
| ④ SQL in repo | ✅ clean | `capture`/`enrich` contain **zero** SQL literals — they call typed `db::` fns. All SQL lives in `db.rs`/`migrations.rs`. (The `Connection` *handle* living in the capture consumer + Tauri state is the documented D38/D31 design — see "Verified, not a bug".) |
| ⑤ Native/AI isolated | ✅ good | `capture`/`ai` are traits with fakes; `objc2`/Swift behind `#[cfg(target_os="macos")]`. |
| ⑥ Narrate-never-count | ✅ clean | `rollup` computes all numbers; `format_recap_prompt` hands the model pre-formatted, unit-spelled strings labeled "category"/"project" with "narrate only these, change nothing." Prose-only output. |
| ⑦ Design tokens | ⚠️ some | Real ad-hoc colors/px in a handful of components (see Warnings → Frontend). |

---

## Test coverage (measured)

Measured with `cargo-llvm-cov` (Rust) and `vitest --coverage` v8 (frontend). **127 tests** (107 Rust + 20 frontend), all green.

**Rust — 78% lines blended, but the blend is misleading:**

| Layer | Coverage | Note |
|---|---|---|
| `rollup.rs` | **99%** | the read-model / segmentation / recap — the brain |
| `migrations.rs` | **99%** | runner + checksum/drift |
| `db.rs` | **94%** | repository |
| `enrich/project.rs` | **93%** | project inference |
| `ai/mod.rs`, `enrich/mod.rs` | **100%** | |
| `capture/mod.rs` | **85%** | the single-writer state machine |
| `apps.rs` | **86%** | |
| `capture/macos/*`, `polling.rs` | **0%** | native FFI — verified **on-device** by design (D33), not in CI |
| `lib.rs` (4%), `main.rs` (0%) | low | Tauri command glue + `setup` — integration surface |

→ **The deterministic core is ~90%+; the 0% files are exactly the native/glue code verified out-of-band.** A flat 70% gate would punish on-device-verified code.

**Frontend — 54% lines blended:** hooks 90%, `dates` 91%, `AppIcon` 94%, `TimelineRow` 81%, `MiniDial` 100% are strong; gaps are interactive components — and notably **`CategoryEditorModal` is at 31%**, the exact file where Codex *thought* it found two bugs.

**Recommendation — a targeted standard, not a flat %:** keep coverage a **diagnostic**, and hold a high bar (~85–90%) on the **deterministic core** (`rollup`, `db`, `migrations`, `enrich`, `capture/mod`, `lib`'s pure helpers, and the frontend `lib/`). Leave the native FFI + Tauri glue out of the judgement (verified on-device / via Storybook). Do **not** bolt a single repo-wide % onto CI — it would either be gamed or sit unfairly red because of the 0% native layer. This matches the existing stance (D50: "no invented %-threshold"). First concrete action: add tests for `CategoryEditorModal` (the under-tested file where real bugs cluster) and `rollup::category_of` (see the slugless-category bug below).

---

## Codebase shape (for the record — owner asked)

~11.9k lines under `src*`, but only **~8.4k is production code**: Rust 3,938 + frontend 4,246 + Swift 103 + CSS 134. The rest is **tests 2,419** (Rust inline 2,078 + FE 341), **Storybook stories 592**, **generated `bindings.ts` 472**. For the feature surface (native event-driven capture via objc2 FFI, SQLite + a hand-rolled migration runner, the debated segmentation read-model, 4 views + Settings, ~30 components, an AI sidecar seam) that is **proportionate, not bloated**.

- **Inline Rust tests** (`#[cfg(test)] mod tests`) are idiomatic and compiled out of release builds — not a smell. The frontend correctly uses separate `*.test.tsx` (JS convention). Each language follows its own idiom.
- **Two files >1k lines:** `db.rs` (1,576; ~1,400 non-test) and `rollup.rs` (1,185; ~760 non-test). `rollup.rs` is one cohesive read-model (defensible). **`db.rs` is a legitimate split candidate** → `db/{events,projects,categories,exclusions,settings}.rs` re-exported from a `db/mod.rs` (organizational, low-risk; see Simplifications).

---

## Findings

**Verification:** Claude panel 196 raw → **187 confirmed** (9 dropped) · Codex 106 raw, 37 substantive **adjudicated** → 23 confirmed / 11 downgraded / 3 dropped · **51 cross-model matches** (same file ±6 lines from both models = high confidence).
**Severity after verification: 0 Critical · ~24 distinct Warnings · ~150 Info (simplification).**

### Critical — none

The one finding Codex rated critical that survived verification (`db.rs:753` empty-exclusion blackout) is **downgraded to Warning**: the consequence is severe (an empty `exclude` pattern matches every event via `contains("")` → total tracking blackout), but the UI guards it (`ExclusionModal` rejects empty patterns, button disabled on empty). It's a backend defense-in-depth gap, not a reachable critical.

### Warnings — real bugs worth fixing (the linter can't see these)

Grouped by area. `[X]` = cross-model confirmed (Codex **and** Claude independently).

**Capture (the honesty-promise core — fix these first):**
- `capture/mod.rs:282` — **Idle-inflation: the same-window event path extends/reopens a span with no idle check.** Idle is only judged on the ~20s tick, not on event arrival — the exact "chatty live-title tab while away balloons the span / spawns phantom spans" case the `consume` docstring claims to prevent. Route same-window re-fires through the same gate `on_tick` uses; no-op `set_span_end` when the end doesn't advance. *Touches the app's core promise — pair with a test.*
- `capture/macos/mod.rs:191` `[X]` — **Title-only dedupe drops URL / cwd / private-state changes.** Same-title SPA navigation (or `cd` to another repo in the same terminal) is suppressed, so the new URL/site/project is never recorded. Dedupe on the full identity, not just the title.
- `capture/mod.rs:147` — **`same_window()` ignores `project_id`** (compares app/title/url/is_private only). `cd` to a different git repo with an unchanged title extends the old span under the **wrong project** (D30). Add `project_id` to the comparison. *(Interacts with the dedupe above.)*
- `capture/macos/mod.rs:214` — **`emit_with_title` spawns `osascript`/`pgrep` synchronously on the main run loop.** A wedged scripted app (browser/terminal) freezes the **whole UI**. Dispatch the title/URL/cwd resolution to a worker, deliver via the existing channel.

**Data / DB:**
- `db.rs:429` `[X]` — **`reprocess_logs` matches with SQL `LIKE` while live `find_category` uses Rust substring** — the same rows categorize differently after a reprocess (`%`/`_` in a pattern act as wildcards in one path only). Unify on one matching primitive.
- `db.rs:264` + `rollup.rs:156` — **Spans crossing local midnight are counted whole on the start day and absent from the next** (no range clipping at the day boundary). Inflates one day, drops time from the next. Clip span durations to the queried `[start, end)`.
- `db.rs:374` / `db.rs:753` — **Empty/whitespace patterns match every event** (rule → miscategorize-all; exclusion → blackout/privatize-all). UI-guarded today, but the repository layer should reject empty patterns at `create_rule`/`create_exclusion` **and** skip them in `find_category`/`match_exclusion` (defense-in-depth).
- `db.rs:420` — **`reprocess_logs` is not atomic** — a mid-reprocess failure leaves history half-recategorized. Wrap in a transaction.
- `migrations/0006:12` `[X]` — **The "fresh-install-only" gate is `COUNT(activity_logs)=0`**, which also rewrites a user who configured categories but hasn't captured events yet (D47 intended "never rewrite an existing user's categories"). Gate on a real first-run marker, not emptiness of the log.
- `migrations.rs:97` — Drift check ignores a stored migration **absent** from `MIGRATIONS` (a binary downgrade / dropped migration goes undetected). Consider surfacing it.
- `migrations/0001:76` — Enum-like columns (`mode`, `match_type`, `alias_kind`, abstain reason) are documented but not `CHECK`-enforced; a bad string passes silently.

**Rollup / enrich:**
- `rollup.rs:164` — **★ Highest-impact correctness bug: user-created (slugless) categories collapse into "Uncategorized."** `category_of` maps any `slug=None` category to `OTHER_SLUG`, and `build_day_view` aggregates `totals` keyed by **slug** — so every custom category merges into one gray "Uncategorized" arc **and** with truly-uncategorized time, losing its name and custom color. Silently breaks the custom-category feature (D42/D44). Key aggregation on `category_id` (or carry the custom `color` into `CategorySlice` and key on a stable identity). *Add a test with a slugless category.*
- `enrich/project.rs:73` — **Browser-title fallback persists fake projects** (e.g. "YouTube", "GitHub") for general/non-repo pages — D30 says the title is the weakest signal and should abstain for non-project pages. Tighten the title→project path to abstain unless there's a real repo signal.
- `enrich/project.rs:159` `[X]` — `github.com` matched with `ends_with` lets look-alike hosts through (`evilgithub.com`). Use host equality / suffix-with-dot.
- `enrich/project.rs:176` `[X]` — `127.0.0.1:port` isn't treated as a local dev server while `localhost:port` is (asymmetry → inconsistent ambiguity handling).
- `rollup.rs:757` — `leading_project` can narrate a **tie** as the clear leader (no tie-break / margin). Require a margin or abstain on ties.

**Config / hygiene:**
- `capabilities/default.json:9` `[X]` — **`opener:default` grants `allow-open-url`** (open arbitrary `http(s)`), unused and over-broad for a local-only app. Scope to the reveal-in-dir / open-file permissions actually used. *(File is touched by PR #17 — coordinate.)*
- `tauri.conf.json:25` — **CSP is disabled (`csp: null`)** in a privacy-first app. Set a strict CSP (`default-src 'self'`, no remote). *(Also touched by PR #17.)*
- `.gitignore` — **The prebuilt sidecar binary `src-tauri/binaries/usageos-ai-*` is untracked AND not gitignored** — a `git add .` would commit a 111KB binary. *(Resolved by PR #17, which adds `binaries/.gitkeep` + the ignore — exclude from main cleanup.)*
- `apps.rs:189` `[X]` — **Icon cache key collisions** can serve one app's icon for a different app name (the cache key isn't unique enough). Verify the keying.

**Frontend — correctness:**
- `useDayData.ts:22` `[X]` / `useTimelineData.ts:25` / `useWeekData.ts:23` — **Stale-response race in all three (identical) data hooks:** no out-of-order guard, so fast day/week stepping renders the **wrong range's data**. Collapse into one generic `useViewData<T>(fetchFn, deps)` that owns an AbortController/request-id latch — fixes three correctness bugs and removes 3× duplication at once.
- `lib/dates.ts:7` — **Day vs Week boundary semantics disagree:** `dayBounds` uses an inclusive `23:59:59` end (drops the final second) while the Week path is half-open `[start, next)` (can double-count a midnight event). Pick half-open everywhere; let Rust own range arithmetic, frontend passes only local midnights.
- `SettingsView.tsx:33` — `CANON_ORDER` is missing the `personal` slug (added in D47), so **Personal sorts before Work**.
- `SettingsView.tsx:150` — "New category…" from the Uncategorized list **drops the selected app** (the `onNewCategory` arg is ignored) — the app you were sorting doesn't get added.
- `lib/categories.ts:12` — Hard-coded canonical display names can **disagree with user-edited DB names** (rename a canonical category → the UI label and the DB diverge).
- `CategoryEditorModal.tsx:142` — Toggling a just-**moved** app back **off** leaves the old owner's rule queued in `movedIds`, so saving still deletes it (the "move" isn't undone).
- `CategoryEditorModal.tsx:54` — The "match by window title" Advanced drawer saves **bare tokens as *process* rules** unless prefixed `title:` — a UX/labeling mismatch (the drawer implies title-matching). *(Lower confidence — verify the drawer copy intends bare = title.)*
- `ThemeProvider.tsx:57` — Initial settings load can **overwrite a user-selected theme** if it resolves after the user toggles (race).
- `DayView.tsx:76` — Degraded-banner **Retry refreshes day data but not the capture-health check**, so the banner can stick after recovery.
- `runs.ts:14` — Dial inspector hides mixed "no project" time (the inspector drops the no-project slice).

**Frontend — design tokens (hard rule 7) + a11y:**
- Ad-hoc colors bypassing tokens: `TitleBar.tsx:5` (`rgba()` muted), `CategoryEditorModal.tsx:24` (a 6-hex `PALETTE` literal), `ThemeSwitcher.tsx:7` (per-theme hex), `TimelineRow.tsx:47` (inline `var(--fg)`), `RadioGroup.tsx:35` (hardcoded `c-deep`), `Modal.tsx:36` (raw black overlay), `index.css:49` (skeleton shimmer vs flat-fill), plus raw `[18px]`/`[34px]` Tailwind values in `DetailInspector`/`Chip`/`AppShell`/the views. Sweep to tokens in one pass.
- a11y gaps in hand-rolled primitives: `Modal.tsx:21` (no focus trap / restore), `Select.tsx:61` (no keyboard navigation), `LedgerRow.tsx:27` (clickable row not keyboard-reachable), `Modal.tsx:47` (no max-height/scroll → tall modals overflow off-screen). Either adopt a headless primitive (Radix / React Aria) or add the minimum (focus trap + roving tabindex).

**Microcopy / labels:**
- `TimelineRow.tsx:84` `[X]` — expanded header labels the **segment count as "switches"**, contradicting the collapsed `countSwitches` summary.
- `DayView.tsx:54` — focus-stat comment/label stale after the Category rename.

### Info — simplification (the dominant theme; ~150 nits clustered below)

The largest cluster by far is **simplification**, exactly as requested. The 9 cross-cutting themes from synthesis:

1. **Decision-archaeology comments are the #1 noise across *every* subsystem (40+).** Source comments re-narrate decisions (`D8`/`D26`/`D29`/`D30`/`D34`/`D35`/`D41`/`D46`/`D47`/`R18`/`R57`/`C5`…) and spike provenance instead of describing what the code does. Module headers run 6–10 lines re-arguing settled debates (`apps.rs:1`, `browser.rs:1`, `terminal.rs:1`, `migrations.rs:1`, `project.rs:1`, `ruleMatch.ts:3`, `appIcons.ts:1`, `main.swift:1`); migration SQL headers re-litigate (`0004`/`0005`/`0006`); constants carry dogfooding history. **`decisions.md` is already the home for this** (CLAUDE.md mandates cross-reference, not duplicate). **Two have gone stale:** `0001`'s comment says the rename is "deferred D31" but D46 reversed it; `migrations/README.md` lists 2 of 6 migrations. → One sweeping pass: cut narration to a one-line *why* + bare cross-ref (`// patient gate — see D39`); keep genuine invariant/UAF comments; fix the two stale spots. **Low-risk, mechanical, biggest single win.**
2. **Three React data hooks are byte-identical** and share the stale-response race (see Warnings). → one `useViewData<T>`.
3. **Day/week time-boundary math is duplicated Rust↔TS and disagrees with itself** (see Warnings). → one half-open convention, Rust owns it.
4. **Rust/TS categorization + host-matching reimplemented and diverging** (`LIKE` vs substring; empty-pattern semantics; `ends_with` hosts; `slack`→`slackdownloader` icon prefix). → one matching primitive in Rust; question whether `ruleMatch.ts` is needed at read time at all (frontend already gets pre-computed aggregates per rule 6).
5. **Capture span-machine gaps + redundant writes + dead fields** (see Warnings; plus `FocusEvent.bundle_id`/`.pid` are dead carrier fields — set by the macOS source, never read).
6. **The dormant AI seam is over-built for zero callers** → **in-flight in PR #17** (see below) — *exclude from main cleanup.*
7. **Recurring micro-duplication in the Rust core:** `get_day`/`get_week`/`get_timeline` rebuild the same lookup-map block → `load_lookup_maps(conn)`; `get_setting`/`find_project_by_alias`/`get_project` repeat the query-one-row dance → a `query_one_opt` helper; `RawRunBuilder`/`SegRun` are the same struct with a copy → merge (or `From`); `build_runs` hand-copies 7 fields that are a subset of `TimelineRun` → struct-update; `best_icns` hand-rolls a fold → `Iterator::max_by_key`; `get_watcher_status` duplicates `ERROR_THRESHOLD` as a bare `6`; `parse_site`/`parse_url` each re-strip ports.
8. **Custom UI primitives ship without baseline a11y and bypass tokens** (see Warnings → design tokens). Plus single-use indirection (e.g. `DetailInspector`) and possibly-unused primitives to delete rather than keep "just in case."
9. **Defensive guards / ad-hoc logging diverging from the typed-error contract:** `open_span` swallows a `find_category` error with `eprintln!` while siblings propagate `Result`; startup retention cleanup uses `println!/eprintln!` `[Startup]` prefixes (5 levels deep) instead of `AppError`/a `tracing` logger; negative-duration `.max(0)` clamps in both `rollup.rs:156` and `format.ts:6` guard an impossible case with no documented why; an impossible-case `throw "Missing category id"`. → make `open_span` propagate; pick one logging approach; prove `end>=start` at the write boundary and drop the clamps (or keep one with a one-line invariant).

**Top simplifications to do first** (highest leverage):
1. One decision-archaeology comment sweep (theme 1) + fix the 2 stale comments.
2. Collapse the 3 data hooks into `useViewData<T>` with a stale-guard (fixes 3 bugs).
3. One canonical half-open day/week bound, Rust-owned.
4. `load_lookup_maps(conn)` for the 3 command handlers.
5. Move `emit_with_title`'s subprocess off the main run loop.
6. Route same-window re-fires through the idle gate.
7. Merge `RawRunBuilder`→`SegRun`; build `TimelineRun` via struct-update.
8. Extract shared `ErrorState` + `useCaptureHealth` + one `DayStepper`; reference `ERROR_THRESHOLD`.
9. One Rust host/pattern matching primitive (fixes the `LIKE`/`ends_with` divergences).
10. Sweep hardcoded colors → tokens.
11. Delete dead `FocusEvent.bundle_id`/`.pid` + single-use indirection.
12. **Split `db.rs`** (~1,400 lines) into `db/{events,projects,categories,exclusions,settings}.rs`.

**Best-practice gaps** (idiomatic Rust / modern React+TS): no fetch cancellation/stale-guard in hooks; custom Modal/Select without WAI-ARIA; hand-passed magic strings across the IPC boundary (`generated_by "fm"`, threshold `6`) instead of generated/exported types; inconsistent Rust error handling (print-and-swallow vs `Result`); hand-rolled folds where std combinators read better; owned `String`s where borrows suffice (hot capture/enrich paths); loose `ends_with`/`contains("")` as identity checks; sync subprocess on the main thread + `git` shelled per terminal focus with no caching; `&'static str` sentinels for a closed abstain-reason set (use an enum); sequential `await`s for independent rule writes (use `Promise.all`); placeholder manifest metadata (`Cargo authors = ["you"]`, `package.json name = "tauri-app"`) in a public MIT repo; unused `tokio` features (`time`, `sync`).

---

## In-flight in PR #17 — exclude from `main` cleanup

[PR #17](https://github.com/f-gozie/usage-os/pull/17) (open, `phase3/recap-sidecar-impl`) wires the AI sidecar chunks B–D. The audit ran against `main`, so several findings describe the *pre-wiring* state and are **owned by #17, not this cleanup**:
- The **"over-built dormant AI seam"** theme (no live callers) — #17 adds `get_recap` + `SidecarNarrator`.
- The **`"fm"` magic literal** in `RecapCard` — #17 explicitly fixes the dead check (badge now keys on `generated_by === "foundation-models"`).
- **`AiError` variants `build_recap` ignores** — gain real callers in #17.
- The **untracked sidecar binary** hygiene item — #17 adds `binaries/.gitkeep` + gitignore + `externalBin`.
- The **Swift findings** (blank-stdin guard, `emitLine` dropping on failure, `--serve` raw-line handling) were against the **old spike** at `spikes/foundation-models/`, *not* the production sidecar `sidecar/usageos-ai/` that #17 introduces — review them in the **#17-targeted session**, not here.
- `capabilities/default.json` + `tauri.conf.json` (opener/CSP) are **touched by #17** — fix those two Warnings in coordination with it.

## Verified — NOT a bug (checked, correctly not flagged)

The verification pass dropped these (Codex flagged some as critical; `decisions.md` shows they're deliberate):
- **`Arc<Mutex<Connection>>` handle owned by the capture consumer + Tauri state** (`capture/mod.rs:22`, `lib.rs:31`, `enrich/project.rs:57`) — the documented **D38 single-writer / D31 interim** design. SQL strings still live only in `db.rs`, so hard rule 4 holds.
- **Keeping the AX title for unprovable-private Chromium** (`browser.rs:64`) — documented **R18/D33** safe-default (URL dropped, title kept).
- specta `rc.20` pin + `bindings.ts @ts-nocheck` (D27); legacy SQL names `categories`/`category_id` (D42/D46); `mem::forget` of `!Send` keep-alives (D33); `current_idle_secs` `unwrap_or(0)` (D41); hand-rolled base64/CSV (D43/D42); forward-only migrations (D35); dial-arc-vs-ledger divergence (D40); commands-only IPC (D27); the dogfood-tunable named constants.

---

## Auto-fixes applied

**None.** `fmt`/`clippy` are already clean, and every remaining finding is a logic change, a judgement-heavy comment trim, a multi-file edit, or user-identity metadata — none is in the "provably-safe mechanical" set. All items are Manual TODO.

## Manual TODO (prioritized)

**P1 — correctness (fix soon):**
- [ ] `rollup.rs:164` — slugless/user categories collapse into Uncategorized (key aggregation on category id; carry custom color). **+ test.**
- [ ] `capture/mod.rs:282` + `:147` + `macos/mod.rs:191` — capture idle-inflation, `same_window` project, full-identity dedupe. **+ tests.**
- [ ] `db.rs:264` / `rollup.rs:156` — clip spans at day boundaries.
- [ ] `db.rs:429` + `db.rs:420` — unify reprocess matching with live; wrap reprocess in a transaction.
- [ ] `useViewData<T>` — fix the 3-hook stale-response race + dedup.
- [ ] `SettingsView.tsx:33` + `:150` — `CANON_ORDER` add `personal`; new-category keeps the selected app.

**P2 — hardening / privacy:**
- [ ] `tauri.conf.json` CSP + `capabilities/default.json` opener scope (coordinate w/ PR #17).
- [ ] Guard empty patterns at `create_rule`/`create_exclusion` + skip in matchers.
- [ ] `migrations/0006` fresh-install gate; `enrich/project.rs:73`/`:159`/`:176` project-inference tightening.
- [ ] Move `emit_with_title` subprocess off the main run loop.
- [ ] a11y: Modal focus-trap + max-height, Select keyboard nav, LedgerRow keyboard reach.

**P3 — simplification sweep (mechanical, high-volume):**
- [ ] Decision-archaeology comment pass (+ fix 2 stale comments) — theme 1.
- [ ] Hardcoded colors → tokens; delete dead `FocusEvent.bundle_id`/`.pid`.
- [ ] `load_lookup_maps`, `query_one_opt`, merge `RawRunBuilder`/`SegRun`, `ERROR_THRESHOLD` reuse, `max_by_key`.
- [ ] Split `db.rs` into submodules.
- [ ] Manifest metadata (`authors`, package name), unused `tokio` features.

**Coverage follow-up:**
- [ ] Add tests for `CategoryEditorModal` (31%) and `rollup::category_of` (the slugless case).

---

## Definition of Done / lifecycle

- [x] This audit changes **no `src/` code** (read-only review) → the `pre-push` tripwire would **not** fire.
- [ ] `plan.md` Phase 6 — annotate that the code-review sweep / security review ran (this report); leave the listed Phase-6 items (memory diagnosis, perf pass, WAL writer thread) open.
- [ ] No new ADR required *by the audit itself*; any decision to adopt the "targeted coverage standard" or to do the simplification sweep would warrant a short note when acted on.
- [x] Report written into the active plan's `reviews/` with raw findings JSON attached. Pairs with **Phase 6** (no single impl-plan — cross-cutting).

## Plan compliance

**Alignment: good.** This is precisely the Phase 6 "Code-review sweep across the redesign surface" + "Security review / reaffirm no-network" work the roadmap lists, pulled forward as an initial whole-tree pass. No scope creep; the findings feed Phase 6 hardening and a future per-PR `/usageos-review` (the intended steady-state use). The single most valuable outcome: **the codebase has no critical issues or hard-rule violations** — its debt is concentrated, mechanical (comment-archaeology), and a short list of bounded correctness bugs, all enumerated above.
