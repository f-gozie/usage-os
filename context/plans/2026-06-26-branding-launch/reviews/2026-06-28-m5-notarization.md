# Review — M5 notarization pipeline + capture/UI fixes

**Date:** 2026-06-28 · **Scope:** staged (`git diff --cached`) · **Files:** 17
**Plan:** [plan.md](../plan.md) · **Impl-plan:** none (M5 was direct work — noted in DoD)
**Codex:** ran (`codex` cross-model, read-only) — 1 finding, dropped (out-of-scope + documented)

## Merge gates
| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy --all-targets --all-features -D warnings | ✅ |
| cargo test | ✅ 127 passed |
| tsc --noEmit | ✅ |
| vitest | ✅ 32 passed |
| bindings fresh (regenerate == staged) | ✅ |

## Findings
**Verification:** 1 verified · 1 dropped · 0 cross-model

### Critical (must fix before merge)
- _None._

### Warnings (should fix) — fixed
- `src/components/timeline/TimelineRow.tsx:97` — **[correctness/CSS]** the title now lives in a CSS-grid
  `1fr` track (= `minmax(auto,1fr)`), so a long `truncate` (nowrap) title can't shrink below its
  min-content → it overflows the row instead of ellipsizing. The owner explicitly asked for "cut
  properly if too long." **Fixed** → `minmax(0,1fr)` so the track shrinks and `truncate` ellipsizes.

### Info
- The release scripts (`scripts/*.sh`, `gen-licenses-extra.mjs`) make build-time network/process calls
  (`tauri build` notarization → Apple; `cargo-about`; `npm ls`). These are **release tooling, not the
  app's data path** — hard rule 1 (nothing leaves the machine) governs `src-tauri/src/**` + `sidecar/**`,
  which are untouched. No violation.

### Dropped
- **Codex →** `db/events.rs:129` "bounded scan drops valid overlapping spans" (critical). Dropped on two
  counts: (1) **out of scope** — `db/events.rs` is not in the staged diff (Codex reviewed beyond the
  patch); (2) it is the **documented [D58] decision** and the concern is explicitly handled — the write
  path caps span length at `MAX_OPEN_SPAN_SECS` (12 h ≪ the 2-day lookback, `capture/mod.rs:110`) so the
  bounded scan is complete by construction, with regression tests asserting both halves. Not a bug.

## Hard-rules pass (Lane A)
- ① Privacy — no new network in `src-tauri/src/**` or `sidecar/**`. ✅
- ② IPC generated — `bindings.ts` regenerated via the export test (not hand-edited); freshness gate green. ✅
- ③ No `unwrap`/`expect`/`panic` — `enable_manual_accessibility` guards `kCFBooleanTrue` with `let-else`;
  the set-attr result is `let _ =` (intentional best-effort). `rollup` uses `.filter(...)`, no unwrap. ✅
- ④ SQL — none changed. ✅
- ⑤ Native isolated — the `AXManualAccessibility` set lives in `capture/macos/mod.rs` behind cfg; two
  `unsafe` blocks, both with safety comments. ✅
- ⑥ Smart layer — no `ai`/recap change. ✅
- ⑦ Design system — `TimelineRow` uses `truncate`/`text-muted` tokens; no ad-hoc colors/px/fonts. ✅

## Auto-fixes applied
- `TimelineRow.tsx` grid track `1fr` → `minmax(0,1fr)` (re-ran tsc + vitest: green).

## Manual TODO
- [ ] The notarized DMG itself — blocked on Apple's Notary Service outage; re-run `scripts/release-macos.sh`
      (or staple the in-flight submission) once Apple recovers. Pipeline + signing verified.
- [ ] Owner ~1 click: set the GitHub social preview to `landing/public/og.png`.
- [ ] `AXManualAccessibility` perf: confirm forcing Chrome's AX tree on feels fine over a longer session
      (verified working for title/URL capture; watch idle CPU).

## Definition of Done
- [x] plan.md ticked for what landed (M5 → `[~]`, sub-items checked; the two dogfood fixes noted)
- [x] decisions.md ADRs appended — D60 (distribution), D61 (telemetry/update), D62 (title display), D63 (Chromium AX)
- [~] no impl-plan (M5 was direct work, not a plan-mode task) · handoff to follow at session end
- [x] docs move with code — the pre-push tripwire would NOT fire (this diff touches `context/plans/` + `decisions.md`)

## Plan compliance
Alignment: **good.** The diff is M5 (signing/cert/notarization pipeline, license notices, bundle config,
seed_db→example) plus two capture/UI fixes discovered dogfooding the signed build (title display D62,
Chromium AX D63) — all launch-readiness, no scope creep. The notarized DMG is correctly deferred to
Apple's recovery; auto-update/Homebrew/Sponsor remain the Phase-5 tail.
